//! Research only: bounded recursive quadratic nonvanishing, with separate replay.
//! No certificate here is accepted by the production solver or its verifier.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    algebra::{Algebra, Poly},
    domain::{
        CertificateOptions, Domain, DomainCertificate, certify_nonzero, derive_domain,
        verify_certificate,
    },
};

const MAX_NODES: usize = 128;
const MAX_WORK: usize = 1_000_000;
const MAX_TERMS: usize = 128;

#[derive(Parser)]
struct Args {
    checkpoints: PathBuf,
    /// Exact target-free original constraints, each interpreted as p > 0.
    #[arg(long)]
    constraint: Vec<String>,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    #[arg(long, default_value_t = 2048)]
    memory_mib: u64,
    #[arg(long, default_value_t = 2048)]
    candidates: usize,
    #[arg(long, hide = true)]
    worker: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Proof {
    Leaf {
        certificate: Box<DomainCertificate>,
    },
    Quadratic {
        variable: usize,
        leading: Box<Proof>,
        negative_discriminant: Box<Proof>,
    },
}

#[derive(Default)]
struct Budget {
    nodes: usize,
    work: usize,
    exhausted: bool,
}
impl Budget {
    fn spend(&mut self, p: &Poly) -> bool {
        self.nodes = self.nodes.saturating_add(1);
        self.work = self
            .work
            .saturating_add(p.nterms().saturating_mul(p.nterms()));
        let ok = self.nodes <= MAX_NODES && self.work <= MAX_WORK;
        self.exhausted |= !ok;
        ok
    }
}

fn bounded(p: &Poly) -> bool {
    p.nterms() <= MAX_TERMS
        && p.nvars() <= 16
        && (0..p.nvars()).all(|i| p.degree(i) <= 32)
        && p.coefficients.iter().all(|c| {
            c.numerator_ref()
                .significant_bits()
                .saturating_add(c.denominator_ref().significant_bits())
                <= 512
        })
}

fn leaf_options() -> CertificateOptions {
    CertificateOptions {
        max_terms: MAX_TERMS,
        quadratic_no_real_roots_candidates: 0,
        ..Default::default()
    }
}

fn coefficient(p: &Poly, v: usize, power: u32) -> Poly {
    let mut q = p.zero();
    for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
        if e[v] == power {
            let mut e = e.to_vec();
            e[v] = 0;
            q.append_monomial(c.clone(), &e);
        }
    }
    q
}

fn parts(p: &Poly, v: usize) -> Option<(Poly, Poly, Poly, Poly)> {
    if v >= p.nvars() || !bounded(p) || p.degree(v) != 2 {
        return None;
    }
    let a = coefficient(p, v, 2);
    let b = coefficient(p, v, 1);
    let c = coefficient(p, v, 0);
    if a.nterms()
        .saturating_mul(c.nterms())
        .saturating_add(b.nterms().saturating_mul(b.nterms()))
        > 4096
    {
        return None;
    }
    let minus_d = (&a * &c).mul_coeff(Rational::from(4)) - &b * &b;
    if !bounded(&minus_d) {
        return None;
    }
    Some((a, b, c, minus_d))
}

fn discover(
    a: &Algebra,
    p: &Poly,
    domain: &Domain,
    depth: usize,
    budget: &mut Budget,
) -> Result<Option<(i8, Proof)>> {
    if !bounded(p) || !budget.spend(p) {
        return Ok(None);
    }
    if let Some(certificate) = certify_nonzero(a, p, domain, &leaf_options())? {
        return Ok(Some((
            certificate.sign,
            Proof::Leaf {
                certificate: Box::new(certificate),
            },
        )));
    }
    if depth == 0 {
        return Ok(None);
    }
    // The cap matches the default production quadratic candidate count.
    for v in (0..p.nvars()).filter(|&v| p.degree(v) == 2).take(4) {
        let Some((lc, _, _, minus_d)) = parts(p, v) else {
            continue;
        };
        let Some((sign, leading)) = discover(a, &lc, domain, depth - 1, budget)? else {
            continue;
        };
        let Some((1, negative_discriminant)) = discover(a, &minus_d, domain, depth - 1, budget)?
        else {
            continue;
        };
        return Ok(Some((
            sign,
            Proof::Quadratic {
                variable: v,
                leading: Box::new(leading),
                negative_discriminant: Box::new(negative_discriminant),
            },
        )));
    }
    Ok(None)
}

