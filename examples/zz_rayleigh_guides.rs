//! Research-only first-Symanzik guide checks on unchanged symbolic ZZ domains.
//! Upstream Python arrays are parsed as literal data; no Python is executed.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    Problem,
    algebra::{Algebra, Poly},
    domain::{CertificateOptions, certify_nonzero, derive_domain, verify_certificate},
    problem::normalize,
    solver::evaluate,
};

#[derive(Parser)]
struct Args {
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value = "DO_NOT_PUSH_REFERENCES/gcad/tests/ZZ_integrals")]
    source_root: PathBuf,
    #[arg(long, default_value_t = 9)]
    cpu: usize,
    #[arg(long, default_value_t = 120.0)]
    seconds: f64,
    #[arg(long, hide = true)]
    worker: bool,
    /// Also inspect a positive orientation of the original F's t derivative as a guide.
    #[arg(long)]
    kinematic_guide: bool,
}

fn literal_array<'a>(line: &'a str, key: &str) -> Result<&'a str> {
    let start = line
        .find(&format!("{key}="))
        .context("missing array field")?
        + key.len()
        + 1;
    let data = &line[start..];
    ensure!(data.starts_with('['), "array literal must start with '['");
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (i, ch) in data.char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '[' => depth += 1,
            ']' => {
                depth = depth.checked_sub(1).context("bad bracket depth")?;
                if depth == 0 {
                    return Ok(&data[..i + 1]);
                }
            }
            _ => {}
        }
    }
    anyhow::bail!("unterminated literal array")
}

fn source_polynomial(source: &str, name: &str) -> Result<String> {
    let line = source
        .lines()
        .find(|line| line.starts_with(&format!("{name} = ExponentiatedPolynomial(")))
        .context("missing named upstream polynomial")?;
    let exponents: Vec<Vec<u32>> = serde_json::from_str(literal_array(line, "expolist")?)?;
    let coefficients =
        serde_json::from_str::<Vec<Value>>(&literal_array(line, "coeffs")?.replace('\'', "\""))?
            .into_iter()
            .map(|value| match value {
                Value::String(expression) => Ok(expression),
                Value::Number(number) if number.is_i64() || number.is_u64() => {
                    Ok(number.to_string())
                }
                _ => anyhow::bail!("upstream coefficient must be a string or exact integer"),
            })
            .collect::<Result<Vec<_>>>()?;
    let variables: Vec<String> =
        serde_json::from_str(&literal_array(line, "polysymbols")?.replace('\'', "\""))?;
    ensure!(
        exponents.len() == coefficients.len() && exponents.len() <= 10000,
        "upstream term count mismatch/cap"
    );
    exponents
        .iter()
        .zip(&coefficients)
        .map(|(powers, coefficient)| {
            ensure!(
                powers.len() == variables.len(),
                "upstream exponent map mismatch"
            );
            let mut term = format!("({coefficient})");
            for (name, &power) in variables.iter().zip(powers) {
                if power != 0 {
                    ensure!(
                        name.starts_with('x') && name[1..].chars().all(|c| c.is_ascii_digit()),
                        "unexpected active upstream symbol"
                    );
                    term.push_str(&format!("*{name}^{power}"));
                }
            }
            Ok(term)
        })
        .collect::<Result<Vec<_>>>()
        .map(|terms| terms.join("+"))
}

fn signs_witness(p: &Poly, constraints: &[Poly], names: &[String]) -> Result<Option<Value>> {
    let Some(t) = names.iter().position(|n| n == "t") else {
        return Ok(None);
    };
    if p.degree(t) != 1 {
        return Ok(None);
    }
    let dt = p.derivative(t);
    for trial in 0..64 {
        let mut point = names
            .iter()
            .enumerate()
            .map(|(i, name)| match name.as_str() {
                "s" => Rational::from(5),
                "t" => Rational::zero(),
                _ => Rational::from(((i + 1) * (trial + 1) % 11 + 1) as i64),
            })
            .collect::<Vec<_>>();
        let slope = evaluate(&dt, &point);
        if slope.is_zero() {
            continue;
        }
        let intercept = evaluate(p, &point);
        let magnitude = intercept.clone().abs() + Rational::one();
        let mut witnesses = Vec::new();
        for sign in [-1i64, 1] {
            point[t] = Rational::from(sign) * magnitude.clone() / slope.clone();
            ensure!(
                constraints
                    .iter()
                    .all(|q| evaluate(q, &point) > Rational::zero()),
                "counterexample left original domain"
            );
            let value = evaluate(p, &point);
            ensure!(
                if sign < 0 {
                    value < Rational::zero()
                } else {
                    value > Rational::zero()
                },
                "affine sign witness failed"
            );
            witnesses.push(json!({"point":names.iter().zip(&point).map(|(name,value)|(name.clone(),json!(value.to_string()))).collect::<serde_json::Map<_,_>>(),"value":value.to_string()}));
        }
        return Ok(Some(
            json!({"method":"exact_affine_unrestricted_t","negative":witnesses[0],"positive":witnesses[1]}),
        ));
    }
    Ok(None)
}

