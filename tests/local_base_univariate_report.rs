use symgcad::{
    Problem,
    algebra::{Algebra, Poly},
    local_projection::{self, BaseUnivariateReport, LocalEvidence, LocalOptions},
    output::Status,
};

fn bits(p: &Poly) -> u64 {
    p.coefficients
        .iter()
        .map(|c| c.numerator_ref().significant_bits() + c.denominator_ref().significant_bits())
        .sum()
}

#[test]
fn retention_matches_every_record_including_reused_blocks_and_has_no_proof_authority() {
    let mut p = Problem {
        variables: vec!["x".into(), "z".into()],
        split: vec!["z^2-(x-1)*(x-2)".into(), "z^2-2*(x-1)*(x-2)".into()],
        ..Default::default()
    };
    p.solver.engine = "local".into();
    p.solver.ordering = "given".into();
    p.solver.local_base_univariate_basis = true;
    p.limits.wall_time_secs = 20.0;
    let mut result = local_projection::solve_local(
        &p,
        &LocalOptions {
            base_univariate_basis: true,
            domain_pruning: false,
            positive_ray: false,
            fallback_to_global: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.solution.status, Status::CompleteGeneric);
    assert!(local_projection::verify_local(&result).unwrap().verified);
    let LocalEvidence::Local {
        base_univariate_basis: Some(basis),
        report: local_report,
        ..
    } = &result.evidence
    else {
        panic!("missing typed basis");
    };
    let a = Algebra::new(&result.solution.order).unwrap();
    let mut source_bits = 0;
    let mut block_bits = 0;
    let mut maximum_source_bits = 0;
    let mut maximum_source_terms = 0;
    let mut maximum_source_height = 0;
    let mut maximum_source_text = 0;
    let mut maximum_block_bits = 0;
    let mut references = 0;
    for source in &basis.sources {
        let q = a.parse(&source.polynomial).unwrap();
        let cost = bits(&q);
        source_bits += cost;
        maximum_source_bits = maximum_source_bits.max(cost);
        maximum_source_terms = maximum_source_terms.max(q.nterms());
        maximum_source_height = maximum_source_height.max(
            q.coefficients
                .iter()
                .map(|c| {
                    c.numerator_ref()
                        .significant_bits()
                        .max(c.denominator_ref().significant_bits())
                })
                .max()
                .unwrap(),
        );
        maximum_source_text = maximum_source_text.max(source.polynomial.len());
        let blocks: u64 = source
            .blocks
            .iter()
            .map(|b| bits(&a.parse(&result.solution.polynomials[b.polynomial]).unwrap()))
            .sum();
        block_bits += blocks;
        maximum_block_bits = maximum_block_bits.max(blocks);
        references += source.blocks.len();
    }
    let report = local_report.base_univariate_basis.as_ref().unwrap();
    assert!(
        references > report.blocks,
        "fixture must reuse a block across source records"
    );
    assert_eq!(report.retained_source_bits, source_bits);
    assert_eq!(report.retained_block_bits, block_bits);
    assert_eq!(report.retained_total_bits, source_bits + block_bits);
    assert_eq!(report.peak_source_bits, maximum_source_bits);
    assert_eq!(report.peak_source_terms, maximum_source_terms);
    assert_eq!(report.peak_source_height_bits, maximum_source_height);
    assert_eq!(report.peak_source_text_bytes, maximum_source_text);
    assert_eq!(report.peak_proposed_block_bits, maximum_block_bits);
    assert_eq!(report.peak_proposed_total_bits, source_bits + block_bits);
    assert_eq!(report.post_entry_resource_stops, 0);
    assert_eq!(report.post_entry_failures, 0);
    // Report edits cannot certify or invalidate the independently checked tree.
    if let LocalEvidence::Local { report, .. } = &mut result.evidence {
        report.base_univariate_basis = Some(BaseUnivariateReport::default());
    }
    assert!(local_projection::verify_local(&result).unwrap().verified);
}

#[test]
fn historical_report_without_telemetry_fields_still_decodes() {
    let report: BaseUnivariateReport = serde_json::from_value(serde_json::json!({
        "sources": 2, "blocks": 3, "hits": 4, "static_declines": 5,
        "decomposition_seconds": 1.5, "reconstruction_seconds": 0.5, "root_union_seconds": 2.0
    }))
    .unwrap();
    assert_eq!(report.sources, 2);
    assert_eq!(report.static_declines, 5);
    assert_eq!(report.retained_total_bits, 0);
    assert_eq!(report.declines.registry_storage, 0);
    assert!(report.last_attempt.is_none());
}