// Replay uses supplied identities and ordinary leaf verification, never discovery.
// Every quadratic step eliminates its axis from both children, so no cyclic
// assumption, sampled sign, or target-dependent domain enters the argument.
fn replay(
    a: &Algebra,
    p: &Poly,
    constraints: &[Poly],
    proof: &Proof,
    depth: usize,
    budget: &mut Budget,
) -> Result<i8> {
    ensure!(
        bounded(p) && budget.spend(p),
        "proof resource envelope exceeded"
    );
    match proof {
        Proof::Leaf { certificate } => {
            ensure!(
                certificate.limits.quadratic_no_real_roots_candidates == 0,
                "nested production discovery is not a leaf"
            );
            ensure!(
                !matches!(
                    certificate.method,
                    symgcad::domain::CertificateMethod::QuadraticNoRealRoots { .. }
                ),
                "quadratic node hidden in a leaf"
            );
            verify_certificate(a, p, constraints, &[], certificate)?;
            Ok(certificate.sign)
        }
        Proof::Quadratic {
            variable,
            leading,
            negative_discriminant,
        } => {
            ensure!(depth > 0, "quadratic proof exceeds depth");
            let (lc, b, c, minus_d) =
                parts(p, *variable).ok_or_else(|| anyhow::anyhow!("invalid quadratic node"))?;
            let z = a.parse(&a.names[*variable])?;
            ensure!(
                &(&lc * &z) * &z + &b * &z + c == *p,
                "coefficient reconstruction failed"
            );
            ensure!(
                lc.degree(*variable) == 0 && minus_d.degree(*variable) == 0,
                "axis not eliminated"
            );
            let sign = replay(a, &lc, constraints, leading, depth - 1, budget)?;
            ensure!(
                replay(
                    a,
                    &minus_d,
                    constraints,
                    negative_discriminant,
                    depth - 1,
                    budget
                )? == 1,
                "discriminant is not strictly negative"
            );
            Ok(sign)
        }
    }
}

fn read_bounded(path: &Path, cap: usize) -> Result<Vec<u8>> {
    ensure!(path.is_file(), "not a regular input file");
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(cap as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= cap, "input exceeds byte limit");
    Ok(bytes)
}

fn checked_outputs(record: &Value) -> Result<Vec<String>> {
    let output: Vec<String> = serde_json::from_value(record["output"].clone())?;
    let multiplicities: Option<Vec<u64>> =
        serde_json::from_value(record["multiplicities"].clone())?;
    let expected = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(&output, &multiplicities))?)
    );
    ensure!(
        record["output_hash"].as_str() == Some(expected.as_str()),
        "checkpoint output checksum mismatch"
    );
    Ok(output)
}

