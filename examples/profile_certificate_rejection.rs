//! Research only: exact rational counterexamples to box nonvanishing.
//! No sampled sign is ever accepted as a certificate.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf, time::Instant};
use symbolica::domains::rational::Rational;
use symgcad::{
    algebra::{Algebra, Poly, rational},
    domain::{CertificateOptions, Domain, certify_nonzero, derive_domain, verify_certificate},
};

#[derive(Parser)]
struct Args {
    /// Directory containing original operation checkpoint JSON files.
    checkpoints: PathBuf,
    #[arg(long)]
    output: PathBuf,
    /// Work through at most this many distinct nonconstant factor outputs.
    #[arg(long, default_value_t = 128)]
    max_cases: usize,
    #[arg(long, default_value_t = 9)]
    max_points: usize,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    /// Separately time the identity replacements used by positive-box shifting.
    #[arg(long)]
    measure_identity_shifts: bool,
}

#[derive(Deserialize)]
struct Operation {
    variable_names: Vec<String>,
    operation: String,
    output: Vec<String>,
    output_hash: String,
    multiplicities: Option<Vec<usize>>,
}

#[derive(Clone, Debug, Serialize)]
struct Observation {
    point: Vec<String>,
    value: String,
    sign: i8,
}

fn height(q: &Rational) -> u64 {
    q.numerator_ref()
        .significant_bits()
        .max(q.denominator_ref().significant_bits())
}

/// A finite deterministic set strictly inside a nonempty rational open box.
/// Point construction is unrelated to the polynomial's sampled signs.
fn points(domain: &Domain, active: &[usize], cap: usize) -> Result<Vec<Vec<Rational>>> {
    if domain.empty || cap == 0 || active.len() > 16 {
        return Ok(vec![]);
    }
    let mut choices = Vec::new();
    for bound in &domain.bounds {
        let lower = bound.lower.as_deref().map(rational).transpose()?;
        let upper = bound.upper.as_deref().map(rational).transpose()?;
        if lower
            .as_ref()
            .into_iter()
            .chain(upper.as_ref())
            .any(|q| height(q) > 256)
        {
            return Ok(vec![]);
        }
        let candidates = match (&lower, &upper) {
            (Some(l), Some(u)) => {
                if l >= u {
                    return Ok(vec![]);
                }
                let width = u - l;
                vec![
                    l.clone() + width.clone() / Rational::from(2),
                    l.clone() + width.clone() / Rational::from(4),
                    l.clone() + width * Rational::from((3, 4)),
                ]
            }
            (Some(l), None) => vec![
                l.clone() + Rational::from(1),
                l.clone() + Rational::from((1, 4)),
                l.clone() + Rational::from(4),
            ],
            (None, Some(u)) => vec![
                u.clone() - Rational::from(1),
                u.clone() - Rational::from((1, 4)),
                u.clone() - Rational::from(4),
            ],
            (None, None) => vec![Rational::from(0), Rational::from(-1), Rational::from(1)],
        };
        ensure!(
            candidates
                .iter()
                .all(|q| lower.as_ref().is_none_or(|l| l < q)
                    && upper.as_ref().is_none_or(|u| q < u)),
            "point outside open box"
        );
        choices.push(candidates);
    }
    let center = choices.iter().map(|c| c[0].clone()).collect::<Vec<_>>();
    let mut result = vec![center.clone()];
    for offset in [1, 2] {
        let mut point = center.clone();
        for &v in active {
            point[v] = choices[v][offset].clone();
        }
        if !result.contains(&point) {
            result.push(point);
        }
    }
    for &v in active {
        for offset in [1, 2] {
            let mut point = center.clone();
            point[v] = choices[v][offset].clone();
            if !result.contains(&point) {
                result.push(point);
            }
        }
    }
    result.truncate(cap);
    Ok(result)
}

