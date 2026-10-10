//! Test-only replay of the real generated-guard path. The captured operand
//! refines a synthetic unconstrained problem; this is never an A377 solve.
use super::*;
use anyhow::Context;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read(path: &Path) -> Result<Vec<u8>> {
    ensure!(
        fs::symlink_metadata(path)?.file_type().is_file(),
        "regular input required"
    );
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 16 * 1024 * 1024, "input byte cap");
    Ok(bytes)
}

// The authenticated candidate uses explicit integer monomials in s. Check
// grammar, term/degree/digit bounds before allocating polynomial arithmetic.
fn parse_candidate(algebra: &Algebra, text: &str) -> Result<Poly> {
    ensure!(
        algebra.names.first().is_some_and(|s| s == "s"),
        "first axis must be s"
    );
    ensure!(base_univariate::map_admissible(algebra), "map cap");
    ensure!(text.len() <= 16 * 1024 * 1024, "expression cap");
    let mut entries = Vec::new();
    let mut last = None;
    for term in text.split('+') {
        ensure!(entries.len() < 4096, "term cap");
        let (coefficient, exponent) = term
            .strip_prefix('(')
            .and_then(|s| s.split_once(")*s^"))
            .context("integer monomial grammar")?;
        let digits = coefficient.strip_prefix('-').unwrap_or(coefficient);
        ensure!(
            !digits.is_empty()
                && digits.len() <= 6000
                && digits.bytes().all(|b| b.is_ascii_digit()),
            "coefficient grammar/height cap"
        );
        ensure!(
            !exponent.is_empty()
                && exponent.len() <= 4
                && exponent.bytes().all(|b| b.is_ascii_digit()),
            "exponent grammar"
        );
        let exponent: u32 = exponent.parse()?;
        ensure!(
            exponent <= 4096 && last.is_none_or(|p| exponent > p),
            "degree/order cap"
        );
        last = Some(exponent);
        entries.push((coefficient, exponent));
    }
    let mut p = algebra.parse("0")?;
    for (coefficient, exponent) in entries {
        let c = crate::algebra::rational(coefficient)?;
        ensure!(
            !c.is_zero() && c.numerator_ref().significant_bits() <= 20_000,
            "zero or over-height coefficient"
        );
        let mut powers = vec![0; algebra.names.len()];
        powers[0] = exponent;
        p.append_monomial(c, &powers);
    }
    ensure!(
        base_univariate::source_cost(&p).is_some(),
        "source admission cap"
    );
    Ok(p)
}

