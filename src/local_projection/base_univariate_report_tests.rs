use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

fn builder<'a>(
    problem: &'a Problem,
    normalized: &'a NormalizedProblem,
    options: &'a LocalOptions,
) -> Builder<'a> {
    let started = Instant::now();
    Builder {
        problem,
        normalized,
        options,
        started,
        polynomials: Vec::new(),
        ids: HashMap::new(),
        input_factors: Vec::new(),
        input_levels: vec![BTreeSet::new(); problem.variables.len()],
        report: LocalReport {
            base_univariate_basis: Some(BaseUnivariateReport::default()),
            ..Default::default()
        },
        operations: CachedOperations {
            cache: ProjectionCache::new(
                &normalized.algebra,
                &OptimizedProjectionOptions::default(),
            ),
            phases: LocalPhaseCosts::default(),
            progress_path: None,
            progress_context: None,
            last_captured_context: None,
            started,
        },
        progress_path: None,
        last_progress: started,
        last_base_progress: None,
        guard_domains: (0..problem.variables.len())
            .map(|i| guard_domain(normalized, i).unwrap())
            .collect(),
        protected: BTreeSet::new(),
        guard_certificates: Vec::new(),
        guard_certificate_index: HashMap::new(),
        completed_subtrees: subtree_cache::CompletedSubtrees::new(0),
        learned_cuts: learned_cuts::LearnedCuts::new(0),
        hint_archive: None,
        descartes_stock: descartes::WitnessStock::default(),
        interval_stock: interval_descartes::WitnessStock::new(2).unwrap(),
        pair_witness_stock: pair_witnesses::WitnessStock::default(),
        bezout_registry: strict_bezout::Registry::default(),
        obstruction_observer: None,
        base_sources: base_univariate::SourceIndex::default(),
        base_records: Vec::new(),
        base_typed: BTreeSet::new(),
        base_retained_bits: 0,
    }
}

#[test]
fn more_than_128_small_sources_replay_with_exact_duplicate_and_scale_hits() {
    let problem = Problem {
        variables: vec!["x".into(), "z".into()],
        ..Default::default()
    };
    let normalized = normalize(&problem, &problem.variables).unwrap();
    let options = LocalOptions {
        base_univariate_basis: true,
        ..Default::default()
    };
    let mut b = builder(&problem, &normalized, &options);
    let mut raw = Vec::new();
    for i in 1..=160 {
        raw.push(normalized.algebra.parse(&format!("x^2+{i}")).unwrap());
    }
    let ids = b.guard_basis(&raw).unwrap();
    assert_eq!(b.base_records.len(), 160);
    assert_eq!(ids.len(), 160);
    let storage = b
        .report
        .base_univariate_basis
        .as_ref()
        .unwrap()
        .retained_storage;
    assert_eq!(storage.source_terms, 320);
    assert_eq!(storage.block_terms, 320);
    let scaled = raw
        .iter()
        .map(|p| p.clone().mul_coeff(Rational::from((-7, 3))))
        .collect::<Vec<_>>();
    assert_eq!(b.guard_basis(&scaled).unwrap(), ids);
    assert_eq!(b.guard_basis(&raw).unwrap(), ids);
    assert_eq!(b.base_records.len(), 160);
    assert_eq!(
        b.report
            .base_univariate_basis
            .as_ref()
            .unwrap()
            .retained_storage,
        storage
    );
    assert_eq!(b.report.base_univariate_basis.as_ref().unwrap().hits, 320);
    let evidence = BaseUnivariateBasis {
        schema_version: base_univariate::SCHEMA,
        sources: b.base_records.clone(),
    };
    let encoded = serde_json::to_vec(&evidence).unwrap();
    let decoded: BaseUnivariateBasis = serde_json::from_slice(&encoded).unwrap();
    let checked =
        base_univariate::validate(&normalized.algebra, &b.polynomials, Some(&decoded), true)
            .unwrap();
    for p in &scaled {
        assert!(checked.proportional_ids(p).is_some());
    }
    let mut legacy = decoded;
    legacy.schema_version = 1;
    assert!(
        base_univariate::validate(&normalized.algebra, &b.polynomials, Some(&legacy), true)
            .is_err()
    );
    legacy.sources.truncate(128);
    base_univariate::validate(&normalized.algebra, &b.polynomials, Some(&legacy), true).unwrap();
}

