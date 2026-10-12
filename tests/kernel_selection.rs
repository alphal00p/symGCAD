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
        let assignment = assignment_degree_bound(&p, &q, 1, 0).unwrap();
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
    assert_eq!(assignment_degree_bound(&p, &q, 1, 0).unwrap(), 0);
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
    assert!(try_resultant(&p, &q, 1, &options, || Ok(())).unwrap().is_none());
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