fn stage(output: Option<&Path>, value: Value) -> Result<()> {
    if let Some(output) = output {
        let temporary = output.join("progress.tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(&value)?)?;
        fs::rename(temporary, output.join("progress.json"))?;
    }
    Ok(())
}

fn replay(
    names: Vec<String>,
    raw: Poly,
    output: Option<&Path>,
) -> Result<(LocalSolveResult, Value)> {
    let started = Instant::now();
    let mut problem = Problem {
        variables: names.clone(),
        ..Default::default()
    };
    problem.solver.engine = "local".into();
    problem.solver.ordering = "given".into();
    problem.solver.local_base_univariate_basis = true;
    problem.limits.wall_time_secs = 300.0;
    problem.limits.memory_mib = 4096;
    problem.limits.max_cells = Some(1024);
    problem.validate()?;
    let normalized = normalize(&problem, &names)?;
    ensure!(
        raw.variables() == &normalized.algebra.variables,
        "captured full map changed"
    );
    let options = LocalOptions {
        base_univariate_basis: true,
        domain_pruning: false,
        positive_ray: false,
        defer_projection: false,
        fallback_to_global: false,
        ..Default::default()
    };
    let cache_options = OptimizedProjectionOptions {
        memory_cache_bytes: 0,
        ..Default::default()
    };
    let mut builder = Builder {
        problem: &problem,
        normalized: &normalized,
        options: &options,
        started,
        polynomials: Vec::new(),
        ids: HashMap::new(),
        input_factors: Vec::new(),
        input_levels: vec![BTreeSet::new(); names.len()],
        report: LocalReport {
            base_univariate_basis: Some(BaseUnivariateReport::default()),
            ..Default::default()
        },
        operations: CachedOperations {
            cache: ProjectionCache::new(&normalized.algebra, &cache_options),
            phases: LocalPhaseCosts::default(),
            progress_path: None,
            progress_context: None,
            last_captured_context: None,
            started,
        },
        progress_path: None,
        last_progress: started,
        guard_domains: (0..names.len())
            .map(|i| guard_domain(&normalized, i))
            .collect::<Result<_>>()?,
        protected: BTreeSet::new(),
        guard_certificates: Vec::new(),
        guard_certificate_index: HashMap::new(),
        completed_subtrees: subtree_cache::CompletedSubtrees::new(0),
        learned_cuts: learned_cuts::LearnedCuts::new(1024 * 1024),
        hint_archive: None,
        descartes_stock: descartes::WitnessStock::default(),
        interval_stock: interval_descartes::WitnessStock::new(2)?,
        pair_witness_stock: pair_witnesses::WitnessStock::default(),
        bezout_registry: strict_bezout::Registry::default(),
        obstruction_observer: None,
        base_sources: base_univariate::SourceIndex::default(),
        base_records: Vec::new(),
        base_typed: BTreeSet::new(),
        base_retained_bits: 0,
    };
    stage(
        output,
        json!({"phase":"production_guard_basis","original_coverage_claimed":false}),
    )?;
    let cuts = builder
        .guard_basis(std::slice::from_ref(&raw))
        .map_err(|e| anyhow!("guard basis: {e:?}"))?;
    ensure!(
        builder.base_records.len() == 1 && !cuts.is_empty(),
        "source was not admitted"
    );
    let source = normalized
        .algebra
        .parse(&builder.base_records[0].polynomial)?;
    ensure!(
        &source * &source.constant(raw.lcoeff() / source.lcoeff()) == raw,
        "signed original source identity"
    );
    ensure!(
        builder
            .guard_basis(std::slice::from_ref(&raw))
            .map_err(|e| anyhow!("guard hit: {e:?}"))?
            == cuts,
        "immutable basis changed"
    );
    ensure!(
        builder.report.base_univariate_basis.as_ref().unwrap().hits == 1,
        "missing immutable hit"
    );
    builder.learned_cuts.remember(&[], &cuts);
    stage(
        output,
        json!({"phase":"production_visit","original_coverage_claimed":false,
        "basis":builder.report.base_univariate_basis}),
    )?;
    let built = builder
        .visit(&mut Vec::new())
        .map_err(|e| anyhow!("support visit: {e:?}"))?;
    ensure!(built.required.is_empty(), "unproved top-level guard");
    let mut cells = Vec::new();
    let mut used = BTreeSet::new();
    let mut statistics = Statistics::default();
    collect_output(
        &built.node,
        &names,
        &mut Vec::new(),
        &mut cells,
        &mut used,
        &mut statistics,
    );
    statistics.raw_cells = cells.len();
    statistics.total_seconds = started.elapsed().as_secs_f64();
    let mut levels = vec![Vec::new(); names.len()];
    for &id in &used {
        levels[highest(&builder.polynomials[id]).unwrap()].push(id);
    }
    let mut solution = SolveResult::failure(problem.clone(), Status::CompleteGeneric, "");
    solution.order = names.clone();
    solution.diagnostic = None;
    solution.polynomials = builder
        .polynomials
        .iter()
        .map(|p| normalized.algebra.format(p))
        .collect();
    solution.projection_levels = levels;
    solution.exceptional_polynomials = used.into_iter().collect();
    solution.cells = cells;
    solution.statistics = statistics;
    let evidence = LocalEvidence::Local {
        options: options.clone(),
        input_factors: Vec::new(),
        guard_certificates: Vec::new(),
        base_univariate_basis: Some(BaseUnivariateBasis {
            schema_version: base_univariate::SCHEMA,
            sources: builder.base_records.clone(),
        }),
        root: built.node,
        report: builder.snapshot(),
    };
    let solve_seconds = started.elapsed().as_secs_f64();
    // Replay gets no builder, operation cache, source memo or typed ID set.
    drop(builder);
    drop(normalized);
    stage(
        output,
        json!({"phase":"fresh_checker_guard_basis","original_coverage_claimed":false,
        "solve_seconds":solve_seconds}),
    )?;
    let replay_started = Instant::now();
    let normalized = normalize(&solution.problem, &solution.order)?;
    let polynomials = solution
        .polynomials
        .iter()
        .map(|s| normalized.algebra.parse(s))
        .collect::<Result<Vec<_>>>()?;
    let LocalEvidence::Local {
        base_univariate_basis,
        ..
    } = &evidence
    else {
        unreachable!()
    };
    let checker = Checker {
        normalized: &normalized,
        polynomials: &polynomials,
        ids: polynomials
            .iter()
            .cloned()
            .enumerate()
            .map(|(id, p)| (p, id))
            .collect(),
        input_factors: &[],
        guard_certificates: &[],
        guard_domains: (0..names.len())
            .map(|i| guard_domain(&normalized, i))
            .collect::<Result<_>>()?,
        protected: BTreeSet::new(),
        domain_pruning: false,
        positive_ray: false,
        positive_pair_separation: false,
        positive_pair_cancellations: 1,
        positive_pair_witnesses: false,
        strict_bezout: false,
        bezout_registry: std::cell::RefCell::new(strict_bezout::Registry::default()),
        stable_sector_samples: false,
        descartes_one: false,
        descartes_witnesses: false,
        descartes_intervals: false,
        base_point_projection: true,
        adjacent_pairs: false,
        binary_discriminants: false,
        base_basis: base_univariate::validate(
            &normalized.algebra,
            &polynomials,
            base_univariate_basis.as_ref(),
            true,
        )?,
    };
    ensure!(
        checker.guard_basis(std::slice::from_ref(&raw))? == cuts,
        "fresh raw-guard inventory changed"
    );
    drop(checker);
    let guard_replay_seconds = replay_started.elapsed().as_secs_f64();
    stage(
        output,
        json!({"phase":"full_independent_verifier","original_coverage_claimed":false,
        "solve_seconds":solve_seconds,"guard_replay_seconds":guard_replay_seconds}),
    )?;
    let verify_started = Instant::now();
    let verified = verify_local_parts(&solution, &evidence)?;
    ensure!(verified.verified, "support proof rejected");
    let verify_seconds = verify_started.elapsed().as_secs_f64();
    let LocalEvidence::Local {
        root: LocalNode::Stack { roots, sectors, .. },
        report,
        ..
    } = &evidence
    else {
        anyhow::bail!("missing support stack")
    };
    ensure!(
        roots.len() + 1 == sectors.len() && solution.cells.len() == sectors.len(),
        "support coverage count"
    );
    let summary = json!({"schema_version":1,"complete":true,"synthetic_support_harness":true,
        "original_coverage_claimed":false,"verified":true,"variables":names,
        "raw_signed_sha256":sha(normalized.algebra.format(&raw).as_bytes()),
        "raw_degree":raw.degree(0),"raw_terms":raw.nterms(),
        "source_records":base_univariate_basis,"basis_report":report.base_univariate_basis,
        "root_count":roots.len(),"base_sector_count":sectors.len(),"roots":roots,
        "solve_seconds":solve_seconds,"guard_replay_seconds":guard_replay_seconds,
        "verify_seconds":verify_seconds,"total_pipeline_seconds":started.elapsed().as_secs_f64()});
    Ok((LocalSolveResult { solution, evidence }, summary))
}

#[test]
fn small_signed_repeated_full_map_production_replay() {
    let names = ["s", "t", "unused"].map(String::from).to_vec();
    let algebra = Algebra::new(&names).unwrap();
    let raw = algebra.parse("-3/5*s^2*(s-4)^3*(s^2-2)*(s^2+1)").unwrap();
    let (result, report) = replay(names, raw, None).unwrap();
    assert_eq!(report["root_count"], 4);
    assert_eq!(report["base_sector_count"], 5);
    assert!(result.solution.cells.iter().all(|c| c.axes.len() == 3));
    let mut forged = result.clone();
    if let LocalEvidence::Local {
        root: LocalNode::Stack { roots, .. },
        ..
    } = &mut forged.evidence
    {
        roots[0].index += 1;
    }
    assert!(verify_local(&forged).is_err());
    let mut forged = result;
    if let LocalEvidence::Local {
        base_univariate_basis: Some(basis),
        ..
    } = &mut forged.evidence
    {
        basis.sources[0].blocks[0].multiplicity += 1;
    }
    assert!(verify_local(&forged).is_err());
    for bad in ["(1)*(s+1)^4000", "(1)*s^4097", "(1)*t^2", "(1)*s^2+(1)*s^2"] {
        assert!(parse_candidate(&algebra, bad).is_err());
    }
    assert_eq!(
        parse_candidate(&algebra, "(-2)*s^0+(1)*s^2").unwrap(),
        algebra.parse("s^2-2").unwrap()
    );
}

#[test]
#[ignore = "requires authenticated local capture and an external 300s/4GiB supervisor"]
fn captured_a377_discriminant_production_replay() -> Result<()> {
    let candidate_path = PathBuf::from(
        std::env::var_os("SYMGCAD_BASE_REPLAY_CANDIDATE").context("explicit candidate required")?,
    );
    let capture_path = PathBuf::from(
        std::env::var_os("SYMGCAD_BASE_REPLAY_CAPTURE").context("explicit capture required")?,
    );
    let output = PathBuf::from(
        std::env::var_os("SYMGCAD_BASE_REPLAY_OUTPUT")
            .context("fresh output directory required")?,
    );
    ensure!(!output.exists(), "output already exists");
    let bytes = read(&candidate_path)?;
    let capture_bytes = read(&capture_path)?;
    let candidate_sha = sha(&bytes);
    let capture_sha = sha(&capture_bytes);
    ensure!(
        candidate_sha == "0c457cfce6ed2df4545054b7ba79b76e62123e284a665ea39db12bc4124c09c2",
        "candidate binding changed"
    );
    ensure!(
        capture_sha == "df2a381b027bbbf7e721c47c4533d11177a9172038375ae0a8fd029a7c4aab92",
        "capture binding changed"
    );
    let candidate: Value = serde_json::from_slice(&bytes)?;
    let capture: Value = serde_json::from_slice(&capture_bytes)?;
    ensure!(
        candidate["operation"] == "discriminant"
            && candidate["input_hash"] == capture["input_hash"],
        "operation binding"
    );
    let names: Vec<String> = serde_json::from_value(capture["variable_names"].clone())?;
    ensure!(
        names == ["s", "t", "x0", "x3", "x5", "x6", "x8"],
        "original full map changed"
    );
    let algebra = Algebra::new(&names)?;
    let text = candidate["result"]
        .as_str()
        .context("candidate polynomial")?;
    ensure!(
        sha(text.as_bytes()) == candidate["result_sha256"],
        "expression digest changed"
    );
    let raw = parse_candidate(&algebra, text)?;
    ensure!(
        raw.degree(0) == 2169 && raw.nterms() == 2030,
        "captured degree/terms changed"
    );
    fs::create_dir_all(&output)?;
    let (result, mut report) = replay(names, raw, Some(&output))?;
    ensure!(
        report["root_count"] == 84 && report["base_sector_count"] == 85,
        "captured support inventory changed"
    );
    report["candidate_sha256"] = json!(candidate_sha);
    report["capture_sha256"] = json!(capture_sha);
    report["input_hash"] = candidate["input_hash"].clone();
    let proof = serde_json::to_vec(&result)?;
    report["synthetic_proof_sha256"] = json!(sha(&proof));
    fs::write(output.join("synthetic-support-proof.json"), proof)?;
    fs::write(
        output.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    stage(Some(&output), report)?;
    Ok(())
}
