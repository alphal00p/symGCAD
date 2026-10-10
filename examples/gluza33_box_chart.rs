//! Research-only triangular orthant/box chart. No original CAD is exported.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf, process::Command, time::Instant};
use symbolica::prelude::*;
use symgcad::{
    Limits, Problem,
    algebra::{Algebra, Poly},
    domain::homogeneous_degree,
    problem::normalize,
};

const ORIGINAL: [&str; 6] = ["x0", "x6", "x4", "x7", "x1", "x2"];
const CHART: [&str; 5] = ["u", "z", "r", "v", "w"];

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, hide = true)]
    worker: bool,
}

fn positive_coordinate(p: &Poly, i: usize) -> bool {
    p.nterms() == 1
        && p.coefficients[0] > Rational::zero()
        && p.exponents_iter()
            .next()
            .is_some_and(|e| e.iter().enumerate().all(|(j, &d)| d == u32::from(i == j)))
}

/// q=(1-u)^m*r^n*p(1,u/(1-u),z,z*(1-r)/r,v,w).
/// Original map is ORIGINAL; q uses CHART. Homogeneity supplies scale^degree.
fn forward(p: &Poly, a: &Algebra) -> Result<(Poly, u32, u32, u32)> {
    let degree =
        homogeneous_degree(p, &(0..6).collect::<Vec<_>>()).context("nonhomogeneous input")?;
    ensure!(degree <= 12 && p.nterms() <= 4096, "research expansion cap");
    let (m, n) = (p.degree(1), p.degree(3));
    let axes = CHART
        .iter()
        .map(|v| a.parse(v))
        .collect::<Result<Vec<_>>>()?;
    let one = a.parse("1")?;
    let omu = &one - &axes[0];
    let omr = &one - &axes[2];
    let mut q = one.zero();
    for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
        let mut term = one.clone().mul_coeff(c.clone());
        for factor in [
            axes[0].pow(e[1] as usize),
            omu.pow((m - e[1]) as usize),
            axes[1].pow((e[2] + e[3]) as usize),
            axes[2].pow((n - e[3]) as usize),
            omr.pow(e[3] as usize),
            axes[3].pow(e[4] as usize),
            axes[4].pow(e[5] as usize),
        ] {
            term = &term * &factor;
        }
        q = &q + &term;
        ensure!(q.nterms() <= 65536, "forward term cap");
    }
    Ok((q, m, n, degree))
}

/// Check the complete polynomial inverse, including degree drops. Every
/// denominator is a product of x0, A=x0+x6, D=x4+x7, all strictly positive.
fn check_inverse(p: &Poly, q: &Poly, a: &Algebra, m: u32, n: u32, degree: u32) -> Result<()> {
    let axes = ORIGINAL
        .iter()
        .map(|v| a.parse(v))
        .collect::<Result<Vec<_>>>()?;
    let aa = &axes[0] + &axes[1];
    let dd = &axes[2] + &axes[3];
    let mu = q.degree(0);
    let mr = q.degree(2);
    let t = q.degree(1) + q.degree(3) + q.degree(4);
    let mut inverse = p.zero();
    for (c, e) in q.coefficients.iter().zip(q.exponents_iter()) {
        let mut term = p.one().mul_coeff(c.clone());
        for factor in [
            axes[1].pow(e[0] as usize),
            aa.pow((mu - e[0]) as usize),
            axes[2].pow((e[1] + e[2]) as usize),
            dd.pow((mr - e[2]) as usize),
            axes[4].pow(e[3] as usize),
            axes[5].pow(e[4] as usize),
            axes[0].pow((t - e[1] - e[3] - e[4]) as usize),
        ] {
            term = &term * &factor;
        }
        inverse = &inverse + &term;
        ensure!(inverse.nterms() <= 262144, "inverse term cap");
    }
    let mut expected = p * &axes[2].pow(n as usize);
    for (factor, exponent) in [
        (&axes[0], i64::from(t) + i64::from(m) - i64::from(degree)),
        (&aa, i64::from(mu) - i64::from(m)),
        (&dd, i64::from(mr) - i64::from(n)),
    ] {
        if exponent >= 0 {
            expected = &expected * &factor.pow(exponent as usize);
        } else {
            inverse = &inverse * &factor.pow((-exponent) as usize);
        }
    }
    ensure!(
        inverse == expected,
        "exact inverse polynomial identity failed"
    );
    Ok(())
}

fn strip_positive(q: &Poly, a: &Algebra) -> Result<(Poly, Poly)> {
    ensure!(!q.is_zero(), "zero polynomial is not a sign cut");
    let mut reduced = q.clone();
    let mut removed = q.one();
    for expression in ["u", "z", "r", "v", "w", "1-u", "1-r"] {
        let factor = a.parse(expression)?;
        for _ in 0..32 {
            let (quotient, remainder) = reduced.quot_rem(&factor, false);
            if !remainder.is_zero() {
                break;
            }
            reduced = quotient;
            removed = &removed * &factor;
        }
    }
    ensure!(
        &reduced * &removed == *q,
        "positive factor reconstruction failed"
    );
    Ok((reduced, removed))
}

