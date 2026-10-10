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
    /// Prove both positive endpoint projections and restrict the base to d>0.
    #[arg(long, requires = "prepare_generic_base")]
    positive_endpoint_domain: bool,
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
        if row.get("positive_endpoint_projection").is_some() {
            base.constraints.push(format!(
                "({})>0",
                row["coefficients"]["d"]
                    .as_str()
                    .context("endpoint missing")?
            ));
        }
        for label in ["a", "b", "c", "d"] {
            if label == "d" && row.get("positive_endpoint_projection").is_some() {
                continue;
            }
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
        let constraint_degrees: Vec<_> = normalized
            .positives
            .iter()
            .map(|p| {
                symgcad::domain::homogeneous_degree(
                    p,
                    &(0..base.variables.len()).collect::<Vec<_>>(),
                )
            })
            .collect();
        let mut anchor = None;
        if !base.variables.is_empty()
            && degrees.iter().all(Option::is_some)
            && constraint_degrees.iter().all(Option::is_some)
        {
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
        row["generic_base"] = json!({"variables":base.variables,"constraints":base.constraints,"splits":base.split,"split_homogeneous_degrees":degrees,"constraint_homogeneous_degrees":constraint_degrees,"delta_zero_factors_remain_explicit_splits":true,"unanchored_input":format!("generic-base-{index}.toml"),"anchored_input":anchor.as_ref().map(|_|format!("generic-base-{index}-anchor.toml")),"anchor":anchor,"original_completion":false});
    }
    Ok(())
}

