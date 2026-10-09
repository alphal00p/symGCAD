use anyhow::bail;
use symgcad::{
    algebra::{Algebra, Poly},
    projection::{exact_resultant, exact_resultant_ducos, exact_resultant_with_statistics},
    resultant_interpolation::{Options, Stopped, degree_bound, try_resultant},
};

fn setup(names: &[&str]) -> Algebra {
    Algebra::new(&names.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
}
fn force() -> Options {
    Options {
        min_degree_product: 0,
        min_input_terms: 0,
        ..Default::default()
    }
}
fn compare(
    a: &Algebra,
    p: &str,
    q: &str,
    v: usize,
) -> (Poly, symgcad::resultant_interpolation::Statistics) {
    let p = a.parse(p).unwrap();
    let q = a.parse(q).unwrap();
    let result = try_resultant(&p, &q, v, &force(), || Ok(()))
        .unwrap()
        .unwrap();
    assert_eq!(result.0, exact_resultant_ducos(&p, &q, v).unwrap());
    assert_eq!(result.0.variables(), p.variables());
    assert_eq!(result.1.images, result.1.degree_bound + 1);
    result
}

#[test]
fn interpolation_preserves_signed_rational_scale_and_parameter_position() {
    let a = setup(&["unused", "z", "s", "trailing"]);
    for (p, q) in [
        ("(s*z^3+z^2+2*z+s^2)/2", "-(z^3+s*z+1)/3"),
        ("-(z^3+s*z+1)/3", "(s*z^3+z^2+2*z+s^2)/2"),
        ("z^4+s*z^2+1", "(s+1)*z^2+s^2*z+2"),
    ] {
        let (result, stats) = compare(&a, p, q, 1);
        assert_eq!(stats.parameter, 2);
        assert_eq!(result.degree(0), 0);
        assert_eq!(result.degree(1), 0);
        assert_eq!(result.degree(3), 0);
    }
}

#[test]
fn zero_first_image_zero_result_and_degree_drop_points_remain_exact() {
    let a = setup(&["s", "t"]);
    compare(&a, "t^2+s", "t^3+s*t+s", 1);
    assert!(compare(&a, "t^2+s", "(t^2+s)*(t+s)", 1).0.is_zero());
    let (_, stats) = compare(&a, "s*(s-1)*t^2+t+s", "(s+1)*t^3+t^2+s^2", 1);
    assert!(stats.skipped_degree_drops >= 3);
}

#[test]
fn total_degree_bound_is_tighter_and_interpolation_matches_independent_ducos() {
    let a = setup(&["s", "t"]);
    let p = a.parse("s^3+s^2*t+s*t^2+t^3+1").unwrap();
    let q = a.parse("s^2+t^2+1").unwrap();
    assert_eq!(degree_bound(&p, &q, 1, 0).unwrap(), 6);
    compare(&a, "s^3+s^2*t+s*t^2+t^3+1", "s^2+t^2+1", 1);
}

#[test]
fn caps_decline_before_work_and_dynamic_resource_or_cancellation_errors_propagate() {
    let a = setup(&["s", "t", "u"]);
    let p = a.parse("s*t^3+t+1").unwrap();
    let q = a.parse("t^3+s+1").unwrap();
    for options in [
        Options {
            max_degree_bound: 0,
            ..force()
        },
        Options {
            max_input_terms: 1,
            ..force()
        },
        Options {
            max_input_coefficient_bits: 0,
            ..force()
        },
        Options {
            max_elimination_degree: 2,
            ..force()
        },
    ] {
        assert!(
            try_resultant(&p, &q, 1, &options, || Ok(()))
                .unwrap()
                .is_none()
        );
    }
    assert!(
        try_resultant(&p, &a.parse("t^3+s+u").unwrap(), 1, &force(), || Ok(()))
            .unwrap()
            .is_none()
    );
    assert!(
        try_resultant(
            &a.parse("t^3+1").unwrap(),
            &a.parse("t^2+2").unwrap(),
            1,
            &force(),
            || Ok(())
        )
        .unwrap()
        .is_none()
    );
    let error = try_resultant(
        &p,
        &q,
        1,
        &Options {
            max_image_bits: 0,
            ..force()
        },
        || Ok(()),
    )
    .unwrap_err();
    assert!(error.downcast_ref::<Stopped>().is_some());
    let mut calls = 0;
    let error = try_resultant(&p, &q, 1, &force(), || {
        calls += 1;
        if calls >= 3 {
            bail!("cancelled")
        }
        Ok(())
    })
    .unwrap_err();
    assert!(error.downcast_ref::<Stopped>().is_some());
    assert_eq!(calls, 3);
}

#[test]
fn production_dispatch_is_conservative_and_exact_in_both_operand_orders() {
    let a = setup(&["s", "t"]);
    let p = a.parse("(t+s+1)^8+s*t+1").unwrap();
    let q = a.parse("(t+2*s+3)^8+t^2+s").unwrap();
    let (result, stats) = exact_resultant_with_statistics(&p, &q, 1).unwrap();
    assert!(stats.is_some());
    assert_eq!(result, exact_resultant_ducos(&p, &q, 1).unwrap());
    assert_eq!(
        exact_resultant(&q, &p, 1).unwrap(),
        exact_resultant_ducos(&q, &p, 1).unwrap()
    );
    let small = a.parse("t^2+s").unwrap();
    assert!(
        exact_resultant_with_statistics(&small, &q, 1)
            .unwrap()
            .1
            .is_none()
    );
}
