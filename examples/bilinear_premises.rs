//! Research-only applicability screen for the strict two-fiber positive rule.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command, time::Instant};
use symbolica::poly::factor::Factorize;
use symbolica::prelude::*;
use symgcad::{
    Limits, Problem,
    algebra::{Algebra, Poly},
    domain::{CertificateOptions, certify_nonzero, derive_domain, verify_certificate},
    problem::normalize,
    solver::evaluate,
};

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    guide: PathBuf,
    #[arg(long)]
    output: PathBuf,
    /// Compare exact mathematics against an extracted JSON problem.
    #[arg(long)]
    compare_base: Option<PathBuf>,
    /// Factor the exact root-order polynomial and export research base inputs.
    #[arg(long)]
    prepare_generic_base: bool,
    #[arg(long, hide = true)]
    worker: bool,
}

fn coordinate(p: &Poly) -> bool {
    p.nterms() == 1
        && p.coefficients[0] > Rational::zero()
        && p.exponents_iter()
            .next()
            .is_some_and(|e| e.iter().map(|&x| u64::from(x)).sum::<u64>() == 1)
}

fn compare_base(left: &Problem, right: &Problem) -> Result<Value> {
    let mut names = left.variables.clone();
    names.sort();
    let mut right_names = right.variables.clone();
    right_names.sort();
    ensure!(names == right_names, "coordinate sets differ");
    let l = normalize(left, &names)?;
    let r = normalize(right, &names)?;
    let mut lp: Vec<_> = l.positives.iter().map(|p| l.algebra.format(p)).collect();
    lp.sort();
    let mut rp: Vec<_> = r.positives.iter().map(|p| r.algebra.format(p)).collect();
    rp.sort();
    let groups = |p: &Problem| {
        p.variable_groups
            .iter()
            .map(|group| {
                let mut group = group.clone();
                group.sort();
                group
            })
            .collect::<Vec<_>>()
    };
    ensure!(lp == rp, "normalized strict constraint multisets differ");
    ensure!(
        l.split == r.split,
        "ordered normalized split polynomials differ"
    );
    ensure!(
        groups(left) == groups(right),
        "ordered variable group memberships differ"
    );
    Ok(
        json!({"coordinate_sets_equal":true,"strict_constraint_multisets_exactly_equal":true,"ordered_split_polynomials_exactly_equal":true,"ordered_group_memberships_equal":true,"left_declared_variables":left.variables,"right_declared_variables":right.variables,"left_constraints":left.constraints,"right_constraints":right.constraints,"normalized_split_terms":l.split.iter().map(Poly::nterms).collect::<Vec<_>>(),"scope":"Exact normalized mathematical equality only; declaration/constraint order and operational options can still change decomposition choices."}),
    )
}

