//! Research-only joint homogeneous-anchor and exact two-stage projection costs.
//! Every candidate retains the standard projection basis. No CAD completion,
//! root deletion, or reuse after an unchecked variable-map change is asserted.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    Limits, Problem,
    algebra::{Algebra, Poly},
    benchmark::run_bounded,
    domain::{derive_domain, homogeneous_anchor},
    optimized_projection::{OptimizedProjectionOptions, ProjectionCache},
    problem::normalize,
};

const INPUT_BYTES: usize = 1024 * 1024;
const REPORT_BYTES: usize = 16 * 1024 * 1024;
const MAX_CANDIDATES: usize = 8;
const STAGES: usize = 2;

#[derive(Parser)]
struct Args {
    problem: PathBuf,
    #[arg(long)]
    output: PathBuf,
    /// Explicit Feynman block; must agree with an existing nonempty block.
    #[arg(long, value_delimiter = ',')]
    block: Vec<String>,
    /// Restrict the anchor portfolio; never overrides an original fixed anchor.
    #[arg(long, value_delimiter = ',')]
    anchors: Vec<String>,
    #[arg(long, default_value_t = 30.0)]
    seconds: f64,
    #[arg(long, default_value_t = MAX_CANDIDATES)]
    candidates: usize,
    #[arg(long, default_value_t = 0)]
    seed: u64,
    #[arg(long, default_value_t = 0)]
    cpu: usize,
    #[arg(long, hide = true)]
    worker_request: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Candidate {
    id: usize,
    anchor: Option<String>,
    /// Rank in the cheap degree/term ordering of legal active first axes.
    first_rank: usize,
    /// Rank on the actual first projected basis, not on the original input.
    second_rank: usize,
}

#[derive(Serialize, Deserialize)]
struct Request {
    problem: Problem,
    source_sha256: String,
    candidate: Candidate,
    directory: PathBuf,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read(path: &Path, cap: usize) -> Result<Vec<u8>> {
    ensure!(
        fs::symlink_metadata(path)?.file_type().is_file(),
        "regular file required"
    );
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(cap as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= cap, "input/report byte cap");
    Ok(bytes)
}

fn write(path: &Path, value: &Value) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    ensure!(bytes.len() <= REPORT_BYTES, "report byte cap");
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn groups(problem: &Problem) -> Vec<Vec<String>> {
    if problem.variable_groups.is_empty() {
        vec![problem.variables.clone()]
    } else {
        problem.variable_groups.clone()
    }
}

fn fixed(problem: &Problem) -> bool {
    !problem.solver.order.is_empty() || problem.solver.ordering == "given"
}

fn given_order(problem: &Problem) -> Vec<String> {
    if problem.solver.order.is_empty() {
        groups(problem).into_iter().flatten().collect()
    } else {
        problem.solver.order.clone()
    }
}

fn group_of(problem: &Problem, name: &str) -> usize {
    groups(problem)
        .iter()
        .position(|g| g.iter().any(|v| v == name))
        .unwrap()
}

fn anchor_order_legal(problem: &Problem, anchor: &str) -> bool {
    let block = &problem.solver.homogeneous_block;
    if !block.iter().any(|v| v == anchor) {
        return false;
    }
    if fixed(problem) {
        let order = given_order(problem);
        let position = order.iter().position(|v| v == anchor).unwrap();
        block
            .iter()
            .all(|v| order.iter().position(|n| n == v).unwrap() >= position)
    } else {
        let group = group_of(problem, anchor);
        block.iter().all(|v| group_of(problem, v) >= group)
    }
}

fn validate_anchor_menu(problem: &Problem, menu: &[String]) -> Result<()> {
    ensure!(
        menu.len() <= MAX_CANDIDATES && menu.iter().collect::<BTreeSet<_>>().len() == menu.len(),
        "anchor menu cap or duplicates"
    );
    ensure!(
        menu.iter()
            .all(|a| problem.solver.homogeneous_block.contains(a)),
        "anchor menu must lie in the explicit homogeneous block"
    );
    if let Some(anchor) = &problem.solver.homogeneous_anchor {
        ensure!(
            menu.is_empty() || menu == std::slice::from_ref(anchor),
            "anchor menu must not override the original fixed anchor"
        );
    }
    ensure!(
        menu.iter().all(|a| anchor_order_legal(problem, a)),
        "anchor menu violates fixed order or variable groups"
    );
    Ok(())
}

fn plans(problem: &Problem, count: usize, menu: &[String]) -> Vec<Candidate> {
    let anchors = if let Some(anchor) = &problem.solver.homogeneous_anchor {
        vec![Some(anchor.clone())]
    } else if !menu.is_empty() {
        menu.iter().cloned().map(Some).collect()
    } else {
        std::iter::once(None)
            .chain(
                problem
                    .solver
                    .homogeneous_block
                    .iter()
                    .filter(|a| anchor_order_legal(problem, a))
                    .cloned()
                    .map(Some),
            )
            .collect::<Vec<_>>()
    };
    let mut result = Vec::new();
    // Bounded beam diversity: compare actual second-stage alternatives before
    // spending every slot on additional first axes. Round-robin across gauges.
    let ranks = if fixed(problem) {
        vec![(0, 0)]
    } else {
        vec![
            (0, 0),
            (1, 0),
            (0, 1),
            (1, 1),
            (2, 0),
            (2, 1),
            (0, 2),
            (1, 2),
        ]
    };
    for (first_rank, second_rank) in ranks {
        for anchor in &anchors {
            if anchor
                .as_ref()
                .is_some_and(|a| !anchor_order_legal(problem, a))
            {
                continue;
            }
            let last_group = groups(problem)
                .into_iter()
                .rev()
                .find(|g| g.iter().any(|v| Some(v) != anchor.as_ref()));
            let possible = last_group.as_ref().map_or(0, |g| {
                g.iter().filter(|v| Some(*v) != anchor.as_ref()).count()
            });
            if first_rank >= possible.max(1) {
                continue;
            }
            result.push(Candidate {
                id: result.len(),
                anchor: anchor.clone(),
                first_rank,
                second_rank,
            });
            if result.len() == count {
                return result;
            }
        }
    }
    result
}

fn checked_block(problem: &mut Problem, block: &[String]) -> Result<()> {
    if !block.is_empty() {
        ensure!(
            problem.solver.homogeneous_block.is_empty()
                || problem.solver.homogeneous_block == block,
            "CLI block must agree with the original homogeneous block"
        );
        problem.solver.homogeneous_block = block.to_vec();
    }
    problem.validate()?;
    ensure!(
        problem.variables.len() <= 32 && problem.constraints.len() + problem.split.len() <= 256,
        "research variable/polynomial cap"
    );
    Ok(())
}

fn bounded(polynomials: &[Poly], algebra: &Algebra, initial: bool) -> Result<()> {
    let terms = polynomials
        .iter()
        .try_fold(0usize, |n, p| n.checked_add(p.nterms()))
        .context("term count overflow")?;
    ensure!(
        polynomials.len() <= 1024 && terms <= if initial { 20_000 } else { 100_000 },
        "research basis factor/term cap"
    );
    ensure!(
        polynomials
            .iter()
            .all(|p| p.variables() == &algebra.variables
                && p.nterms() <= 20_000
                && (0..p.nvars()).all(|i| p.degree(i) <= 256)
                && p.coefficients
                    .iter()
                    .all(|c| c.numerator_ref().significant_bits() <= 8192
                        && c.denominator_ref().significant_bits() <= 8192)),
        "research polynomial map/term/degree/coefficient cap"
    );
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Cost {
    factors: usize,
    terms: usize,
    maximum_degree: u32,
    maximum_coefficient_bits: u64,
    basis_sha256: String,
    polynomial_sha256: Vec<String>,
}

fn cost(algebra: &Algebra, basis: &[Poly]) -> Result<Cost> {
    bounded(basis, algebra, false)?;
    let expressions = basis.iter().map(|p| algebra.format(p)).collect::<Vec<_>>();
    let serialized = serde_json::to_vec(&(&algebra.names, &expressions))?;
    ensure!(
        serialized.len() <= REPORT_BYTES,
        "basis expression byte cap"
    );
    Ok(Cost {
        factors: basis.len(),
        terms: basis.iter().map(Poly::nterms).sum(),
        maximum_degree: basis
            .iter()
            .flat_map(|p| (0..p.nvars()).map(|i| p.degree(i)))
            .max()
            .unwrap_or(0),
        maximum_coefficient_bits: basis
            .iter()
            .flat_map(|p| &p.coefficients)
            .map(|c| {
                c.numerator_ref()
                    .significant_bits()
                    .max(c.denominator_ref().significant_bits())
            })
            .max()
            .unwrap_or(0),
        basis_sha256: hash(&serialized),
        polynomial_sha256: expressions.iter().map(|p| hash(p.as_bytes())).collect(),
    })
}

fn legal_axes(problem: &Problem, remaining: &BTreeSet<String>) -> Vec<String> {
    if fixed(problem) {
        return given_order(problem)
            .into_iter()
            .rev()
            .find(|v| remaining.contains(v))
            .into_iter()
            .collect();
    }
    groups(problem)
        .into_iter()
        .rev()
        .map(|g| {
            g.into_iter()
                .filter(|v| remaining.contains(v))
                .collect::<Vec<_>>()
        })
        .find(|g| !g.is_empty())
        .unwrap_or_default()
}

fn axis_choices(
    problem: &Problem,
    algebra: &Algebra,
    basis: &[Poly],
    remaining: &BTreeSet<String>,
) -> Vec<String> {
    let mut legal = legal_axes(problem, remaining);
    let score = |name: &String| {
        let axis = algebra.names.iter().position(|v| v == name).unwrap();
        let degree = basis.iter().map(|p| p.degree(axis)).max().unwrap_or(0);
        let bearing_terms = basis
            .iter()
            .filter(|p| p.degree(axis) > 0)
            .map(Poly::nterms)
            .sum::<usize>();
        (degree, bearing_terms, axis)
    };
    if !fixed(problem) {
        let active = legal.iter().any(|v| score(v).0 > 0);
        if active {
            legal.retain(|v| score(v).0 > 0);
            legal.sort_by_key(score);
        } else {
            // The last group is inactive. Carry this forced elimination but
            // offer no redundant inactive-variable permutations as candidates.
            legal.reverse();
            legal.truncate(1);
        }
    }
    legal
}

fn original_order(
    problem: &Problem,
    anchor: Option<&String>,
    eliminated: &[String],
) -> Result<Vec<String>> {
    if fixed(problem) {
        let order = given_order(problem);
        let expected = order
            .iter()
            .rev()
            .filter(|v| Some(*v) != anchor)
            .take(eliminated.len())
            .cloned()
            .collect::<Vec<_>>();
        ensure!(expected == eliminated, "prefix changes fixed order");
        return Ok(order);
    }
    let mut order = groups(problem)
        .into_iter()
        .flatten()
        .filter(|v| Some(v) != anchor && !eliminated.contains(v))
        .collect::<Vec<_>>();
    order.extend(eliminated.iter().rev().cloned());
    if let Some(anchor) = anchor {
        let anchor_group = group_of(problem, anchor);
        let insertion = order
            .iter()
            .position(|v| group_of(problem, v) >= anchor_group)
            .unwrap_or(order.len());
        order.insert(insertion, anchor.clone());
        let position = order.iter().position(|v| v == anchor).unwrap();
        ensure!(
            problem.solver.homogeneous_block.iter().all(|v| order
                .iter()
                .position(|n| n == v)
                .unwrap()
                >= position),
            "restored anchor does not precede its ratios"
        );
    }
    let mut check = problem.clone();
    check.solver.order = order.clone();
    check.validate()?;
    Ok(order)
}

fn restart_problem(
    problem: &Problem,
    anchor: Option<&String>,
    eliminated: &[String],
) -> Result<Problem> {
    let mut restart = problem.clone();
    restart.solver.order = original_order(problem, anchor, eliminated)?;
    restart.solver.ordering = "given".into();
    restart.solver.homogeneous_anchor = anchor.cloned();
    if anchor.is_none() {
        restart.solver.homogeneous_block.clear();
    }
    restart.validate()?;
    Ok(restart)
}

fn work(request: &Request) -> Result<Value> {
    let progress = request.directory.join("candidate.json");
    let mut report = json!({
        "kind":"research_exact_projection_prefix_not_decomposition", "status":"running",
        "candidate":request.candidate, "source_sha256":request.source_sha256, "seed":0,
        "phase":"normalization", "full_variable_map":request.problem.variables,
        "stages":[], "consecutive_all_linear_stages":0,
        "checkpoint_scope":"helper full map only; reordered/reduced real solves require checked map transport",
        "claimed_complete_generic":false, "domain_removals":0,
    });
    write(&progress, &report)?;
    let result = (|| -> Result<()> {
        let started = Instant::now();
        let normalized = normalize(&request.problem, &request.problem.variables)?;
        let algebra = &normalized.algebra;
        let mut all = normalized.positives.clone();
        all.extend(normalized.split.clone());
        bounded(&all, algebra, true)?;
        report["normalized_original_constraints"] = json!(
            normalized
                .positives
                .iter()
                .map(|p| algebra.format(p))
                .collect::<Vec<_>>()
        );
        report["normalized_original_split"] = json!(
            normalized
                .split
                .iter()
                .map(|p| algebra.format(p))
                .collect::<Vec<_>>()
        );
        report["normalization_seconds"] = json!(started.elapsed().as_secs_f64());
        if normalized
            .positives
            .iter()
            .any(|p| p.is_zero() || (p.is_constant() && p.coefficients[0] < Rational::zero()))
        {
            report["status"] = json!("declined_empty_strict_domain");
            return Ok(());
        }
        if let Some(anchor) = &request.candidate.anchor {
            ensure!(
                anchor_order_legal(&request.problem, anchor),
                "anchor incompatible with order/groups"
            );
            let axis = algebra
                .names
                .iter()
                .position(|v| v == anchor)
                .context("unknown anchor")?;
            let block = request
                .problem
                .solver
                .homogeneous_block
                .iter()
                .map(|v| algebra.names.iter().position(|n| n == v).unwrap())
                .collect::<Vec<_>>();
            let domain = derive_domain(algebra, &normalized.positives, &[])?;
            let Some(evidence) = homogeneous_anchor(algebra, &all, &block, axis, &domain)? else {
                report["status"] = json!("declined_homogeneity_or_positive_anchor");
                return Ok(());
            };
            // Keep the original full map, including the now inactive scale.
            all = all
                .iter()
                .map(|p| p.replace(axis, &Rational::one()))
                .collect();
            ensure!(
                all.iter()
                    .zip(&evidence.normalized_polynomials)
                    .all(|(p, text)| algebra.format(p) == *text),
                "anchor substitution replay mismatch"
            );
            report["anchor_evidence"] = json!(evidence);
        }
        report["phase"] = json!("factor_initial_basis");
        write(&progress, &report)?;
        let options = OptimizedProjectionOptions {
            checkpoint_dir: Some(request.directory.join("cache")),
            progress_path: Some(request.directory.join("projection-progress.json")),
            capture_dir: Some(request.directory.join("captures")),
            max_capture_bytes: 4 * 1024 * 1024,
            memory_cache_bytes: 64 * 1024 * 1024,
            max_checkpoint_bytes: REPORT_BYTES,
            // All standard obligations remain; no domain-pruning API is used.
            ..Default::default()
        };
        let mut cache = ProjectionCache::new(algebra, &options);
        let factor_started = Instant::now();
        let mut basis = cache.initial_basis(&all)?;
        report["factor_initial_seconds"] = json!(factor_started.elapsed().as_secs_f64());
        report["initial_basis"] = json!(cost(algebra, &basis)?);
        report["restart_candidate_problem"] = json!(restart_problem(
            &request.problem,
            request.candidate.anchor.as_ref(),
            &[]
        )?);
        let mut remaining = request
            .problem
            .variables
            .iter()
            .filter(|v| Some(*v) != request.candidate.anchor.as_ref())
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut eliminated = Vec::new();
        let mut linear_prefix = 0;
        let mut nonlinear_seen = false;
        // A CAD projection stops before its final lifting variable.
        let requested_stages = STAGES.min(remaining.len().saturating_sub(1));
        report["requested_stages"] = json!(requested_stages);
        for stage in 0..requested_stages {
            let choices = axis_choices(&request.problem, algebra, &basis, &remaining);
            let rank = if stage == 0 {
                request.candidate.first_rank
            } else {
                request.candidate.second_rank
            };
            let Some(name) = choices.get(rank).cloned() else {
                report["status"] = json!("declined_duplicate_or_inactive_first_rank");
                return Ok(());
            };
            let axis = algebra.names.iter().position(|v| v == &name).unwrap();
            let bearing = basis.iter().filter(|p| p.degree(axis) > 0).count();
            let maximum_degree = basis.iter().map(|p| p.degree(axis)).max().unwrap_or(0);
            let all_linear = bearing > 0 && maximum_degree == 1;
            let input = cost(algebra, &basis)?;
            report["phase"] = json!("project_one");
            report["active_axis"] = json!(name);
            report["active_stage"] = json!(stage);
            report["active_legal_axis_ranking"] = json!(choices);
            report["pending_input_basis"] = json!(input);
            write(&progress, &report)?;
            let clock = Instant::now();
            // initial_basis and each preceding project_one return the required
            // square-free pairwise-coprime factor basis. Never project raw
            // products with shared factors and then discard zero resultants.
            let output = cache.project_one(&basis, axis)?;
            let projection_seconds = clock.elapsed().as_secs_f64();
            bounded(&output, algebra, false)?;
            ensure!(
                output.iter().all(|p| p.degree(axis) == 0
                    && eliminated.iter().all(|v| p
                        .degree(algebra.names.iter().position(|n| n == v).unwrap())
                        == 0)),
                "projected basis reintroduced an eliminated axis"
            );
            let output_cost = cost(algebra, &output)?;
            if bearing > 0 {
                if all_linear && !nonlinear_seen {
                    linear_prefix += 1;
                }
                if !all_linear {
                    nonlinear_seen = true;
                }
            }
            eliminated.push(name.clone());
            remaining.remove(&name);
            report["stages"].as_array_mut().unwrap().push(json!({
                "axis":name, "axis_index":axis, "bearing_factors":bearing,
                "all_linear":all_linear, "forced_inactive":bearing==0,
                "input_axis_degree":maximum_degree, "input":input,"output":output_cost,
                "factor_growth":[output_cost.factors,input.factors.max(1)],
                "term_growth":[output_cost.terms,input.terms.max(1)],
                "coefficient_bit_growth":[output_cost.maximum_coefficient_bits,input.maximum_coefficient_bits.max(1)],
                "projection_seconds":projection_seconds,
            }));
            report["consecutive_all_linear_stages"] = json!(linear_prefix);
            report["elimination_prefix"] = json!(eliminated);
            report["original_lifting_order"] = json!(original_order(
                &request.problem,
                request.candidate.anchor.as_ref(),
                &eliminated
            )?);
            report["restart_candidate_problem"] = json!(restart_problem(
                &request.problem,
                request.candidate.anchor.as_ref(),
                &eliminated
            )?);
            report["phase"] = json!("stage_completed");
            report["projection_report"] = json!(cache.report);
            write(&progress, &report)?;
            basis = output;
        }
        let order = original_order(
            &request.problem,
            request.candidate.anchor.as_ref(),
            &eliminated,
        )?;
        let restart = restart_problem(
            &request.problem,
            request.candidate.anchor.as_ref(),
            &eliminated,
        )?;
        report["restart_candidate_problem"] = json!(restart);
        report["original_lifting_order"] = json!(order);
        report["reduced_lifting_order"] = json!(
            report["original_lifting_order"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| v.as_str() != request.candidate.anchor.as_deref())
                .collect::<Vec<_>>()
        );
        report["final_basis"] = json!(cost(algebra, &basis)?);
        report["projection_report"] = json!(cache.report);
        report["status"] = json!("prefix_completed_not_solved");
        report["phase"] = json!("finished");
        report["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
        Ok(())
    })();
    if let Err(error) = result {
        report["status"] = json!("failed_or_declined");
        report["error"] = json!(format!("{error:#}"));
    }
    write(&progress, &report)?;
    Ok(report)
}

fn ratio_compare(a: &Value, b: &Value, field: &str) -> Ordering {
    let numerator = |v: &Value| v["final_basis"][field].as_u64().unwrap_or(u64::MAX);
    let denominator = |v: &Value| v["initial_basis"][field].as_u64().unwrap_or(1).max(1);
    (u128::from(numerator(a)) * u128::from(denominator(b)))
        .cmp(&(u128::from(numerator(b)) * u128::from(denominator(a))))
}

fn compare_candidates(a: &Value, b: &Value) -> Ordering {
    let a = &a["result"];
    let b = &b["result"];
    b["consecutive_all_linear_stages"]
        .as_u64()
        .unwrap_or(0)
        .cmp(&a["consecutive_all_linear_stages"].as_u64().unwrap_or(0))
        .then_with(|| ratio_compare(a, b, "factors"))
        .then_with(|| ratio_compare(a, b, "terms"))
        .then_with(|| ratio_compare(a, b, "maximum_coefficient_bits"))
        .then_with(|| {
            a["candidate"]["id"]
                .as_u64()
                .cmp(&b["candidate"]["id"].as_u64())
        })
}

fn supervise(args: &Args) -> Result<()> {
    let started = Instant::now();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 30.0,
        "total profile budget must be in (0,30] seconds"
    );
    ensure!(
        (1..=MAX_CANDIDATES).contains(&args.candidates),
        "candidate cap is1..=8"
    );
    ensure!(
        args.seed == 0,
        "this deterministic profiler supports seed0 only"
    );
    ensure!(
        !args.output.exists(),
        "use a fresh output directory to preserve existing evidence"
    );
    fs::create_dir_all(&args.output)?;
    let source = read(&args.problem, INPUT_BYTES)?;
    let mut problem = Problem::from_toml(std::str::from_utf8(&source)?)?;
    checked_block(&mut problem, &args.block)?;
    validate_anchor_menu(&problem, &args.anchors)?;
    let candidates = plans(&problem, args.candidates, &args.anchors);
    ensure!(!candidates.is_empty(), "no legal anchor/order candidate");
    let executable = std::env::current_exe()?;
    let mut report = json!({
        "kind":"research_joint_anchor_two_stage_portfolio_not_completion", "seed":0,
        "source":args.problem,"source_sha256":hash(&source),"problem":problem,
        "binary_sha256":hash(&fs::read(&executable)?),"candidate_plans":candidates,
        "anchor_menu":args.anchors,
        "total_budget_seconds":args.seconds,"memory_mib":4096,"workers":1,"cpu":args.cpu,
        "records":[],"ranking":[],"restart_candidate":null,
        "score_note":"completed prefixes only: descending consecutive nontrivial all-linear stages, then exact factor/term/coefficient-bit growth ratios; predictions, never solved-region evidence",
        "tail_completion":"unused variables retain their input order within lifting groups; this is not a Brown-ranked tail",
        "cache_note":"full original variable maps retained; no automatic reuse in differently mapped production solves",
    });
    for (index, candidate) in candidates.iter().enumerate() {
        let remaining = args.seconds - started.elapsed().as_secs_f64();
        if remaining <= 0.05 {
            break;
        }
        let seconds = (remaining - 0.025) / (candidates.len() - index) as f64;
        let directory = args.output.join(format!("candidate-{:02}", candidate.id));
        fs::create_dir_all(&directory)?;
        let request = Request {
            problem: problem.clone(),
            source_sha256: hash(&source),
            candidate: candidate.clone(),
            directory: directory.clone(),
        };
        let request_path = directory.join("request.json");
        write(&request_path, &serde_json::to_value(&request)?)?;
        write(&args.output.join("report.json"), &report)?;
        let mut command = Command::new("taskset");
        command
            .arg("-c")
            .arg(args.cpu.to_string())
            .arg(&executable)
            .arg(&args.problem)
            .arg("--output")
            .arg(&args.output)
            .arg("--worker-request")
            .arg(&request_path);
        let limits = Limits {
            wall_time_secs: seconds,
            memory_mib: 4096,
            workers: 1,
            max_cells: None,
        };
        let process = run_bounded(&mut command, &limits)?;
        let result = read(&directory.join("candidate.json"), REPORT_BYTES)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
        report["records"].as_array_mut().unwrap().push(json!({
            "candidate":candidate,"directory":directory,"limits":limits,"process":process,"result":result,
        }));
        report["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
        write(&args.output.join("report.json"), &report)?;
    }
    let mut ranked = report["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| {
            r["process"]["status"] == "completed"
                && r["process"]["exit_code"] == 0
                && r["result"]["status"] == "prefix_completed_not_solved"
        })
        .cloned()
        .collect::<Vec<_>>();
    ranked.sort_by(compare_candidates);
    report["ranking"] = json!(
        ranked
            .iter()
            .map(|r| r["candidate"]["id"].clone())
            .collect::<Vec<_>>()
    );
    let mut partial = report["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["result"]["restart_candidate_problem"].is_object())
        .cloned()
        .collect::<Vec<_>>();
    partial.sort_by(|a, b| {
        b["result"]["stages"]
            .as_array()
            .map_or(0, Vec::len)
            .cmp(&a["result"]["stages"].as_array().map_or(0, Vec::len))
            .then_with(|| {
                a["candidate"]["id"]
                    .as_u64()
                    .cmp(&b["candidate"]["id"].as_u64())
            })
    });
    report["restart_candidate"] = ranked
        .first()
        .or_else(|| partial.first())
        .map(|r| r["result"]["restart_candidate_problem"].clone())
        .unwrap_or(Value::Null);
    report["restart_scope"] = json!(if !ranked.is_empty() {
        "finished short prefix; heuristic prediction only"
    } else {
        "interrupted or declined prefix; completed intermediate work only; no winner established"
    });
    report["status"] = json!(if ranked.is_empty() {
        "incomplete_no_finished_prefix"
    } else {
        "research_prefix_candidates_available_not_solved"
    });
    report["source_unchanged"] = json!(hash(&read(&args.problem, INPUT_BYTES)?) == hash(&source));
    report["executable_unchanged"] =
        json!(hash(&fs::read(&executable)?) == report["binary_sha256"].as_str().unwrap());
    if report["source_unchanged"] != true || report["executable_unchanged"] != true {
        report["status"] = json!("invalidated_changed_source_or_executable");
        report["ranking"] = json!([]);
        report["restart_candidate"] = Value::Null;
    }
    report["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
    write(&args.output.join("report.json"), &report)?;
    println!(
        "{}",
        json!({"report":args.output.join("report.json"),"status":report["status"],"ranking":report["ranking"],"elapsed_seconds":report["elapsed_seconds"]})
    );
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    if let Some(path) = &args.worker_request {
        let request: Request = serde_json::from_slice(&read(path, REPORT_BYTES)?)?;
        request.problem.validate()?;
        work(&request)?;
        Ok(())
    } else {
        supervise(&args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn problem() -> Problem {
        Problem {
            variables: vec!["s".into(), "x".into(), "y".into(), "z".into()],
            constraints: vec!["s>4".into(), "x>0".into(), "y>0".into(), "z>0".into()],
            split: vec!["x^2+y*z-s*x*y".into()],
            variable_groups: vec![vec!["s".into()], vec!["x".into(), "y".into(), "z".into()]],
            solver: symgcad::config::SolverOptions {
                ordering: "brown".into(),
                homogeneous_block: vec!["x".into(), "y".into(), "z".into()],
                ..Default::default()
            },
            ..Default::default()
        }
    }
    #[test]
    fn legal_groups_fixed_order_and_anchor_prefix_are_preserved() -> Result<()> {
        let mut p = problem();
        for plan in plans(&p, 8, &[]) {
            let last = if plan.anchor.as_deref() == Some("z") {
                "y"
            } else {
                "z"
            };
            let order = original_order(&p, plan.anchor.as_ref(), &[last.into()])?;
            assert_eq!(order[0], "s");
            if let Some(anchor) = plan.anchor {
                assert_eq!(order[1], anchor);
            }
        }
        p.solver.order = vec!["s".into(), "y".into(), "x".into(), "z".into()];
        let candidates = plans(&p, 8, &[]);
        assert!(candidates.iter().all(|c| c.first_rank == 0));
        assert_eq!(
            candidates
                .iter()
                .filter_map(|c| c.anchor.as_deref())
                .collect::<Vec<_>>(),
            vec!["y"]
        );
        assert!(original_order(&p, Some(&"y".into()), &["x".into()]).is_err());
        assert_eq!(
            original_order(&p, Some(&"y".into()), &["z".into(), "x".into()])?,
            p.solver.order
        );
        Ok(())
    }
    #[test]
    fn exact_parameter_substitution_precedes_anchor_admission() -> Result<()> {
        let mut p = problem();
        p.parameters.insert("mass".into(), "5/7".into());
        p.split = vec!["mass*x^2+y*z".into()];
        let n = normalize(&p, &p.variables)?;
        let mut all = n.positives.clone();
        all.extend(n.split.clone());
        let domain = derive_domain(&n.algebra, &n.positives, &[])?;
        let certificate = homogeneous_anchor(&n.algebra, &all, &[1, 2, 3], 1, &domain)?.unwrap();
        assert_eq!(certificate.degrees, vec![0, 1, 1, 1, 2]);
        assert_eq!(
            n.algebra
                .parse(certificate.normalized_polynomials.last().unwrap())?,
            n.algebra.parse("5+7*y*z")?
        );
        all.push(n.algebra.parse("x+1")?);
        assert!(homogeneous_anchor(&n.algebra, &all, &[1, 2, 3], 1, &domain)?.is_none());
        Ok(())
    }
    #[test]
    fn first_linear_projection_can_create_nonlinear_next_axis() -> Result<()> {
        let a = Algebra::new(&["x".into(), "y".into(), "z".into()])?;
        let mut cache = ProjectionCache::new(&a, &Default::default());
        let input = cache.initial_basis(&[a.parse("y*z+x")?, a.parse("z-y")?])?;
        assert!(input.iter().all(|p| p.degree(2) <= 1));
        let first = cache.project_one(&input, 2)?;
        assert!(first.iter().any(|p| p.degree(1) == 2));
        let second = cache.project_one(&first, 1)?;
        assert!(second.contains(&symgcad::algebra::canonical(&a.parse("x")?)));
        Ok(())
    }
    #[test]
    fn shared_factor_zero_resultant_requires_factor_first_rebasing() -> Result<()> {
        let a = Algebra::new(&["x".into(), "y".into(), "z".into()])?;
        let mut cache = ProjectionCache::new(&a, &Default::default());
        let p = a.parse("(z-x)*(z-y)")?;
        let q = a.parse("(z-x)*(z-1)")?;
        assert!(cache.resultant(&p, &q, 2)?.is_zero());
        let basis = cache.initial_basis(&[p, q])?;
        assert_eq!(basis.len(), 3);
        let projected = cache.project_one(&basis, 2)?;
        for required in ["x-y", "x-1", "y-1"] {
            assert!(projected.contains(&symgcad::algebra::canonical(&a.parse(required)?)));
        }
        assert!(projected.iter().all(|p| !p.is_zero()));
        Ok(())
    }
    #[test]
    fn active_axes_precede_inactive_choices_without_crossing_groups() -> Result<()> {
        let mut p = problem();
        let n = normalize(&p, &p.variables)?;
        let basis = vec![n.algebra.parse("x+y")?];
        let remaining = p.variables.iter().cloned().collect();
        assert_eq!(
            axis_choices(&p, &n.algebra, &basis, &remaining),
            vec!["x", "y"]
        );
        p.variable_groups = vec![vec!["s".into(), "x".into(), "y".into()], vec!["z".into()]];
        assert_eq!(axis_choices(&p, &n.algebra, &basis, &remaining), vec!["z"]);
        Ok(())
    }
    #[test]
    fn anchor_menu_and_rank_diversity_preserve_fixed_bindings() -> Result<()> {
        let mut p = problem();
        let menu = vec!["x".into(), "y".into()];
        validate_anchor_menu(&p, &menu)?;
        let candidates = plans(&p, 8, &menu);
        assert_eq!(candidates.len(), 8);
        assert_eq!(
            candidates
                .iter()
                .map(|c| (c.first_rank, c.second_rank))
                .collect::<Vec<_>>(),
            vec![
                (0, 0),
                (0, 0),
                (1, 0),
                (1, 0),
                (0, 1),
                (0, 1),
                (1, 1),
                (1, 1)
            ]
        );
        assert!(
            candidates
                .iter()
                .enumerate()
                .all(|(i, c)| c.anchor.as_ref() == Some(&menu[i % 2]))
        );
        p.solver.homogeneous_anchor = Some("x".into());
        assert!(validate_anchor_menu(&p, &menu).is_err());
        validate_anchor_menu(&p, &["x".into()])?;
        p.solver.order = vec!["s".into(), "x".into(), "y".into(), "z".into()];
        let candidates = plans(&p, 8, &["x".into()]);
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            (candidates[0].first_rank, candidates[0].second_rank),
            (0, 0)
        );
        Ok(())
    }
    #[test]
    fn alternative_second_axis_retains_different_collision_guard() -> Result<()> {
        let a = Algebra::new(&["t".into(), "a".into(), "b".into(), "c".into()])?;
        let mut cache = ProjectionCache::new(&a, &Default::default());
        let basis = cache.initial_basis(&[
            a.parse("a")?,
            a.parse("b")?,
            a.parse("c")?,
            a.parse("a*c+a+b+c+t")?,
        ])?;
        let first = cache.project_one(&basis, 1)?;
        let remove_b = cache.project_one(&first, 2)?;
        let remove_c = cache.project_one(&first, 3)?;
        assert!(remove_c.contains(&symgcad::algebra::canonical(&a.parse("b+t-1")?)));
        assert!(remove_b.iter().all(|p| p.degree(2) == 0));
        assert!(remove_b != remove_c);
        Ok(())
    }
}