fn coordinate_identities(a: &Algebra) -> Result<()> {
    // For each forward coordinate after the inverse chart, its numerator and
    // denominator below must reconstruct the original variable exactly.
    let x0 = a.parse("x0")?;
    let aa = a.parse("x0+x6")?;
    let dd = a.parse("x4+x7")?;
    let checks = [
        ("x0", x0.clone(), a.parse("1")?),
        ("x6", &x0 * &a.parse("x6")?, &aa - &a.parse("x6")?),
        ("x4", &x0 * &a.parse("x4")?, x0.clone()),
        (
            "x7",
            &a.parse("x4")? * &(&dd - &a.parse("x4")?),
            a.parse("x4")?,
        ),
        ("x1", &x0 * &a.parse("x1")?, x0.clone()),
        ("x2", &x0 * &a.parse("x2")?, x0),
    ];
    for (name, num, den) in checks {
        ensure!(num == &a.parse(name)? * &den, "coordinate inverse failed");
    }
    let k = a.parse("x6*x4-x0*x7")?;
    // u+r-1 = K/(A*D), checked without any division.
    ensure!(
        &k == &(&(&a.parse("x6")? * &dd) + &(&a.parse("x4")? * &aa) - &aa * &dd),
        "compact K ratio identity failed"
    );
    Ok(())
}