#[test]
fn source_aggregate_decline_precedes_native_and_blocks_remain_resource_stops() {
    let problem = Problem {
        variables: vec!["x".into(), "z".into()],
        ..Default::default()
    };
    let normalized = normalize(&problem, &problem.variables).unwrap();
    let options = LocalOptions {
        base_univariate_basis: true,
        ..Default::default()
    };
    let q = normalized.algebra.parse("(x-1)*(x-2)").unwrap();
    let mut b = builder(&problem, &normalized, &options);
    // Boundary routing only: no proof is constructed from synthetic counters.
    b.report
        .base_univariate_basis
        .as_mut()
        .unwrap()
        .retained_storage
        .source_terms = base_univariate::MAX_REGISTRY_TERMS;
    assert_eq!(b.guard_basis(std::slice::from_ref(&q)).unwrap().len(), 2);
    let report = b.report.base_univariate_basis.as_ref().unwrap();
    assert_eq!(report.declines.registry_terms, 1);
    assert_eq!(report.decomposition_seconds, 0.0);
    assert_eq!(report.sources, 0);
    let mut b = builder(&problem, &normalized, &options);
    b.report
        .base_univariate_basis
        .as_mut()
        .unwrap()
        .retained_storage
        .source_terms = base_univariate::MAX_REGISTRY_TERMS - q.nterms() as u64;
    assert!(matches!(b.guard_basis(&[q]), Err(Stop::Resource(_))));
    assert_eq!(b.operations.cache.report.cache.factor_misses, 0);
    assert_eq!(b.report.base_univariate_basis.as_ref().unwrap().sources, 0);
}

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "symgcad-base-report-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn decline_checkpoint_precedes_factor_fallback_and_survives_capture_refresh() {
    let directory = Directory::new();
    let problem = Problem {
        variables: vec!["x".into(), "z".into()],
        ..Default::default()
    };
    let normalized = normalize(&problem, &problem.variables).unwrap();
    let options = LocalOptions {
        base_univariate_basis: true,
        ..Default::default()
    };
    let mut b = builder(&problem, &normalized, &options);
    let q = normalized.algebra.parse("(x-1)*(x-2)").unwrap();
    // Exercise the existing admission boundary without allocating an enormous
    // live registry. No proof is constructed from this synthetic accounting.
    b.base_retained_bits =
        base_univariate::MAX_RETAINED_BITS - base_univariate::source_cost(&q).unwrap() + 1;
    b.report
        .base_univariate_basis
        .as_mut()
        .unwrap()
        .retained_total_bits = b.base_retained_bits;
    let path = directory.0.join("progress.json");
    b.progress_path = Some(path.clone());
    b.operations.progress_path = Some(path.clone());
    b.operations.progress_context = Some(LocalReport::default()); // stale context
    let factors = b.guard_basis(&[q]).unwrap();
    assert_eq!(factors.len(), 2);
    assert_eq!(b.operations.cache.report.cache.factor_misses, 1);
    let before: LocalReport = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        before.cache.factor_misses, 0,
        "snapshot must precede factorization"
    );
    let report = before.base_univariate_basis.unwrap();
    assert_eq!(report.static_declines, 1);
    assert_eq!(report.declines.registry_storage, 1);
    assert_eq!(report.retained_total_bits, b.base_retained_bits);
    let attempt = report.last_attempt.unwrap();
    assert_eq!(attempt.phase, "ordinary_fallback");
    assert_eq!(
        attempt.decline,
        Some(BaseUnivariateDecline::RegistryStorage)
    );
    assert!(attempt.proposed_total_bits.unwrap() > base_univariate::MAX_RETAINED_BITS);
    b.operations.capture_context().unwrap();
    let refreshed: LocalReport = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(refreshed.cache.factor_misses, 1);
    assert_eq!(
        refreshed
            .base_univariate_basis
            .unwrap()
            .declines
            .registry_storage,
        1
    );
}

#[test]
fn diagnostic_write_failure_does_not_change_fallback_or_failure_routing() {
    let directory = Directory::new();
    let problem = Problem {
        variables: vec!["x".into(), "z".into()],
        ..Default::default()
    };
    let normalized = normalize(&problem, &problem.variables).unwrap();
    let options = LocalOptions {
        base_univariate_basis: true,
        ..Default::default()
    };
    let mut b = builder(&problem, &normalized, &options);
    let bad_path = directory.0.join("a-directory");
    std::fs::create_dir(&bad_path).unwrap();
    b.progress_path = Some(bad_path.clone());
    assert_eq!(
        b.guard_basis(&[normalized.algebra.parse("x").unwrap()])
            .unwrap()
            .len(),
        1
    );
    // The >=64-term capture hook used to skip this already captured prefix.
    // Metadata refresh must not force a new fatal write to the invalid path.
    b.operations.progress_path = Some(bad_path);
    b.operations.progress_context = Some(b.snapshot());
    b.operations.last_captured_context =
        Some((b.report.current_level, b.report.current_prefix.clone()));
    let multivariate = normalized.algebra.parse("z+(x+1)^63").unwrap();
    assert!(multivariate.nterms() >= 64);
    assert_eq!(b.guard_basis(&[multivariate]).unwrap().len(), 1);
    assert_eq!(
        b.report
            .base_univariate_basis
            .as_ref()
            .unwrap()
            .declines
            .map_or_support,
        1
    );
    assert_eq!(
        b.report
            .base_univariate_basis
            .as_ref()
            .unwrap()
            .declines
            .degree,
        1
    );
    assert!(matches!(
        b.observed_base_stop(Stop::Resource("cap".into())),
        Stop::Resource(_)
    ));
    assert!(matches!(
        b.observed_base_stop(Stop::Failed("identity".into())),
        Stop::Failed(_)
    ));
    let report = b.report.base_univariate_basis.as_ref().unwrap();
    assert_eq!(report.post_entry_resource_stops, 1);
    assert_eq!(report.post_entry_failures, 1);
}