/// A>0,B<0 is incompatible with B*C-A*D >= 0 for every C,D>0.
/// The search only reports fully checked exact interior points; absence is inconclusive.
fn incompatible_endpoint_witness(
    a: &Poly,
    b: &Poly,
    orientation: i64,
    constraints: &[Poly],
    names: &[String],
) -> Result<Option<Value>> {
    let Some(t) = names.iter().position(|name| name == "t") else {
        return Ok(None);
    };
    if a.degree(t) > 1 || b.degree(t) > 1 {
        return Ok(None);
    }
    let da = a.derivative(t);
    let db = b.derivative(t);
    let physical_axes = names
        .iter()
        .enumerate()
        .filter_map(|(i, name)| name.starts_with('x').then_some(i))
        .collect::<Vec<_>>();
    for trial in 0..(256 + physical_axes.len() * 9) {
        let mut point = names
            .iter()
            .enumerate()
            .map(|(i, name)| match name.as_str() {
                "s" => Rational::from(5 + (trial / 64) as i64 * 5),
                "t" => Rational::zero(),
                _ => Rational::from(((i + 1) * (trial + 1) % 11 + 1) as i64),
            })
            .collect::<Vec<_>>();
        if trial >= 256 {
            for &axis in &physical_axes {
                point[axis] = Rational::one();
            }
            if let Some(s) = names.iter().position(|name| name == "s") {
                point[s] = Rational::from(5);
            }
            let axis = physical_axes[(trial - 256) / 9];
            point[axis] = Rational::from(1i64 << ((trial - 256) % 9));
        }
        let sign = Rational::from(orientation);
        let inequalities = [
            (
                evaluate(&da, &point) * sign.clone(),
                evaluate(a, &point) * sign.clone(),
            ),
            (
                -evaluate(&db, &point) * sign.clone(),
                -evaluate(b, &point) * sign,
            ),
        ];
        let mut lower: Option<Rational> = None;
        let mut upper: Option<Rational> = None;
        let mut feasible = true;
        for (slope, intercept) in inequalities {
            if slope.is_zero() {
                if intercept <= Rational::zero() {
                    feasible = false;
                    break;
                }
            } else {
                let bound = -intercept / slope.clone();
                if slope > Rational::zero() {
                    if lower.as_ref().is_none_or(|old| bound > *old) {
                        lower = Some(bound);
                    }
                } else if upper.as_ref().is_none_or(|old| bound < *old) {
                    upper = Some(bound);
                }
            }
        }
        if !feasible
            || lower
                .as_ref()
                .zip(upper.as_ref())
                .is_some_and(|(l, u)| l >= u)
        {
            continue;
        }
        point[t] = match (&lower, &upper) {
            (Some(l), Some(u)) => (l + u) / Rational::from(2),
            (Some(l), None) => l.clone() + Rational::one(),
            (None, Some(u)) => u.clone() - Rational::one(),
            (None, None) => Rational::zero(),
        };
        ensure!(
            constraints
                .iter()
                .all(|q| evaluate(q, &point) > Rational::zero()),
            "endpoint counterexample left original domain"
        );
        let av = evaluate(a, &point) * Rational::from(orientation);
        let bv = evaluate(b, &point) * Rational::from(orientation);
        ensure!(
            av > Rational::zero() && bv < Rational::zero(),
            "endpoint counterexample failed"
        );
        return Ok(Some(
            json!({"orientation":orientation,"point":names.iter().zip(&point).map(|(name,value)|(name.clone(),json!(value.to_string()))).collect::<serde_json::Map<_,_>>(),"oriented_leading":av.to_string(),"oriented_endpoint":bv.to_string(),"strict_t_interval":{"lower":lower.map(|v|v.to_string()),"upper":upper.map(|v|v.to_string())}}),
        ));
    }
    Ok(None)
}