fn worker(args: &Args) -> Result<Value> {
    let started = Instant::now();
    let mut paths = std::fs::read_dir(&args.checkpoints)?
        .map(|p| p.map(|p| p.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    ensure!(paths.len() <= 4096, "checkpoint count limit");
    paths.retain(|p| p.extension().is_some_and(|s| s == "json"));
    paths.sort();
    let mut names = None;
    let mut inventory: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut bytes_read = 0usize;
    for path in paths {
        let bytes = read_bounded(&path, 32 * 1024 * 1024)?;
        bytes_read += bytes.len();
        ensure!(bytes_read <= 128 * 1024 * 1024, "aggregate input cap");
        let record: Value = serde_json::from_slice(&bytes)?;
        let map: Vec<String> = serde_json::from_value(record["variable_names"].clone())?;
        if let Some(old) = &names {
            ensure!(old == &map, "mixed checkpoint maps");
        } else {
            names = Some(map);
        }
        let output = checked_outputs(&record)?;
        for expression in output {
            inventory.entry(expression).or_default().push(json!({"path":path,"sha256":format!("{:x}",Sha256::digest(&bytes)),"operation":record["operation"],"key":record["key"]}));
        }
    }
    let a = Algebra::new(&names.ok_or_else(|| anyhow::anyhow!("no checkpoint maps"))?)?;
    let constraints = args
        .constraint
        .iter()
        .map(|s| a.parse(s))
        .collect::<Result<Vec<_>>>()?;
    let domain = derive_domain(&a, &constraints, &[])?;
    ensure!(!domain.empty, "empty supplied domain");
    let inventory_count = inventory.len();
    let mut candidates = Vec::new();
    for (expression, sources) in inventory {
        let p = a.parse(&expression)?;
        if bounded(&p) && (0..p.nvars()).any(|v| p.degree(v) == 2) {
            candidates.push((p, expression, sources));
        }
    }
    candidates.sort_by(|x, y| x.0.nterms().cmp(&y.0.nterms()).then(x.1.cmp(&y.1)));
    let eligible = candidates.len();
    let mut current_count = 0;
    let mut attempted = 0;
    let mut rows = Vec::new();
    let mut completed = true;
    for (p, expression, sources) in candidates {
        if attempted >= args.candidates || started.elapsed().as_secs_f64() >= args.seconds * 0.9 {
            completed = false;
            break;
        }
        attempted += 1;
        if let Some(proof) = certify_nonzero(&a, &p, &domain, &CertificateOptions::default())? {
            verify_certificate(&a, &p, &constraints, &[], &proof)?;
            current_count += 1;
            continue;
        }
        let mut trials = Vec::new();
        let mut result = None;
        for depth in 0..=3 {
            let then = Instant::now();
            let mut budget = Budget::default();
            let found = discover(&a, &p, &domain, depth, &mut budget)?;
            trials.push(json!({"depth":depth,"seconds":then.elapsed().as_secs_f64(),"nodes":budget.nodes,"work":budget.work,"budget_exhausted":budget.exhausted,"certified":found.is_some()}));
            if let Some((sign, proof)) = found {
                ensure!(
                    replay(&a, &p, &constraints, &proof, depth, &mut Budget::default())? == sign,
                    "independent replay sign mismatch"
                );
                result = Some(json!({"depth":depth,"sign":sign,"proof":proof,"replayed":true}));
                break;
            }
        }
        rows.push(json!({"polynomial":expression,"sources":sources,"trials":trials,"additional_certificate":result}));
    }
    Ok(
        json!({"schema_version":1,"kind":"research_recursive_quadratic_nonvanishing","variables":a.names,"constraints":args.constraint,"domain":domain,"unique_output_inventory":inventory_count,"eligible":eligible,"attempted":attempted,"existing_proofs":current_count,"completed_scan":completed,"rows":rows,"wall_seconds":started.elapsed().as_secs_f64(),"limits":{"nodes_per_trial":MAX_NODES,"work_per_trial":MAX_WORK,"max_terms":MAX_TERMS,"max_depth":3},"coverage_claimed":false,"production_changes":false}),
    )
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 300.0,
        "invalid timeout"
    );
    ensure!(
        args.memory_mib > 0
            && args.memory_mib <= 4096
            && args.candidates > 0
            && args.candidates <= 4096,
        "invalid limits"
    );
    ensure!(
        !args.output.exists(),
        "refusing to overwrite research evidence"
    );
    if let Some(parent) = args.output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if args.worker {
        let report = worker(&args)?;
        let bytes = serde_json::to_vec_pretty(&report)?;
        ensure!(bytes.len() <= 32 * 1024 * 1024, "report too large");
        std::fs::write(&args.output, bytes)?;
        return Ok(());
    }
    let sidecar = args.output.with_extension("process.json");
    ensure!(!sidecar.exists(), "process evidence exists");
    let executable = std::env::current_exe()?;
    let mut command = Command::new(&executable);
    command
        .args(std::env::args_os().skip(1))
        .arg("--worker")
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    let limits = symgcad::Limits {
        wall_time_secs: args.seconds,
        memory_mib: args.memory_mib,
        workers: 1,
        max_cells: None,
    };
    let process = symgcad::benchmark::run_bounded(&mut command, &limits)?;
    let completed = process.status == "completed" && !process.output_truncated;
    let report = json!({"schema_version":1,"executable_sha256":format!("{:x}",Sha256::digest(read_bounded(&executable,512*1024*1024)?)),"limits":limits,"process":process});
    std::fs::write(sidecar, serde_json::to_vec_pretty(&report)?)?;
    ensure!(
        completed && args.output.is_file(),
        "bounded scan incomplete; preserved process evidence"
    );
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (Algebra, Vec<Poly>, Domain) {
        let a = Algebra::new(&["m".into(), "x".into(), "y".into(), "z".into()]).unwrap();
        let constraints = ["m", "4-m", "x", "y"]
            .iter()
            .map(|s| a.parse(s).unwrap())
            .collect::<Vec<_>>();
        let domain = derive_domain(&a, &constraints, &[]).unwrap();
        (a, constraints, domain)
    }
    #[test]
    fn nested_quadratic_identity_replays_without_discovery() {
        let (a, c, d) = setup();
        let p = a.parse("(z+x)^2+(x+y+1)^2-m*(x+y)").unwrap();
        assert!(
            discover(&a, &p, &d, 1, &mut Budget::default())
                .unwrap()
                .is_none()
        );
        let (sign, proof) = discover(&a, &p, &d, 3, &mut Budget::default())
            .unwrap()
            .unwrap();
        assert_eq!(sign, 1);
        assert_eq!(
            replay(&a, &p, &c, &proof, 3, &mut Budget::default()).unwrap(),
            1
        );
        assert!(matches!(proof, Proof::Quadratic { .. }));
        assert!(replay(&a, &p, &c, &proof, 0, &mut Budget::default()).is_err());
        assert!(replay(&a, &(&p - &p.one()), &c, &proof, 3, &mut Budget::default()).is_err());
        assert!(replay(&a, &p, &c[..1], &proof, 3, &mut Budget::default()).is_err());
    }
    #[test]
    fn zeros_degree_drops_and_sign_changes_are_not_strict_positivity() {
        let (a, _, d) = setup();
        for expression in ["(z-1)^2", "z^2-m", "(m-2)*z^2+1", "0"] {
            assert!(
                discover(
                    &a,
                    &a.parse(expression).unwrap(),
                    &d,
                    3,
                    &mut Budget::default()
                )
                .unwrap()
                .is_none(),
                "{expression}"
            );
        }
    }
    #[test]
    fn proof_and_work_limits_are_checked_before_recursive_algebra() {
        let (a, c, d) = setup();
        let p = a.parse("z^2+1").unwrap();
        let mut budget = Budget {
            nodes: MAX_NODES,
            ..Default::default()
        };
        assert!(discover(&a, &p, &d, 3, &mut budget).unwrap().is_none());
        assert!(budget.exhausted);
        let (_, proof) = discover(&a, &p, &d, 3, &mut Budget::default())
            .unwrap()
            .unwrap();
        assert!(
            replay(
                &a,
                &p,
                &c,
                &proof,
                3,
                &mut Budget {
                    work: MAX_WORK,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(parts(&p, 99).is_none());
    }

    #[test]
    fn operation_checksum_includes_the_multiplicity_inventory() {
        for multiplicities in [None, Some(vec![2u64])] {
            let output = vec!["x".to_string()];
            let digest = format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(&(&output, &multiplicities)).unwrap())
            );
            let mut record =
                json!({"output":output,"multiplicities":multiplicities,"output_hash":digest});
            assert_eq!(checked_outputs(&record).unwrap(), output);
            record["multiplicities"] = json!([3]);
            assert!(checked_outputs(&record).is_err());
        }
    }
}