fn prepare_generic_base(
    problem: &Problem,
    screen: &mut Value,
    output: &std::path::Path,
) -> Result<()> {
    ensure!(
        problem.solver.order.is_empty(),
        "generic portfolio does not override an original fixed order"
    );
    let algebra = Algebra::new(&problem.variables)?;
    for (index, row) in screen["candidate_pairs"]
        .as_array_mut()
        .context("candidate list missing")?
        .iter_mut()
        .enumerate()
    {
        let x = row["last"]
            .as_str()
            .context("last axis missing")?
            .to_owned();
        let y = row["preceding"]
            .as_str()
            .context("preceding axis missing")?
            .to_owned();
        let delta = algebra.parse(row["delta_bc_minus_ad"].as_str().context("delta missing")?)?;
        ensure!(
            !delta.is_zero() && delta.nterms() <= 4096,
            "unsupported zero/large root-order polynomial"
        );
        let started = Instant::now();
        let factors = delta.factor();
        let rebuilt = factors
            .iter()
            .fold(delta.one(), |acc, (p, m)| &acc * &p.pow(*m));
        ensure!(rebuilt == delta, "delta factor identity failed");
        row["delta_factorization"] = json!({"factors":factors.iter().map(|(p,m)|json!({"polynomial":algebra.format(p),"multiplicity":m,"terms":p.nterms()})).collect::<Vec<_>>(),"exact_reconstruction":true,"elapsed_seconds":started.elapsed().as_secs_f64()});
        let mut base = Problem::default();
        base.name = Some(format!("research generic positive bilinear base {y},{x}"));
        base.variables = problem
            .variables
            .iter()
            .filter(|v| **v != x && **v != y)
            .cloned()
            .collect();
        base.variable_groups = problem
            .variable_groups
            .iter()
            .map(|g| {
                g.iter()
                    .filter(|v| base.variables.contains(v))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .filter(|g| !g.is_empty())
            .collect();
        let mut legal = problem.clone();
        legal.solver.order = if base.variable_groups.is_empty() {
            base.variables.clone()
        } else {
            base.variable_groups.iter().flatten().cloned().collect()
        };
        legal.solver.order.extend([y.clone(), x.clone()]);
        legal.validate()?;
        base.constraints = base.variables.iter().map(|v| format!("{v}>0")).collect();
        for label in ["a", "b", "c", "d"] {
            let expression = row["coefficients"][label]
                .as_str()
                .context("coefficient missing")?;
            ensure!(
                !algebra.parse(expression)?.is_zero(),
                "identically zero coefficient outside generic rule"
            );
            base.split.push(expression.into());
        }
        base.split.extend(
            factors
                .iter()
                .filter(|(p, _)| !p.is_constant())
                .map(|(p, _)| algebra.format(p)),
        );
        base.solver.engine = "local".into();
        base.solver.ordering = "mcts".into();
        base.solver.positivity = true;
        base.solver.local_adjacent_pairs = true;
        base.solver.local_max_refinements = Some(1_000_000);
        base.solver.local_max_nodes = Some(10_000_000);
        base.solver.search_time_secs = Some(300.);
        base.solver.max_rollouts = 256;
        base.limits = Limits {
            wall_time_secs: 1500.,
            memory_mib: 8192,
            workers: 1,
            max_cells: None,
        };
        base.validate()?;
        normalize(&base, &base.variables)?;
        fs::write(
            output.join(format!("generic-base-{index}.toml")),
            toml::to_string_pretty(&base)?,
        )?;
        let normalized = normalize(&base, &base.variables)?;
        let degrees: Vec<_> = normalized
            .split
            .iter()
            .map(|p| {
                symgcad::domain::homogeneous_degree(
                    p,
                    &(0..base.variables.len()).collect::<Vec<_>>(),
                )
            })
            .collect();
        let mut anchor = None;
        if !base.variables.is_empty() && degrees.iter().all(Option::is_some) {
            let mut anchored = base.clone();
            anchored.solver.homogeneous_block = base.variables.clone();
            anchored.solver.homogeneous_anchor = base.variables.last().cloned();
            anchored.validate()?;
            fs::write(
                output.join(format!("generic-base-{index}-anchor.toml")),
                toml::to_string_pretty(&anchored)?,
            )?;
            anchor = anchored.solver.homogeneous_anchor;
        }
        row["generic_base"] = json!({"variables":base.variables,"splits":base.split,"split_homogeneous_degrees":degrees,"delta_zero_factors_remain_explicit_splits":true,"unanchored_input":format!("generic-base-{index}.toml"),"anchored_input":anchor.as_ref().map(|_|format!("generic-base-{index}-anchor.toml")),"anchor":anchor,"original_completion":false});
    }
    Ok(())
}

fn counterexample(algebra: &Algebra, p: &Poly, domain: &[Poly]) -> Result<Option<Value>> {
    ensure!(algebra.names.len() <= 12);
    for mask in 0..(1usize << algebra.names.len()) {
        let sample: Vec<_> = (0..algebra.names.len())
            .map(|i| Rational::from(if mask & (1 << i) == 0 { 1 } else { 2 }))
            .collect();
        let value = evaluate(p, &sample);
        if value <= Rational::zero() {
            ensure!(
                domain
                    .iter()
                    .all(|q| evaluate(q, &sample) > Rational::zero()),
                "counterexample violates original positive-coordinate domain"
            );
            return Ok(Some(
                json!({"sample":sample.iter().map(ToString::to_string).collect::<Vec<_>>(),"value":value.to_string(),"domain_checked_exactly":true,"target_membership_required":false}),
            ));
        }
    }
    Ok(None)
}

fn inspect(problem: &Problem, guide_text: &str) -> Result<Value> {
    let n = normalize(problem, &problem.variables)?;
    let a = &n.algebra;
    ensure!(problem.split.is_empty() && a.names.len() <= 12);
    let targets: Vec<_> = n
        .positives
        .iter()
        .enumerate()
        .filter(|(_, p)| !coordinate(p))
        .collect();
    ensure!(
        targets.len() == 1,
        "one original target and literal positive coordinates required"
    );
    let target = targets[0].0;
    let f = targets[0].1;
    let constraints: Vec<_> = n
        .positives
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != target)
        .map(|(_, p)| p.clone())
        .collect();
    for i in 0..a.names.len() {
        ensure!(
            constraints
                .iter()
                .any(|p| coordinate(p) && p.degree(i) == 1)
        );
    }
    let domain = derive_domain(a, &constraints, &[])?;
    let mut proposed = problem.clone();
    proposed.split = vec![guide_text.into()];
    let u = normalize(&proposed, &problem.variables)?.split.remove(0);
    let mut candidates = Vec::new();
    let mut excluded = Vec::new();
    for x in 0..a.names.len() {
        for y in 0..a.names.len() {
            if x == y {
                continue;
            }
            if f.degree(x) != 1 || f.degree(y) != 1 {
                excluded.push(json!({"last":a.names[x],"preceding":a.names[y],"degrees":[f.degree(x),f.degree(y)],"reason":"target is not affine in both axes"}));
                continue;
            }
            let a0 = f.derivative(x);
            let b0 = f.replace(x, &Rational::zero());
            let aa = a0.derivative(y);
            let b = a0.replace(y, &Rational::zero());
            let c = b0.derivative(y);
            let d = b0.replace(y, &Rational::zero());
            let c0 = u.derivative(x);
            let d0 = u.replace(x, &Rational::zero());
            let c4 = c0.derivative(y);
            let d4 = c0.replace(y, &Rational::zero());
            let w0 = &b0 * &c0 - &a0 * &d0;
            let w4 = &b * &c4 - &aa * &d4;
            let delta = &b * &c - &aa * &d;
            let mut premises = Vec::new();
            let mut all = true;
            for (label, p) in [
                ("d", &d),
                ("C0", &c0),
                ("D0", &d0),
                ("W0", &w0),
                ("C4", &c4),
                ("D4", &d4),
                ("W4", &w4),
            ] {
                let witness = counterexample(a, p, &constraints)?;
                let certificate = if witness.is_none() {
                    certify_nonzero(a, p, &domain, &CertificateOptions::default())?
                } else {
                    None
                };
                let strict = certificate.as_ref().is_some_and(|proof| proof.sign == 1);
                if let Some(proof) = &certificate {
                    verify_certificate(a, p, &constraints, &[], proof)?;
                }
                all &= strict;
                premises.push(json!({"label":label,"polynomial":a.format(p),"terms":p.nterms(),"positive_certificate":certificate,"strict_positive_proved":strict,"nonpositive_counterexample":witness}));
            }
            candidates.push(json!({"last":a.names[x],"preceding":a.names[y],"coefficients":{"a":a.format(&aa),"b":a.format(&b),"c":a.format(&c),"d":a.format(&d)},"delta_bc_minus_ad":a.format(&delta),"delta_terms":delta.nterms(),"guide_affine":u.degree(x)==1&&c0.degree(y)==1,"a_b_c_nonzero":([&aa,&b,&c].iter().all(|p|!p.is_zero())),"premises":premises,"all_seven_strict_premises":all}));
        }
    }
    Ok(
        json!({"variables":a.names,"target":a.format(f),"guide":a.format(&u),"target_excluded_from_certificate_domain":target,"candidate_pairs":candidates,"excluded_pairs":excluded,"original_completion":false,"scope":"Exact applicability and rational counterexamples only. Failed positivity discovery alone is inconclusive; a retained nonpositive point refutes the corresponding strict premise on the original coordinate-only domain."}),
    )
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(!args.output.exists(), "output must be fresh");
    if !args.worker {
        if let Some(parent) = args.output.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut command = Command::new(std::env::current_exe()?);
        command
            .arg(&args.input)
            .arg("--guide")
            .arg(&args.guide)
            .arg("--output")
            .arg(&args.output)
            .arg("--worker");
        if let Some(path) = &args.compare_base {
            command.arg("--compare-base").arg(path);
        }
        if args.prepare_generic_base {
            command.arg("--prepare-generic-base");
        }
        let process = symgcad::benchmark::run_bounded(
            &mut command,
            &Limits {
                wall_time_secs: 60.,
                memory_mib: 4096,
                workers: 1,
                max_cells: None,
            },
        )?;
        fs::write(
            args.output.with_extension("process.json"),
            serde_json::to_vec_pretty(&process)?,
        )?;
        ensure!(
            process.status == "completed" && process.exit_code == Some(0),
            "screen failed: {} {}",
            process.status,
            process.stderr
        );
        return Ok(());
    }
    fs::create_dir_all(&args.output)?;
    let started = Instant::now();
    let source = fs::read(&args.input)?;
    let guide_bytes = fs::read(&args.guide)?;
    let problem = Problem::from_toml(std::str::from_utf8(&source)?)?;
    let guide: Value = serde_json::from_slice(&guide_bytes)?;
    let mut result = if let Some(path) = &args.compare_base {
        let json: Value = serde_json::from_slice(&fs::read(path)?)?;
        let other: Problem = serde_json::from_value(json.get("problem").unwrap_or(&json).clone())?;
        compare_base(&problem, &other)?
    } else {
        inspect(&problem, guide["guide"].as_str().context("guide missing")?)?
    };
    // Preserve the exact premise screen even if optional factorization hits its worker limit.
    fs::write(
        args.output.join("premises.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    if args.prepare_generic_base {
        prepare_generic_base(&problem, &mut result, &args.output)?;
    }
    fs::write(
        args.output.join("report.json"),
        serde_json::to_vec_pretty(
            &json!({"source":args.input,"source_sha256":format!("{:x}",Sha256::digest(source)),"guide_sha256":format!("{:x}",Sha256::digest(guide_bytes)),"elapsed_seconds":started.elapsed().as_secs_f64(),"screen":result}),
        )?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_positive_and_exact_zero_are_distinguished() -> Result<()> {
        let mut p = Problem::default();
        p.variables = vec!["x".into(), "y".into()];
        p.constraints = vec!["x*y+x+y+2>0".into(), "x>0".into(), "y>0".into()];
        let good = inspect(&p, "4*x*y+2*x+2*y+1")?;
        assert!(
            good["candidate_pairs"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["all_seven_strict_premises"] == true)
        );
        let bad = inspect(&p, "x*y+x+y+2")?;
        assert!(
            bad["candidate_pairs"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["all_seven_strict_premises"] == false)
        );
        assert!(
            bad["candidate_pairs"][0]["premises"]
                .as_array()
                .unwrap()
                .iter()
                .any(
                    |row| row["label"] == "W0" && row["nonpositive_counterexample"]["value"] == "0"
                )
        );
        Ok(())
    }
    #[test]
    fn missing_coordinate_domain_is_rejected() -> Result<()> {
        let mut p = Problem::default();
        p.variables = vec!["x".into(), "y".into()];
        p.constraints = vec!["x*y+x+y+2>0".into(), "x>0".into()];
        assert!(inspect(&p, "4*x*y+2*x+2*y+1").is_err());
        Ok(())
    }
    #[test]
    fn exact_comparison_allows_only_constraint_and_declaration_permutations() -> Result<()> {
        let mut a = Problem::default();
        a.variables = vec!["x".into(), "y".into()];
        a.variable_groups = vec![a.variables.clone()];
        a.constraints = vec!["x>0".into(), "y>0".into()];
        a.split = vec!["x+y-1".into(), "x-y".into()];
        let mut b = a.clone();
        b.variables.reverse();
        b.variable_groups[0].reverse();
        b.constraints.reverse();
        compare_base(&a, &b)?;
        b.split.reverse();
        assert!(compare_base(&a, &b).is_err());
        Ok(())
    }
    #[test]
    fn squared_root_order_seam_is_retained_and_false_homogeneity_not_proposed() -> Result<()> {
        let mut p = Problem::default();
        p.variables = vec!["z".into(), "y".into(), "x".into()];
        p.variable_groups = vec![p.variables.clone()];
        p.constraints = vec![
            "-x*y+x+y+z^2-2*z>0".into(),
            "x>0".into(),
            "y>0".into(),
            "z>0".into(),
        ];
        let mut screen = inspect(&p, "x*y+x+y+1")?;
        let dir = std::env::temp_dir().join(format!(
            "symgcad-bilinear-premises-test-{}",
            std::process::id()
        ));
        ensure!(!dir.exists());
        fs::create_dir(&dir)?;
        let result = prepare_generic_base(&p, &mut screen, &dir);
        fs::remove_dir_all(&dir)?;
        result?;
        for row in screen["candidate_pairs"].as_array().unwrap() {
            assert!(row["generic_base"]["anchor"].is_null());
            let a = Algebra::new(&p.variables)?;
            assert!(
                row["generic_base"]["splits"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| a.parse(v.as_str().unwrap()).unwrap() == a.parse("z-1").unwrap())
            );
            assert!(
                row["delta_factorization"]["factors"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v["multiplicity"] == 2)
            );
        }
        Ok(())
    }
}