#[test]
fn source_text_decline_is_bounded_and_published_before_either_algebra_backend() {
    use symbolica::domains::integer::Integer;
    let directory = Directory::new();
    let problem = Problem {
        variables: vec!["x".into()],
        ..Default::default()
    };
    let normalized = normalize(&problem, &problem.variables).unwrap();
    let options = LocalOptions {
        base_univariate_basis: true,
        ..Default::default()
    };
    let mut b = builder(&problem, &normalized, &options);
    let c = Rational::from(Integer::from(2).pow(31_999) + Integer::from(1));
    let mut source = normalized.algebra.parse("1").unwrap();
    for exponent in 1..3999 {
        source.append_monomial(c.clone(), &[exponent]);
    }
    let bits = base_univariate::source_cost(&source).unwrap();
    assert!(bits > 64_000_000 && bits < 128_000_000);
    assert!(base_univariate::cost(&source).is_none());
    let report = b.report.base_univariate_basis.as_mut().unwrap();
    report.begin(&source, 0);
    report.source(&source, bits);
    let path = directory.0.join("text-decline.json");
    b.progress_path = Some(path.clone());
    // The real 32-MiB limit applies. This shared preparation runs before
    // either SQF or the ordinary fallback; no large native call is made.
    assert!(b.prepare_base_text(&source).unwrap().is_none());
    let saved: LocalReport = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(saved.active_phase, "base_univariate_fallback");
    assert_eq!(saved.cache.factor_misses, 0);
    let report = saved.base_univariate_basis.unwrap();
    assert_eq!(report.declines.source_text, 1);
    assert_eq!(report.static_declines, 1);
    assert_eq!(report.sources, 0);
    assert_eq!(report.decomposition_seconds, 0.0);
    assert_eq!(
        report.last_attempt.unwrap().decline,
        Some(BaseUnivariateDecline::SourceText)
    );
    for exponent in 3999..4002 {
        source.append_monomial(c.clone(), &[exponent]);
    }
    assert!(base_univariate::source_cost(&source).is_none());
    assert_eq!(
        base_univariate::source_decline(&source),
        BaseUnivariateDecline::CoefficientStorage
    );
}

#[test]
fn replayed_disk_blocks_still_pay_registry_cost_and_never_fall_back_on_resource_stop() {
    let directory = Directory::new();
    let mut problem = Problem {
        variables: vec!["x".into(), "z".into()],
        ..Default::default()
    };
    problem.solver.checkpoint_dir = Some(directory.0.clone());
    let normalized = normalize(&problem, &problem.variables).unwrap();
    let options = LocalOptions {
        base_univariate_basis: true,
        ..Default::default()
    };
    let source = normalized.algebra.parse("(x-1)*(x-2)").unwrap();
    let mut cold = builder(&problem, &normalized, &options);
    cold.guard_basis(std::slice::from_ref(&source)).unwrap();
    assert_eq!(
        cold.report
            .base_univariate_basis
            .as_ref()
            .unwrap()
            .proposal_cache
            .writes,
        1
    );
    drop(cold);
    let mut warm = builder(&problem, &normalized, &options);
    // The source alone fits; only charging its replayed blocks exceeds the
    // current registry cap. No large live registry is allocated in this test.
    warm.base_retained_bits =
        base_univariate::MAX_RETAINED_BITS - base_univariate::source_cost(&source).unwrap();
    let result = warm.guard_basis(&[source]);
    assert!(matches!(result, Err(Stop::Resource(_))));
    let report = warm.report.base_univariate_basis.as_ref().unwrap();
    assert_eq!(report.proposal_cache.hits, 1);
    assert_eq!(report.decomposition_seconds, 0.0);
    assert_eq!(report.static_declines, 0);
    assert_eq!(report.post_entry_resource_stops, 1);
    assert_eq!(report.sources, 0);
    assert_eq!(warm.operations.cache.report.cache.factor_misses, 0);
}