fn positive_endpoint_domain(problem: &Problem, screen: &mut Value) -> Result<()> {
    let n = normalize(problem, &problem.variables)?;
    let a = &n.algebra;
    let f = a.parse(screen["target"].as_str().context("target missing")?)?;
    let u = a.parse(screen["guide"].as_str().context("guide missing")?)?;
    let positive = |p: &Poly| !p.is_zero() && p.coefficients.iter().all(|c| c > &Rational::zero());
    for row in screen["candidate_pairs"]
        .as_array_mut()
        .context("candidate list missing")?
    {
        let names = [
            row["last"]
                .as_str()
                .context("last axis missing")?
                .to_owned(),
            row["preceding"]
                .as_str()
                .context("preceding axis missing")?
                .to_owned(),
        ];
        let mut q = f.clone();
        let mut g = u.clone();
        let mut steps = Vec::new();
        for name in &names {
            let axis = a
                .names
                .iter()
                .position(|v| v == name)
                .context("unknown axis")?;
            ensure!(
                q.degree(axis) == 1 && g.degree(axis) == 1,
                "endpoint step not affine"
            );
            let leading = q.derivative(axis);
            let endpoint = q.replace(axis, &Rational::zero());
            let c = g.derivative(axis);
            let d = g.replace(axis, &Rational::zero());
            ensure!(
                positive(&c) && positive(&d),
                "endpoint guide does not have strict positive coefficients"
            );
            let w = &endpoint * &c - &leading * &d;
            let factors = w.factor();
            let rebuilt = factors
                .iter()
                .fold(w.one(), |acc, (p, m)| &acc * &p.pow(*m));
            ensure!(rebuilt == w, "endpoint comparison factor identity failed");
            ensure!(
                !w.is_zero() && factors.iter().all(|(p, m)| m % 2 == 0 || positive(p)),
                "nonnegative endpoint comparison not proved"
            );
            steps.push(json!({"axis":name,"target":a.format(&q),"guide":a.format(&g),"leading":a.format(&leading),"endpoint":a.format(&endpoint),"C":a.format(&c),"D":a.format(&d),"C_D_strict_positive_coefficients":true,"comparison":a.format(&w),"comparison_factors":factors.iter().map(|(p,m)|json!({"polynomial":a.format(p),"multiplicity":m})).collect::<Vec<_>>(),"factor_identity_verified":true,"comparison_nonnegative":true}));
            q = endpoint;
            g = d;
        }
        ensure!(
            q == a.parse(row["coefficients"]["d"].as_str().context("d missing")?)?,
            "endpoint chain differs from bilinear d"
        );
        let coefficients: [Poly; 4] = ["a", "b", "c", "d"].map(|label| {
            a.parse(row["coefficients"][label].as_str().unwrap())
                .unwrap()
        });
        let mut signs = vec![
            json!({"label":"c","positive":null,"negative":null}),
            json!({"label":"d","positive":null,"negative":null}),
        ];
        let delta = a.parse(row["delta_bc_minus_ad"].as_str().context("delta missing")?)?;
        let mut pinch = None;
        for axis in 0..a.names.len() {
            for value in [
                Rational::one(),
                Rational::from(2),
                Rational::from(4),
                Rational::from(8),
                Rational::from(15),
                Rational::from(16),
                Rational::from(32),
                Rational::from(64),
                Rational::one() / Rational::from(16),
            ] {
                let mut sample = vec![Rational::one(); a.names.len()];
                sample[axis] = value;
                for (index, p) in [(&coefficients[2]), (&coefficients[3])]
                    .into_iter()
                    .enumerate()
                {
                    let value = evaluate(p, &sample);
                    let side = if value > Rational::zero() {
                        "positive"
                    } else {
                        "negative"
                    };
                    if !value.is_zero() && signs[index][side].is_null() {
                        signs[index][side] = json!({"sample":sample.iter().map(ToString::to_string).collect::<Vec<_>>(),"value":value.to_string(),"all_coordinates_positive":sample.iter().all(|v|v>&Rational::zero())});
                    }
                }
                let values: Vec<_> = coefficients.iter().map(|p| evaluate(p, &sample)).collect();
                if pinch.is_none()
                    && evaluate(&delta, &sample).is_zero()
                    && values[0] < Rational::zero()
                    && values[1] > Rational::zero()
                    && values[2] < Rational::zero()
                    && values[3] > Rational::zero()
                {
                    let ra = -values[1].clone() / &values[0];
                    let rb = -values[3].clone() / &values[2];
                    ensure!(ra == rb && ra > Rational::zero());
                    pinch = Some(
                        json!({"sample":sample.iter().map(ToString::to_string).collect::<Vec<_>>(),"coefficients_a_b_c_d":values.iter().map(ToString::to_string).collect::<Vec<_>>(),"delta":"0","rA":ra.to_string(),"rB":rb.to_string(),"original_F":evaluate(&f,&sample).to_string(),"all_coordinates_positive":true}),
                    );
                }
            }
        }
        row["positive_endpoint_projection"] = json!({"steps":steps,"endpoint":a.format(&q),"projection_equivalence":"there exist positive x,y with F>0 iff d>0","proof":"C,D>0 and BC-AD>=0 imply B<=0 => A<=0 => Az+B<=0. Conversely B>0 gives a positive sufficiently small z. Apply twice with endpoint guides.","global_coefficient_sign_witnesses":signs,"pinched_middle_interval":pinch});
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
        if args.positive_endpoint_domain {
            command.arg("--positive-endpoint-domain");
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
    if args.positive_endpoint_domain {
        positive_endpoint_domain(&problem, &mut result)?;
    }
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
    #[test]
    fn endpoint_guides_prove_projection_and_reject_wrong_orientation() -> Result<()> {
        let mut p = Problem::default();
        p.variables = vec!["y".into(), "x".into()];
        p.constraints = vec!["2-x-y-x*y>0".into(), "x>0".into(), "y>0".into()];
        let mut report = inspect(&p, "x*y+x+y+1")?;
        positive_endpoint_domain(&p, &mut report)?;
        assert!(
            report["candidate_pairs"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["positive_endpoint_projection"]["steps"]
                    .as_array()
                    .unwrap()
                    .len()
                    == 2)
        );
        p.constraints[0] = "x*y+x+y-1>0".into();
        let mut wrong = inspect(&p, "x*y+x+y+1")?;
        assert!(positive_endpoint_domain(&p, &mut wrong).is_err());
        Ok(())
    }
    #[test]
    fn original_gluza33_has_positive_endpoint_projection_and_real_pinched_fiber() -> Result<()> {
        let mut p = Problem::from_toml(include_str!("../tests/fixtures/gluza33_negative.toml"))?;
        ensure!(p.constraints[0].contains("< 0"));
        p.constraints[0] = p.constraints[0].replacen("< 0", "> 0", 1);
        let guide: Value =
            serde_json::from_str(include_str!("../reference/gluza33_rayleigh_guide.json"))?;
        let mut report = inspect(&p, guide["guide"].as_str().unwrap())?;
        positive_endpoint_domain(&p, &mut report)?;
        let row = report["candidate_pairs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["last"] == "x3")
            .unwrap();
        let pinch = &row["positive_endpoint_projection"]["pinched_middle_interval"];
        assert_eq!(
            pinch["coefficients_a_b_c_d"],
            json!(["-110", "200", "-220", "400"])
        );
        assert_eq!(pinch["rA"], "20/11");
        assert_eq!(pinch["rB"], "20/11");
        assert_eq!(pinch["original_F"], "270");
        for row in report["candidate_pairs"].as_array().unwrap() {
            for sign in row["positive_endpoint_projection"]["global_coefficient_sign_witnesses"]
                .as_array()
                .unwrap()
            {
                assert!(!sign["positive"].is_null() && !sign["negative"].is_null());
            }
        }
        Ok(())
    }
}