fn reject(p: &Poly, domain: &Domain, cap: usize) -> Result<(bool, Vec<Observation>)> {
    if p.nterms() > 4096
        || p.coefficients.iter().any(|c| height(c) > 512)
        || p.exponents_iter()
            .any(|m| m.iter().copied().fold(0u32, u32::saturating_add) > 64)
    {
        return Ok((false, vec![]));
    }
    let active = (0..p.nvars())
        .filter(|&v| p.degree(v) > 0)
        .collect::<Vec<_>>();
    let mut observations = Vec::new();
    let mut first = None;
    for point in points(domain, &active, cap)? {
        let value = p.replace_all(&point);
        let sign = if value.is_zero() {
            0
        } else if value.is_negative() {
            -1
        } else {
            1
        };
        observations.push(Observation {
            point: point.iter().map(ToString::to_string).collect(),
            value: value.to_string(),
            sign,
        });
        if sign == 0 || first.is_some_and(|s| s != sign) {
            return Ok((true, observations));
        }
        first = Some(sign);
    }
    Ok((false, observations))
}

fn identity_shifts(algebra: &Algebra, p: &Poly) -> Result<Poly> {
    let mut shifted = p.clone();
    for (variable, name) in algebra.names.iter().enumerate() {
        if shifted.degree(variable) != 0 {
            let replacement = algebra.parse(name)?;
            shifted = shifted.replace_with_poly(variable, &replacement);
        } else {
            shifted = shifted.clone();
        }
    }
    ensure!(shifted == *p, "identity substitution changed polynomial");
    Ok(shifted)
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.max_cases <= 1024
            && args.max_points <= 35
            && args.seconds > 0.0
            && args.seconds <= 300.0,
        "research limits exceeded"
    );
    fs::create_dir_all(&args.output)?;
    let started = Instant::now();
    let mut paths = fs::read_dir(&args.checkpoints)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.sort();
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    let mut probe_seconds = 0.0;
    let mut baseline_seconds = 0.0;
    let mut rejected_baseline_seconds = 0.0;
    let mut identity_seconds = 0.0;
    'files: for path in paths {
        let bytes = fs::read(&path)?;
        let Ok(operation) = serde_json::from_slice::<Operation>(&bytes) else {
            continue;
        };
        if operation.operation != "factor" {
            continue;
        }
        ensure!(
            operation.output_hash
                == format!(
                    "{:x}",
                    Sha256::digest(serde_json::to_vec(&(
                        &operation.output,
                        &operation.multiplicities
                    ))?)
                ),
            "checkpoint output checksum mismatch"
        );
        let algebra = Algebra::new(&operation.variable_names)?;
        // These are exact Gluza replay factors with only positive remaining axes.
        ensure!(
            operation.variable_names.iter().all(|n| n.starts_with('x')),
            "this corpus profile requires an all-positive Feynman axis map"
        );
        let constraints = algebra
            .names
            .iter()
            .map(|n| algebra.parse(n))
            .collect::<Result<Vec<_>>>()?;
        let domain = derive_domain(&algebra, &constraints, &[])?;
        for expression in operation.output {
            if rows.len() >= args.max_cases || started.elapsed().as_secs_f64() >= args.seconds {
                break 'files;
            }
            let identity = serde_json::to_string(&(&operation.variable_names, &expression))?;
            if !seen.insert(identity) {
                continue;
            }
            let p = algebra.parse(&expression)?;
            if p.is_constant() {
                continue;
            }
            // Keep the existing cheap uniform-coefficient success path untouched.
            if p.coefficients
                .iter()
                .all(|c| c.is_negative() == p.coefficients[0].is_negative())
            {
                continue;
            }
            let timer = Instant::now();
            let (rejected, observations) = reject(&p, &domain, args.max_points)?;
            let probe = timer.elapsed().as_secs_f64();
            let timer = Instant::now();
            let certificate =
                certify_nonzero(&algebra, &p, &domain, &CertificateOptions::default())?;
            let baseline = timer.elapsed().as_secs_f64();
            let identity_time = if args.measure_identity_shifts {
                let timer = Instant::now();
                let _ = identity_shifts(&algebra, &p)?;
                timer.elapsed().as_secs_f64()
            } else {
                0.0
            };
            identity_seconds += identity_time;
            if let Some(c) = &certificate {
                verify_certificate(&algebra, &p, &constraints, &[], c)?;
            }
            ensure!(
                !rejected || certificate.is_none(),
                "exact counterexample contradicts certificate"
            );
            probe_seconds += probe;
            baseline_seconds += baseline;
            if rejected {
                rejected_baseline_seconds += baseline;
            }
            rows.push(json!({"checkpoint":path,"checkpoint_sha256":format!("{:x}",Sha256::digest(&bytes)),"variables":algebra.names,"domain":domain,"polynomial":expression,"terms":p.nterms(),"rejected":rejected,"observations":observations,"probe_seconds":probe,"baseline_seconds":baseline,"identity_seconds":identity_time,"certificate_method":certificate.map(|c|c.method)}));
            let report = json!({"schema_version":1,"research_only":true,"policy":"exact zero or opposite signs on a connected open rational box only declines nonvanishing; no sampled sign is accepted","estimate_limitation":"estimated_precheck_seconds bypasses the whole certificate call including initial coordinate shifts; it is not a measured post-shift or end-to-end speedup","max_points":args.max_points,"rows":rows,"cases":rows.len(),"rejected":rows.iter().filter(|r|r["rejected"]==true).count(),"probe_seconds":probe_seconds,"baseline_seconds":baseline_seconds,"identity_seconds":identity_seconds,"rejected_baseline_seconds":rejected_baseline_seconds,"estimated_precheck_seconds":baseline_seconds-rejected_baseline_seconds+probe_seconds,"elapsed_seconds":started.elapsed().as_secs_f64()});
            fs::write(
                args.output.join("report.json"),
                serde_json::to_vec_pretty(&report)?,
            )?;
        }
    }
    println!(
        "{}",
        json!({"cases":rows.len(),"probe_seconds":probe_seconds,"baseline_seconds":baseline_seconds,"rejected_baseline_seconds":rejected_baseline_seconds,"elapsed_seconds":started.elapsed().as_secs_f64()})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup(constraints: &[&str]) -> (Algebra, Domain) {
        let algebra = Algebra::new(&["x".into(), "y".into()]).unwrap();
        let p = constraints
            .iter()
            .map(|p| algebra.parse(p).unwrap())
            .collect::<Vec<_>>();
        let domain = derive_domain(&algebra, &p, &[]).unwrap();
        (algebra, domain)
    }
    #[test]
    fn exact_zero_and_sign_change_only_reject() {
        let (a, d) = setup(&["x", "y"]);
        assert!(reject(&a.parse("x-y").unwrap(), &d, 9).unwrap().0);
        assert!(reject(&a.parse("x-2").unwrap(), &d, 9).unwrap().0);
        assert!(!reject(&a.parse("(x-y)^2+1").unwrap(), &d, 9).unwrap().0);
    }
    #[test]
    fn bounded_upper_and_unrestricted_points_stay_in_box() {
        let (_, d) = setup(&["x+3", "2-x", "5-y"]);
        for p in points(&d, &[0, 1], 9).unwrap() {
            assert!(p[0] > Rational::from(-3) && p[0] < Rational::from(2));
            assert!(p[1] < Rational::from(5));
        }
        let (a, d) = setup(&["x"]);
        assert!(reject(&a.parse("y").unwrap(), &d, 9).unwrap().0);
    }
    #[test]
    fn no_claim_from_same_sample_signs_and_caps() {
        let (a, d) = setup(&["x", "y"]);
        // It has a root at2, but one point with positive value proves nothing.
        assert!(!reject(&a.parse("(x-2)^2").unwrap(), &d, 1).unwrap().0);
        assert!(!reject(&a.parse("x^65-y^65").unwrap(), &d, 9).unwrap().0);
        assert!(points(&d, &[0], 0).unwrap().is_empty());
    }
    #[test]
    fn identity_measurement_reconstructs_exactly() {
        let (a, _) = setup(&["x", "y"]);
        let p = a.parse("(x-y)^4+x*y-3/7").unwrap();
        assert_eq!(identity_shifts(&a, &p).unwrap(), p);
    }
}