fn factor_report(algebra: &Algebra, p: &Poly) -> Result<Value> {
    let factors = p.factor();
    let reconstructed = factors.iter().fold(p.one(), |v, (f, n)| &v * &f.pow(*n));
    ensure!(reconstructed == *p, "factor reconstruction failed");
    Ok(json!(
        factors
            .iter()
            .map(|(f, n)| json!({"polynomial":algebra.format(f),"multiplicity":n}))
            .collect::<Vec<_>>()
    ))
}

fn inspect(path: &Path, source_root: &Path, kinematic_guide: bool) -> Result<Value> {
    let bytes = fs::read(path)?;
    let problem = Problem::from_toml(std::str::from_utf8(&bytes)?)?;
    ensure!(
        problem.split.len() == 1,
        "research input requires one original split"
    );
    let name = problem
        .name
        .as_deref()
        .context("missing source name")?
        .split('/')
        .next()
        .unwrap();
    ensure!(
        name.starts_with("finite") && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "unexpected source name"
    );
    let source_path = source_root.join(format!("{name}.py"));
    let source_bytes = fs::read(&source_path)?;
    let source = std::str::from_utf8(&source_bytes)?;
    let mut upstream = problem.clone();
    upstream.split = vec![source_polynomial(source, "F")?];
    let original = normalize(&problem, &problem.variables)?;
    let reconstructed = normalize(&upstream, &problem.variables)?;
    ensure!(
        original.split == reconstructed.split,
        "upstream F differs from exact imported original"
    );
    let algebra = &original.algebra;
    let f = &original.split[0];
    let u = algebra.parse(&source_polynomial(source, "U")?)?;
    let domain = derive_domain(algebra, &original.positives, &[])?;
    let options = CertificateOptions::default();
    let mut rows = Vec::new();
    for (axis, name) in problem
        .variables
        .iter()
        .enumerate()
        .filter(|(_, n)| n.starts_with('x'))
    {
        if f.degree(axis) != 1 {
            continue;
        }
        let started = Instant::now();
        ensure!(u.degree(axis) == 1, "U lost expected multiaffine support");
        let a = f.derivative(axis);
        let b = f.replace(axis, &Rational::zero());
        let c = u.derivative(axis);
        let d = u.replace(axis, &Rational::zero());
        let c_certificate = certify_nonzero(algebra, &c, &domain, &options)?;
        let d_certificate = certify_nonzero(algebra, &d, &domain, &options)?;
        for (poly, cert) in [(&c, &c_certificate), (&d, &d_certificate)] {
            if let Some(cert) = cert {
                verify_certificate(algebra, poly, &original.positives, &[], cert)?;
            }
        }
        let bc = &b * &c;
        let ad = &a * &d;
        let residual = &bc - &ad;
        ensure!(
            residual.degree(axis) == 0,
            "comparison retained eliminated axis"
        );
        let sign_change = signs_witness(&residual, &original.positives, &problem.variables)?;
        let incompatible_positive =
            incompatible_endpoint_witness(&a, &b, 1, &original.positives, &problem.variables)?;
        let incompatible_negative =
            incompatible_endpoint_witness(&a, &b, -1, &original.positives, &problem.variables)?;
        let certificate = if sign_change.is_none() {
            certify_nonzero(algebra, &residual, &domain, &options)?
        } else {
            None
        };
        if let Some(cert) = &certificate {
            verify_certificate(algebra, &residual, &original.positives, &[], cert)?;
        }
        let t_derivative = problem
            .variables
            .iter()
            .position(|n| n == "t")
            .map(|t| residual.derivative(t));
        let t_factors = if let Some(dt) = t_derivative
            .as_ref()
            .filter(|p| !p.is_zero() && p.nterms() <= 128)
        {
            Some(factor_report(algebra, dt)?)
        } else {
            None
        };
        let alternative = if kinematic_guide {
            if let Some(t) = problem.variables.iter().position(|n| n == "t") {
                let mut guide = f.derivative(t);
                let mut orientation = 1;
                let raw_c = guide.derivative(axis);
                let raw_d = guide.replace(axis, &Rational::zero());
                let raw_cc = certify_nonzero(algebra, &raw_c, &domain, &options)?;
                let raw_dc = certify_nonzero(algebra, &raw_d, &domain, &options)?;
                if raw_cc.as_ref().is_some_and(|c| c.sign == -1)
                    && raw_dc.as_ref().is_some_and(|c| c.sign == -1)
                {
                    guide = -guide;
                    orientation = -1;
                }
                let c = guide.derivative(axis);
                let d = guide.replace(axis, &Rational::zero());
                let cc = certify_nonzero(algebra, &c, &domain, &options)?;
                let dc = certify_nonzero(algebra, &d, &domain, &options)?;
                if cc.as_ref().is_some_and(|c| c.sign == 1)
                    && dc.as_ref().is_some_and(|c| c.sign == 1)
                {
                    verify_certificate(
                        algebra,
                        &c,
                        &original.positives,
                        &[],
                        cc.as_ref().unwrap(),
                    )?;
                    verify_certificate(
                        algebra,
                        &d,
                        &original.positives,
                        &[],
                        dc.as_ref().unwrap(),
                    )?;
                    let residual = &(&b * &c) - &(&a * &d);
                    ensure!(
                        residual.degree(t) == 0,
                        "kinematic comparison did not cancel t"
                    );
                    let cert = certify_nonzero(algebra, &residual, &domain, &options)?;
                    if let Some(cert) = &cert {
                        verify_certificate(algebra, &residual, &original.positives, &[], cert)?;
                    }
                    let factors = if !residual.is_zero() && residual.nterms() <= 128 {
                        Some(factor_report(algebra, &residual)?)
                    } else {
                        None
                    };
                    let s_structure = if let Some(s) =
                        problem.variables.iter().position(|n| n == "s")
                    {
                        let slope = residual.derivative(s);
                        let at_four = residual.replace(s, &Rational::from(4));
                        Some(
                            json!({"degree":residual.degree(s),"slope":algebra.format(&slope),"slope_factors":if !slope.is_zero()&&slope.nterms()<=128 {Some(factor_report(algebra,&slope)?)} else {None},"at_four":algebra.format(&at_four),"at_four_terms":at_four.nterms(),"at_four_factors":if !at_four.is_zero()&&at_four.nterms()<=128 {Some(factor_report(algebra,&at_four)?)} else {None}}),
                        )
                    } else {
                        None
                    };
                    Some(
                        json!({"orientation":orientation,"guide":algebra.format(&guide),"leading_certificate":cc,"endpoint_certificate":dc,"comparison":algebra.format(&residual),"comparison_terms":residual.nterms(),"comparison_factors":factors,"comparison_certificate":cert,"s_structure":s_structure}),
                    )
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        rows.push(json!({"axis":name,"A":algebra.format(&a),"B":algebra.format(&b),"C":algebra.format(&c),"D":algebra.format(&d),"C_certificate":c_certificate,"D_certificate":d_certificate,"residual":algebra.format(&residual),"residual_terms":residual.nterms(),"t_derivative":t_derivative.as_ref().map(|p|algebra.format(p)),"t_derivative_factors":t_factors,"both_signs":sign_change,"positive_orientation_no_positive_guide_witness":incompatible_positive,"negative_orientation_no_positive_guide_witness":incompatible_negative,"strict_residual_certificate":certificate,"residual_identically_zero":residual.is_zero(),"kinematic_slope_guide":alternative,"seconds":started.elapsed().as_secs_f64()}));
    }
    Ok(
        json!({"input":path,"input_sha256":format!("{:x}",Sha256::digest(&bytes)),"source":source_path,"source_sha256":format!("{:x}",Sha256::digest(&source_bytes)),"source_F_identity_verified":true,"variables":problem.variables,"constraints":problem.constraints,"parameters":problem.parameters,"U":algebra.format(&u),"U_terms":u.nterms(),"F_terms":f.nterms(),"linear_axes":rows}),
    )
}

fn write(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("partial.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds > 0.0 && args.seconds <= 300.0 && args.seconds.is_finite(),
        "invalid research budget"
    );
    ensure!(!args.output.exists(), "output must be fresh");
    if !args.worker {
        let mut command = Command::new("taskset");
        command
            .arg("-c")
            .arg(args.cpu.to_string())
            .arg(std::env::current_exe()?)
            .args(&args.inputs)
            .arg("--worker")
            .arg("--output")
            .arg(&args.output)
            .arg("--source-root")
            .arg(&args.source_root)
            .arg("--seconds")
            .arg(args.seconds.to_string());
        if args.kinematic_guide {
            command.arg("--kinematic-guide");
        }
        let process = symgcad::benchmark::run_bounded(
            &mut command,
            &symgcad::Limits {
                wall_time_secs: args.seconds,
                memory_mib: 2048,
                workers: 1,
                max_cells: None,
            },
        )?;
        write(
            &args.output.with_extension("process.json"),
            &serde_json::to_value(&process)?,
        )?;
        ensure!(
            process.status == "completed" && process.exit_code == Some(0),
            "research worker did not complete: {}",
            process.status
        );
        return Ok(());
    }
    let started = Instant::now();
    let mut report = json!({"kind":"exact_upstream_U_rayleigh_diagnostic","coverage_claim":false,"inputs":args.inputs,"rows":[],"completed":false,"seconds":0.0});
    write(&args.output, &report)?;
    for path in &args.inputs {
        report["rows"].as_array_mut().unwrap().push(inspect(
            path,
            &args.source_root,
            args.kinematic_guide,
        )?);
        report["seconds"] = json!(started.elapsed().as_secs_f64());
        write(&args.output, &report)?;
    }
    report["completed"] = json!(true);
    write(&args.output, &report)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use symgcad::algebra::rational;
    #[test]
    fn literal_parser_keeps_maps_and_does_not_execute_python() {
        let source = "U = ExponentiatedPolynomial(expolist=[[1,0,0],[0,1,0]], coeffs=['1','2/3'], polysymbols=['x1','x2','eps'], exponent='-3')";
        let a = Algebra::new(&["x1".into(), "x2".into()]).unwrap();
        assert_eq!(
            a.parse(&source_polynomial(source, "U").unwrap()).unwrap(),
            a.parse("x1+2*x2/3").unwrap()
        );
        assert!(source_polynomial(&source.replace("[1,0,0]", "[1,0,1]"), "U").is_err());
        assert!(literal_array("expolist=[[1]", "expolist").is_err());
        assert_eq!(
            source_polynomial(&source.replace("['1','2/3']", "[1,'2/3']"), "U").unwrap(),
            source_polynomial(source, "U").unwrap()
        );
        assert!(source_polynomial(&source.replace("['1','2/3']", "[1.1,'2/3']"), "U").is_err());
    }
    #[test]
    fn unrestricted_affine_witness_proves_both_signs_exactly() {
        let a = Algebra::new(&["s".into(), "t".into(), "x".into()]).unwrap();
        let constraints = [a.parse("s-4").unwrap(), a.parse("x").unwrap()];
        let p = a.parse("t*(x+1)+s*x^2").unwrap();
        let witness = signs_witness(&p, &constraints, &a.names).unwrap().unwrap();
        assert!(
            rational(witness["negative"]["value"].as_str().unwrap()).unwrap() < Rational::zero()
        );
        assert!(
            rational(witness["positive"]["value"].as_str().unwrap()).unwrap() > Rational::zero()
        );
        assert!(
            signs_witness(&a.parse("x^2+s").unwrap(), &constraints, &a.names)
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn incompatible_endpoint_strict_interval_and_orientation() {
        let algebra = Algebra::new(&["s".into(), "t".into(), "x".into()]).unwrap();
        let constraints = [algebra.parse("s-4").unwrap(), algebra.parse("x").unwrap()];
        let a = algebra.parse("t-1").unwrap();
        let b = algebra.parse("t-3").unwrap();
        let witness = incompatible_endpoint_witness(&a, &b, 1, &constraints, &algebra.names)
            .unwrap()
            .unwrap();
        assert_eq!(witness["point"]["t"], "2");
        assert_eq!(witness["oriented_leading"], "1");
        assert_eq!(witness["oriented_endpoint"], "-1");
        assert!(
            incompatible_endpoint_witness(&a, &b, -1, &constraints, &algebra.names)
                .unwrap()
                .is_none()
        );
        assert!(
            incompatible_endpoint_witness(&a, &a, 1, &constraints, &algebra.names)
                .unwrap()
                .is_none()
        );
    }
}
