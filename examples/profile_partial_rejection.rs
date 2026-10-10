//! Research-only observation of direct-local prefixes; no new pruning rule.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    Problem,
    algebra::{Algebra, Poly, rational},
    benchmark::{run_bounded, run_bounded_observed},
    domain::{derive_domain, homogeneous_anchor},
    problem::normalize,
};

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    executable: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 20.0)]
    seconds: f64,
    #[arg(long, default_value_t = 4096)]
    memory_mib: u64,
}

fn hash(path: &Path) -> Result<String> {
    let mut f = fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = f.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        h.update(&buffer[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
fn write(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

/// Each output polynomial depends only on assigned axes; its key is the
/// remaining positive-coordinate monomial. Their sum reconstructs p exactly.
fn coefficients(p: &Poly, level: usize) -> Vec<Poly> {
    let mut groups = BTreeMap::<Vec<u32>, Poly>::new();
    for (c, m) in p.coefficients.iter().zip(p.exponents_iter()) {
        let key = m[level..].to_vec();
        let mut assigned = m.to_vec();
        assigned[level..].fill(0);
        groups
            .entry(key)
            .or_insert_with(|| p.zero())
            .append_monomial(c.clone(), &assigned);
    }
    groups.into_values().filter(|p| !p.is_zero()).collect()
}
fn signs(groups: &[Poly], prefix: &[Rational], dimension: usize) -> (usize, usize, usize, usize) {
    let mut point = vec![Rational::one(); dimension];
    point[..prefix.len()].clone_from_slice(prefix);
    let (mut positive, mut negative, mut zero, mut guard_zero) = (0, 0, 0, 0);
    for p in groups {
        let value = p.replace_all(&point);
        if value.is_zero() {
            zero += 1;
            guard_zero += usize::from(!p.is_zero());
        } else if value.is_negative() {
            negative += 1;
        } else {
            positive += 1;
        }
    }
    (positive, negative, zero, guard_zero)
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds > 0.0 && args.seconds <= 120.0 && args.memory_mib <= 8192,
        "research bounds exceeded"
    );
    ensure!(!args.output.exists(), "fresh output directory required");
    let started = Instant::now();
    fs::create_dir_all(&args.output)?;
    let output = args.output.canonicalize()?;
    let mut problem = Problem::from_toml(&fs::read_to_string(&args.input)?)?;
    ensure!(
        problem
            .name
            .as_deref()
            .is_some_and(|n| n.starts_with("test_gluza_ex"))
            && problem.split.is_empty(),
        "only original unsplit Gluza inequality inputs are supported"
    );
    let original = problem.clone();
    problem.solver.engine = "local".into();
    problem.solver.ordering = "brown".into();
    problem.solver.positivity = true;
    problem.solver.input_positivity = true;
    problem.solver.checkpoint_dir = None;
    problem.solver.order = symgcad::solver::selected_order(&problem)?;
    problem.solver.ordering = "given".into();
    problem.limits.workers = 1;
    problem.limits.memory_mib = args.memory_mib;
    problem.limits.wall_time_secs = args.seconds;
    problem.validate()?;
    if !original.solver.order.is_empty() {
        ensure!(
            problem.solver.order == original.solver.order,
            "original explicit fixed order changed"
        );
    }
    let normalized = normalize(&problem, &problem.solver.order)?;
    let domain = derive_domain(&normalized.algebra, &normalized.positives, &[])?;
    ensure!(
        domain
            .bounds
            .iter()
            .all(|b| b.lower.as_deref() == Some("0")),
        "original strict positivity is required for every coordinate"
    );
    let anchor = problem.solver.homogeneous_anchor.as_ref();
    let mut polys = normalized.positives.clone();
    if let Some(name) = anchor {
        let axis = problem
            .solver
            .order
            .iter()
            .position(|n| n == name)
            .context("anchor missing")?;
        let block = problem
            .solver
            .homogeneous_block
            .iter()
            .map(|n| problem.solver.order.iter().position(|v| v == n).unwrap())
            .collect::<Vec<_>>();
        ensure!(
            homogeneous_anchor(&normalized.algebra, &polys, &block, axis, &domain)?.is_some(),
            "anchor lacks exact homogeneous identity"
        );
        polys = polys
            .into_iter()
            .map(|p| p.replace(axis, &Rational::one()))
            .collect();
    }
    let order = problem
        .solver
        .order
        .iter()
        .filter(|n| Some(*n) != anchor)
        .cloned()
        .collect::<Vec<_>>();
    let algebra = Algebra::new(&order)?;
    let targets = polys
        .iter()
        .enumerate()
        .filter(|(_, p)| p.nterms() > 1)
        .map(|(index, p)| Ok((index, algebra.parse(&normalized.algebra.format(p))?)))
        .collect::<Result<Vec<_>>>()?;
    ensure!(!targets.is_empty(), "no nonlinear original inequality");
    let grouped = targets
        .iter()
        .map(|(_, p)| {
            (0..order.len())
                .map(|level| coefficients(p, level))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let binary = output.join("symgcad");
    let worker_hash = hash(&args.executable)?;
    fs::copy(&args.executable, &binary)?;
    ensure!(hash(&binary)? == worker_hash, "worker changed during copy");
    let input = output.join("input.json");
    write(&input, &problem)?;
    fs::write(output.join("input.toml"), toml::to_string_pretty(&problem)?)?;
    let result = output.join("result.json");
    let progress = output.join("progress.json");
    let mut command = Command::new(&binary);
    command
        .arg("_worker")
        .arg("solve")
        .arg(&input)
        .arg("--output")
        .arg(&result)
        .env("SYMGCAD_PROGRESS_PATH", &progress)
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    let mut snapshots = Vec::new();
    let mut observed = BTreeSet::new();
    let mut last_poll = 0.0;
    let process = run_bounded_observed(&mut command, &problem.limits, |elapsed| {
        if elapsed.as_secs_f64() - last_poll < 0.05 || snapshots.len() >= 2048 {
            return None;
        }
        last_poll = elapsed.as_secs_f64();
        if let Ok(bytes) = fs::read(&progress) {
            if let Ok(v) = serde_json::from_slice::<Value>(&bytes) {
                let key = v["current_prefix"].to_string();
                if v["current_prefix"].is_array() && observed.insert(key) {
                    snapshots.push(json!({"observed_seconds":elapsed.as_secs_f64(),"prefix":v["current_prefix"],"current_level":v["current_level"],"speculative_nodes":v["speculative_nodes"],"refinements":v["refinements"]}));
                }
            }
        }
        None
    })?;
    write(&output.join("process.json"), &process)?;
    write(&output.join("snapshots.json"), &snapshots)?;
    let evaluation_start = Instant::now();
    let mut rows = Vec::new();
    let mut rejected_prefixes = BTreeSet::new();
    let mut summaries = BTreeMap::<usize, (usize, usize)>::new();
    for (sample, snapshot) in snapshots.iter().enumerate() {
        let prefix = snapshot["prefix"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| rational(v.as_str().unwrap()))
            .collect::<Result<Vec<_>>>()?;
        if prefix.len() >= order.len() || prefix.iter().any(|q| q <= &Rational::zero()) {
            continue;
        }
        let level = prefix.len();
        let mut any_rejected = false;
        for (target, (constraint, p)) in targets.iter().enumerate() {
            if (level..order.len()).all(|v| p.degree(v) == 0) {
                continue;
            }
            let groups = &grouped[target][level];
            let (positive, negative, zero, guard_zero) = signs(groups, &prefix, order.len());
            let reject = positive == 0 && zero == 0 && negative > 0;
            any_rejected |= reject;
            if reject {
                rejected_prefixes.insert(sample);
            }
            rows.push(json!({"snapshot":sample,"constraint":constraint,"level":level,"remaining_axes":order.len()-level,"positive_coefficients":positive,"negative_coefficients":negative,"zero_coefficients":zero,"zero_guard_obstructions":guard_zero,"candidate_rejection":reject,"earlier_coefficient_guards":if reject && rows.len()<64 { Some(groups.iter().filter(|p|!p.is_constant()).map(|p|algebra.format(p)).collect::<Vec<_>>()) } else { None }}));
        }
        let entry = summaries.entry(order.len() - level).or_default();
        entry.0 += 1;
        entry.1 += usize::from(any_rejected);
    }
    let evaluation_seconds = evaluation_start.elapsed().as_secs_f64();
    let mut verification = None;
    if process.status == "completed" && process.exit_code == Some(0) {
        let summary: Value = serde_json::from_str(&process.stdout).unwrap_or(Value::Null);
        if summary["status"] == "complete_generic" {
            let mut command = Command::new(&binary);
            command
                .arg("_worker")
                .arg("verify")
                .arg(&result)
                .arg("--output")
                .arg(output.join("verification.json"));
            let mut limits = problem.limits.clone();
            limits.wall_time_secs = 60.0;
            verification = Some(run_bounded(&mut command, &limits)?);
        }
    }
    let report = json!({"schema_version":1,"research_only":true,"source":args.input,"source_sha256":hash(&args.input)?,"worker_sha256":worker_hash,"harness_sha256":hash(&std::env::current_exe()?)?,"engine":"direct local original inequalities; no sign_filter","problem":problem,"active_order":order,"normalized_targets":targets.iter().map(|(i,p)|json!({"constraint":i,"polynomial":algebra.format(p)})).collect::<Vec<_>>(),"process":process,"verification_process":verification,"observed_prefixes":snapshots.len(),"candidate_rejected_prefixes":rejected_prefixes.len(),"by_remaining_axes":summaries.iter().map(|(axes,(observed,rejected))|json!({"axes":axes,"observed":observed,"rejected":rejected})).collect::<Vec<_>>(),"rows":rows,"evaluation_seconds":evaluation_seconds,"total_seconds":started.elapsed().as_secs_f64(),"limitation":"observed progress prefixes are sparse and not an exhaustive traversal; a candidate requires coefficient guards and original positive-axis premises before production pruning; no discarded region or completion is inferred"});
    write(&output.join("report.json"), &report)?;
    println!(
        "{}",
        json!({"observed_prefixes":snapshots.len(),"candidate_rejected_prefixes":rejected_prefixes.len(),"by_remaining_axes":report["by_remaining_axes"],"process_status":report["process"]["status"]})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_remaining_coefficients_certify_only_a_guarded_fiber() {
        let a = Algebra::new(&["x".into(), "y".into(), "z".into()]).unwrap();
        let p = a.parse("(x-2)*y-x*z-1").unwrap();
        let groups = coefficients(&p, 1);
        assert_eq!(signs(&groups, &[Rational::one()], 3), (0, 3, 0, 0));
        assert_eq!(signs(&groups, &[Rational::from(3)], 3), (1, 2, 0, 0));
        assert_eq!(signs(&groups, &[Rational::from(2)], 3), (0, 2, 1, 1));
    }
    #[test]
    fn grouping_combines_the_full_assigned_coefficient_polynomial() {
        let a = Algebra::new(&["x".into(), "y".into()]).unwrap();
        let p = a.parse("x*y-y+x^2-3*x").unwrap();
        let groups = coefficients(&p, 1);
        assert_eq!(groups.len(), 2);
        assert_eq!(signs(&groups, &[Rational::from((1, 2))], 2), (0, 2, 0, 0));
        assert_eq!(signs(&groups, &[Rational::one()], 2), (0, 1, 1, 1));
    }
}