fn prepare(source: &Problem) -> Result<(Problem, Value)> {
    source.validate()?;
    ensure!(
        source
            .variables
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            == ORIGINAL.into_iter().collect(),
        "requires exactly the six massive base axes"
    );
    ensure!(
        source.solver.order.is_empty() || source.solver.order == ORIGINAL,
        "chart conflicts with explicit original order"
    );
    let groups = if source.variable_groups.is_empty() {
        vec![source.variables.clone()]
    } else {
        source.variable_groups.clone()
    };
    let mut last = 0;
    for name in ORIGINAL {
        let group = groups
            .iter()
            .position(|g| g.iter().any(|s| s == name))
            .context("group omits axis")?;
        ensure!(
            group >= last,
            "chart conflicts with original variable groups"
        );
        last = group;
    }
    let names = ORIGINAL.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let original = normalize(source, &names)?;
    for i in 0..6 {
        ensure!(
            original.positives.iter().any(|p| positive_coordinate(p, i)),
            "missing original positive coordinate"
        );
    }
    ensure!(
        original.split.len() == 4,
        "requires all ordered a,b,c,H sign cuts"
    );
    let targets = original
        .positives
        .iter()
        .filter(|p| !(0..6).any(|i| positive_coordinate(p, i)))
        .collect::<Vec<_>>();
    ensure!(
        targets.len() == 1,
        "requires one homogeneous endpoint inequality"
    );
    coordinate_identities(&original.algebra)?;
    // Coordinate constraints are checked as well; this prevents accidentally
    // accepting a nonhomogeneous additional premise before removing the scale.
    for p in &original.positives {
        ensure!(
            homogeneous_degree(p, &(0..6).collect::<Vec<_>>()).is_some(),
            "nonhomogeneous constraint"
        );
    }
    let ca = Algebra::new(&CHART.iter().map(|s| s.to_string()).collect::<Vec<_>>())?;
    let mut transformed = Vec::new();
    let mut records = Vec::new();
    for (label, p) in ["d", "a", "b", "c", "H"]
        .into_iter()
        .zip(targets.into_iter().chain(original.split.iter()))
    {
        let (q, m, n, degree) = forward(p, &ca)?;
        check_inverse(p, &q, &original.algebra, m, n, degree)?;
        let (reduced, positive) = strip_positive(&q, &ca)?;
        records.push(json!({"label":label,"source":original.algebra.format(p),"source_degree":degree,"source_terms":p.nterms(),
            "raw_transformed":ca.format(&q),"transformed":ca.format(&reduced),"positive_factor_removed":ca.format(&positive),
            "raw_terms":q.nterms(),"terms":reduced.nterms(),"degrees":(0..5).map(|i|reduced.degree(i)).collect::<Vec<_>>(),
            "denominator_powers":{"one_minus_u":m,"r":n},"scale_power":degree,
            "exact_forward_and_inverse_identity":true,"exact_positive_factor_identity":true}));
        transformed.push(reduced);
    }
    let mut problem = Problem::default();
    problem.name = Some("research Gluza33 full endpoint box chart".into());
    problem.variables = ca.names.clone();
    problem.variable_groups = vec![ca.names.clone()];
    problem.constraints = ["u>0", "1-u>0", "z>0", "r>0", "1-r>0", "v>0", "w>0"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    problem
        .constraints
        .push(format!("({})>0", ca.format(&transformed[0])));
    problem.split = transformed[1..].iter().map(|p| ca.format(p)).collect();
    problem.solver.engine = "local".into();
    problem.solver.ordering = "given".into();
    problem.solver.order = ca.names.clone();
    problem.solver.positivity = true;
    problem.solver.local_adjacent_pairs = true;
    problem.solver.local_descartes_witnesses = true;
    problem.solver.local_descartes_intervals = true;
    problem.solver.local_max_refinements = Some(1_000_000);
    problem.solver.local_max_nodes = Some(10_000_000);
    problem.limits = Limits {
        wall_time_secs: 120.,
        memory_mib: 4096,
        workers: 1,
        max_cells: None,
    };
    problem.validate()?;
    Ok((
        problem,
        json!({"original_order":ORIGINAL,"reduced_order":CHART,
        "forward":["x0=s","x6=s*u/(1-u)","x4=s*z","x7=s*z*(1-r)/r","x1=s*v","x2=s*w"],
        "inverse":["s=x0","u=x6/(x0+x6)","z=x4/x0","r=x4/(x4+x7)","v=x1/x0","w=x2/x0"],
        "domain":"s>0,0<u<1,z>0,0<r<1,v>0,w>0",
        "inverse_positive_denominators":["x0","x0+x6","x4+x7"],"forward_positive_denominators":["1-u","r"],
        "axis_orientations":[1,1,1,-1,1,1],"compact_identity":"K/(A*D)=u+r-1",
        "polynomials":records,"original_completion":false,"original_transport_implemented":false,
        "transport_obligation":"x7 interval reversal and chart-restricted root ordinals; all other axes have positive slope on their parent cell"}),
    ))
}

fn run(args: &Args) -> Result<()> {
    let bytes = fs::read(&args.input)?;
    let source = Problem::from_toml(std::str::from_utf8(&bytes)?)?;
    let start = Instant::now();
    let (problem, identities) = prepare(&source)?;
    fs::write(
        args.output.join("input.toml"),
        toml::to_string_pretty(&problem)?,
    )?;
    fs::write(
        args.output.join("report.json"),
        serde_json::to_vec_pretty(&json!({"source":args.input,
        "source_sha256":format!("{:x}",Sha256::digest(bytes)),"elapsed_seconds":start.elapsed().as_secs_f64(),"result":identities}))?,
    )?;
    Ok(())
}
fn main() -> Result<()> {
    let args = Args::parse();
    if args.worker {
        return run(&args);
    }
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(&args.output).context("output must be fresh")?;
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg(&args.input)
        .arg("--output")
        .arg(&args.output)
        .arg("--worker");
    let outcome = symgcad::benchmark::run_bounded(
        &mut command,
        &Limits {
            wall_time_secs: 60.,
            memory_mib: 4096,
            workers: 1,
            max_cells: None,
        },
    )?;
    fs::write(
        args.output.join("process.json"),
        serde_json::to_vec_pretty(&outcome)?,
    )?;
    ensure!(
        outcome.status == "completed" && outcome.exit_code == Some(0),
        "chart preparation failed: {} {}",
        outcome.status,
        outcome.stderr
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn base() -> Problem {
        let mut p = Problem::default();
        p.variables = ORIGINAL.iter().map(|s| s.to_string()).collect();
        p.constraints = ORIGINAL.iter().map(|s| format!("{s}>0")).collect();
        p.constraints.push("x0*x1+x2*x4-x6*x7>0".into());
        p.split = ["x0-x1", "x6-x7", "x4^2-x2*x0", "x1+x2-x4"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        p
    }
    #[test]
    fn complete_polynomial_inverse_and_positive_factor_identities() -> Result<()> {
        let (_, report) = prepare(&base())?;
        assert_eq!(report["polynomials"].as_array().unwrap().len(), 5);
        assert_eq!(report["original_completion"], false);
        let a = Algebra::new(&ORIGINAL.iter().map(|s| s.to_string()).collect::<Vec<_>>())?;
        let c = Algebra::new(&CHART.iter().map(|s| s.to_string()).collect::<Vec<_>>())?;
        for expression in ["x0+x6", "x4+x7", "x0*x6*x4*x7+x1^2*x2^2", "x6^3-x7^3"] {
            let p = a.parse(expression)?;
            let (q, m, n, d) = forward(&p, &c)?;
            check_inverse(&p, &q, &a, m, n, d)?;
            let (reduced, factor) = strip_positive(&q, &c)?;
            assert_eq!(&reduced * &factor, q);
        }
        Ok(())
    }
    #[test]
    fn missing_bound_nonhomogeneity_and_conflicting_orders_rejected() -> Result<()> {
        let mut p = base();
        p.constraints.remove(0);
        assert!(prepare(&p).is_err());
        let mut p = base();
        p.split[0] = "x0+1".into();
        assert!(prepare(&p).is_err());
        let mut p = base();
        p.solver.order = ORIGINAL.iter().rev().map(|s| s.to_string()).collect();
        assert!(prepare(&p).is_err());
        let mut p = base();
        p.variable_groups = vec![
            vec!["x7".into()],
            p.variables
                .iter()
                .filter(|v| v.as_str() != "x7")
                .cloned()
                .collect(),
        ];
        assert!(prepare(&p).is_err());
        Ok(())
    }
}
