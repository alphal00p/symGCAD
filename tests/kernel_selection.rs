//! Size-derived interpolation admission, the exact Sylvester assignment
//! degree bound, and the binary-segmentation solver default.
use symgcad::{
    Problem,
    algebra::{Algebra, Poly},
    projection::{exact_resultant_ducos, exact_resultant_with_statistics},
    resultant_interpolation::{
        Options, assignment_degree_bound, degree_bound, plan, try_resultant,
    },
};

fn setup(names: &[&str]) -> Algebra {
    Algebra::new(&names.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
}

#[test]
fn assignment_bound_is_proved_tighter_and_handles_structural_zeros() {
    let a = setup(&["s", "x"]);
    let mut strictly_tighter = 0;
    for (p, q) in [
        // Sparse high-degree constant terms: the closed forms overcount.
        ("x^6+s^40", "x^5+s*x+s^3"),
        ("s*x^4+x^3+s^9", "x^3+s^7*x+1"),
        ("(x+s+1)^7+s*x", "(x+2*s+3)^5+x^2+s"),
        ("x^3*s^5+x^2+s^2*x+1", "x^2+s^9*x+s"),
        ("-2/3*((s+2)*x^9+(s+1)^30)", "x^8+s^4"),
    ] {
        let p = a.parse(p).unwrap();
        let q = a.parse(q).unwrap();
        let closed = degree_bound(&p, &q, 1, 0).unwrap();
        let assignment = assignment_degree_bound(&p, &q, 1, 0).unwrap().unwrap();
        let actual = exact_resultant_ducos(&p, &q, 1).unwrap().degree(0) as usize;
        assert!(actual <= assignment, "{actual} > {assignment}");
        assert!(assignment <= closed);
        strictly_tighter += usize::from(assignment < closed);
        // Interpolation with the tighter bound stays exact.
        let forced = Options {
            min_degree_product: 0,
            min_input_terms: 0,
            ..Default::default()
        };
        let (result, stats) = try_resultant(&p, &q, 1, &forced, || Ok(()))
            .unwrap()
            .unwrap();
        assert_eq!(stats.degree_bound, assignment);
        assert_eq!(stats.closed_form_bound, closed);
        assert_eq!(result, exact_resultant_ducos(&p, &q, 1).unwrap());
    }
    assert!(strictly_tighter >= 1);
    // A shared root at x=0 empties the Sylvester matrix's last column.
    let p = a.parse("x^3+s*x").unwrap();
    let q = a.parse("x^2+s^5*x").unwrap();
    assert_eq!(assignment_degree_bound(&p, &q, 1, 0).unwrap(), Some(0));
    assert!(exact_resultant_ducos(&p, &q, 1).unwrap().is_zero());
}

#[test]
fn size_admitted_interpolation_exceeds_the_historical_cap_exactly() {
    let a = setup(&["x", "s", "unused"]);
    // Bound 8*130+9 = 1049 > 1024 with small images.
    let p = a.parse("-2/3*((s+2)*x^9+(s+1)^130)").unwrap();
    let q = p.derivative(0);
    let legacy = plan(&p, &q, 0, &Options::legacy()).unwrap();
    assert!(legacy.is_none());
    let admitted = plan(&p, &q, 0, &Options::default()).unwrap().unwrap();
    assert!(admitted.size_admitted);
    assert!(admitted.degree_bound > 1024);
    assert!(admitted.image_bit_bound <= Options::default().max_image_bits);
    let (result, stats) = exact_resultant_with_statistics(&p, &q, 0).unwrap();
    let stats = stats.expect("production dispatch uses size-admitted interpolation");
    assert!(stats.size_admitted);
    assert_eq!(stats.images, stats.degree_bound + 1);
    assert!(stats.maximum_image_bits <= admitted.image_bit_bound);
    assert_eq!(result, exact_resultant_ducos(&p, &q, 0).unwrap());
    assert_eq!(result.variables(), p.variables());
}

#[test]
fn image_size_admission_declines_before_any_dynamic_stop() {
    let a = setup(&["s", "x"]);
    // Bound about 3200, with each image's a-priori bit bound above 65536:
    // the plan declines statically, so no Stopped error can occur later.
    let p = a.parse("x^9+2^4000*(s+1)^400").unwrap();
    let q = p.derivative(1);
    let options = Options {
        max_input_coefficient_bits: 8192,
        ..Default::default()
    };
    assert!(plan(&p, &q, 1, &options).unwrap().is_none());
    assert!(
        try_resultant(&p, &q, 1, &options, || Ok(()))
            .unwrap()
            .is_none()
    );
    // The same operand under the historical bound-only cap also declines.
    assert!(plan(&p, &q, 1, &Options::legacy()).unwrap().is_none());
    // Lower degrees within the unconditional bound keep the historical rule.
    let small = a.parse("x^9+2^100*(s+1)^20").unwrap();
    let forced = Options {
        min_degree_product: 0,
        min_input_terms: 0,
        ..Default::default()
    };
    let small_plan = plan(&small, &small.derivative(1), 1, &forced)
        .unwrap()
        .unwrap();
    assert!(!small_plan.size_admitted);
}

#[test]
fn integer_images_preserve_rational_scale_signs_and_degree_drops() {
    let a = setup(&["s", "t"]);
    let forced = Options {
        min_degree_product: 0,
        min_input_terms: 0,
        ..Default::default()
    };
    for (p, q) in [
        ("(2/3)*s*(s-1)*t^3+t/5+s", "(s+1)*t^3/7+t^2-s^2"),
        ("-(t^4+s*t^2+1)/11", "(s^2-4)*t^2+s*t+3/2"),
        ("t^2+s", "(t^2+s)*(t+s)"),
    ] {
        let p: Poly = a.parse(p).unwrap();
        let q: Poly = a.parse(q).unwrap();
        let (result, stats) = try_resultant(&p, &q, 1, &forced, || Ok(()))
            .unwrap()
            .unwrap();
        assert_eq!(result, exact_resultant_ducos(&p, &q, 1).unwrap());
        assert_eq!(stats.images, stats.degree_bound + 1);
    }
}

#[test]
fn binary_segmentation_is_the_solver_default_with_a_serialized_opt_out() {
    let default = Problem::default();
    assert!(default.solver.binary_resultants);
    let mut opted_out = Problem::default();
    opted_out.solver.binary_resultants = false;
    // Worker hand-off serializes the problem; the opt-out must survive it.
    let json = serde_json::to_string(&opted_out).unwrap();
    let restored: Problem = serde_json::from_str(&json).unwrap();
    assert!(!restored.solver.binary_resultants);
    let toml_text = toml::to_string(&opted_out).unwrap();
    assert!(toml_text.contains("binary_resultants = false"));
    let restored: Problem = toml::from_str(&toml_text).unwrap();
    assert!(!restored.solver.binary_resultants);
    let restored: Problem =
        serde_json::from_str(&serde_json::to_string(&default).unwrap()).unwrap();
    assert!(restored.solver.binary_resultants);
}

/// The reviewer's pair: P0 = x^8-3x^7+3x and Q0 = x^8+x^7+3x share the root
/// x = 0 at s = 0, so the very first interpolation image vanishes. `top`
/// adds s^71 (and 2*s^71), lifting the closed-form bound above 1024 while the
/// assignment bound stays below it.
fn shared_root_pair(a: &Algebra, top: bool) -> (Poly, Poly, String, String) {
    let (mut r1, mut r2) = (Vec::new(), Vec::new());
    for i in 0..8 {
        for j in 0..4 {
            r1.push(format!("{}*x^{i}*s^{j}", 1 + (i + j) % 3));
            r2.push(format!("{}*x^{i}*s^{j}", 2 + (i * j) % 3));
        }
    }
    let tail = |k: u32| {
        if top {
            format!("+{k}*s^71")
        } else {
            String::new()
        }
    };
    let p = format!("x^8-3*x^7+3*x+s*({}){}", r1.join("+"), tail(1));
    let q = format!("x^8+x^7+3*x+s*({}){}", r2.join("+"), tail(2));
    (a.parse(&p).unwrap(), a.parse(&q).unwrap(), p, q)
}

#[test]
fn vanishing_images_are_decided_exactly_instead_of_crashing_ducos() {
    let a = setup(&["s", "x"]);
    for top in [false, true] {
        let (p, q, _, _) = shared_root_pair(&a, top);
        let new = plan(&p, &q, 1, &Options::default()).unwrap().unwrap();
        if top {
            // Outside the historical envelope: the fixed-cap rule declined
            // and only the a-priori size check admits it.
            assert!(plan(&p, &q, 1, &Options::legacy()).unwrap().is_none());
            assert!(new.closed_form_bound > 1024 && new.degree_bound <= 1024);
            assert!(new.size_admitted);
        } else {
            assert!(!new.size_admitted);
        }
        let (result, stats) = exact_resultant_with_statistics(&p, &q, 1).unwrap();
        let stats = stats.expect("interpolation applies");
        assert!(stats.spot_checked);
        // Full Ducos is safe here: Res(P,Q) is not identically zero, so its
        // subresultant chain has no vanishing member.
        assert_eq!(result, exact_resultant_ducos(&p, &q, 1).unwrap());
        assert!(
            result
                .replace(0, &symbolica::prelude::Rational::from(0))
                .is_zero()
        );
    }
}

#[test]
fn assignment_tightened_plans_must_pass_the_image_size_check() {
    let a = setup(&["s", "x"]);
    // The reviewer's operands: the closed forms exceed 1024, the assignment
    // bound does not, and every image may need about 150,000 bits.
    let mut pt = vec!["x^16".to_string(), "2^4090*s^60".into(), "1".into()];
    let mut qt = vec!["x^16".to_string(), "2^4090*s^60".into(), "2".into()];
    for i in 1..16 {
        pt.push(format!("{}*x^{i}*s", 1 + i % 3));
        pt.push(format!("{}*x^{i}", 2 + i % 5));
        qt.push(format!("{}*x^{i}*s", 3 + i % 2));
        qt.push(format!("{}*x^{i}", 1 + i % 4));
    }
    let p = a.parse(&pt.join("+")).unwrap();
    let q = a.parse(&qt.join("+")).unwrap();
    let forced = Options {
        min_degree_product: 0,
        min_input_terms: 0,
        ..Default::default()
    };
    assert!(plan(&p, &q, 1, &Options::legacy()).unwrap().is_none());
    // Without its size check, the tightened bound would be admitted.
    let unchecked = Options {
        max_image_bits: u64::MAX,
        max_total_image_bits: u64::MAX,
        ..forced.clone()
    };
    let would = plan(&p, &q, 1, &unchecked).unwrap().unwrap();
    assert!(would.degree_bound <= 1024 && would.closed_form_bound > 1024);
    assert!(would.image_bit_bound > Options::default().max_image_bits);
    // With it, the plan declines statically (ordinary Ducos remains), so no
    // dynamic image stop can fire.
    assert!(plan(&p, &q, 1, &forced).unwrap().is_none());
    assert!(
        try_resultant(&p, &q, 1, &forced, || Ok(()))
            .unwrap()
            .is_none()
    );
    // Inside the historical envelope the fixed-cap rule is unchanged.
    let small = a.parse("x^9+2^100*(s+1)^20").unwrap();
    let small_plan = plan(&small, &small.derivative(1), 1, &forced)
        .unwrap()
        .unwrap();
    let legacy_plan = plan(
        &small,
        &small.derivative(1),
        1,
        &Options {
            min_degree_product: 0,
            min_input_terms: 0,
            ..Options::legacy()
        },
    )
    .unwrap()
    .unwrap();
    assert!(!small_plan.size_admitted && !legacy_plan.size_admitted);
}

#[test]
fn solve_and_verify_through_interpolation_with_a_vanishing_image() {
    // Base and the first branch build both crashed here (the image at s = 0
    // is zero). The size-admitted variant with s^71 is the same path with a
    // degree-568 resultant; it is too slow for this debug suite and is run on
    // the release CLI instead (artifacts/research/.../algebra-fixes/fix).
    // Kernel-level size admission is covered by the tests above.
    let a = setup(&["s", "x"]);
    let (_, _, p, q) = shared_root_pair(&a, false);
    let mut problem = Problem {
        variables: vec!["s".into(), "x".into()],
        constraints: vec![
            "s > 0".into(),
            "x > 0".into(),
            format!("{p} > 0"),
            format!("{q} > 0"),
        ],
        ..Default::default()
    };
    problem.solver.engine = "optimized".into();
    problem.solver.ordering = "given".into();
    problem.solver.order = vec!["s".into(), "x".into()];
    problem.limits.wall_time_secs = 300.0;
    let result = symgcad::solver::solve(&problem).unwrap();
    assert_eq!(result.status, symgcad::output::Status::CompleteGeneric);
    let report = result.statistics.optimized_projection.as_ref().unwrap();
    assert!(report.cache.interpolated_resultants >= 1);
    assert!(symgcad::verify::verify(&result).unwrap().verified);
}

#[test]
fn optimized_prepare_projection_accepts_any_explicit_order() {
    let mut problem = Problem {
        variables: vec!["x".into(), "y".into()],
        constraints: vec!["x > 0".into(), "y > 0".into()],
        split: vec!["y^2+y-x".into(), "x^3-y*x+1".into()],
        ..Default::default()
    };
    problem.solver.engine = "optimized".into();
    problem.solver.ordering = "given".into();
    problem.solver.order = vec!["x".into(), "y".into()];
    let selected = symgcad::solver::project_problem(&problem).unwrap();
    let (_, same, _) =
        symgcad::solver::prepare_projection(&problem, &["x".into(), "y".into()]).unwrap();
    assert_eq!(same.levels, selected.projection.levels);
    // Another order: the projection solve would compute had it chosen it.
    let (_, other, report) =
        symgcad::solver::prepare_projection(&problem, &["y".into(), "x".into()]).unwrap();
    assert!(report.is_some());
    let mut swapped = problem.clone();
    swapped.solver.order = vec!["y".into(), "x".into()];
    let expected = symgcad::solver::project_problem(&swapped).unwrap();
    assert_eq!(other.levels, expected.projection.levels);
}

#[test]
fn full_ducos_returns_zero_for_shared_factors_instead_of_panicking() {
    let a = setup(&["s", "x"]);
    // Symbolica 3.0.1 panics on the univariate cases (the review's minimal
    // examples) and on some multivariate shared-factor pairs.
    for (p, q) in [
        ("3*x^5", "-3*x-x^3"),
        ("3000000021*x+x^3", "3*x^3"),
        ("3*x^4+3*x^7", "3*x+3*x^7"),
        ("(x^2+s)*(x^3+x+s)", "(x^2+s)*(x^2-7)"),
        ("x^5*s+x^3*(s^2+1)", "x^3*(s+3)+x*s^4"),
        ("(s*x^2+1)*(x^3+s^2*x+1)", "(s*x^2+1)*(x^3-s)"),
    ] {
        let p = a.parse(p).unwrap();
        let q = a.parse(q).unwrap();
        assert!(exact_resultant_ducos(&p, &q, 1).unwrap().is_zero());
        assert!(exact_resultant_ducos(&q, &p, 1).unwrap().is_zero());
    }
    // Coprime operands, including ones whose specialization at the first
    // test points shares a factor, keep their nonzero resultant.
    let forced = Options {
        min_degree_product: 0,
        min_input_terms: 0,
        ..Default::default()
    };
    for (p, q) in [
        ("x^5+s*x+2", "x^3+3*x+s^2"),
        ("(x^2-2)*(x+s-2)", "(x^2-2+s-2)*(x-1)"),
    ] {
        let p = a.parse(p).unwrap();
        let q = a.parse(q).unwrap();
        let r = exact_resultant_ducos(&p, &q, 1).unwrap();
        assert!(!r.is_zero());
        assert_eq!(
            try_resultant(&p, &q, 1, &forced, || Ok(()))
                .unwrap()
                .unwrap()
                .0,
            r
        );
    }
}
