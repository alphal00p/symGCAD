use symgcad::{
    algebra::{Algebra, canonical},
    optimized_projection::{OptimizedProjectionOptions, ProjectionCache},
    projection::{discriminant, discriminant_with_binary_statistics},
};

fn algebra(names: &[&str]) -> Algebra {
    Algebra::new(&names.iter().map(|x| x.to_string()).collect::<Vec<_>>()).unwrap()
}

#[test]
fn signed_binomial_discriminants_preserve_rational_scale_and_leading_coefficients() {
    // Disc(a*x^n+c)=(-1)^(n(n-1)/2)n^n*a^(n-1)*c^(n-1).
    // These have interpolation bounds >1024 but fit the unchanged bit caps.
    for (names, x) in [(vec!["x", "s", "unused"], 0), (vec!["unused", "s", "x"], 2)] {
        let a = algebra(&names);
        for (n, k, sign) in [(9, 130, 1), (10, 120, -1)] {
            let p = a.parse(&format!("-2/3*((s+2)*x^{n}+(s+1)^{k})")).unwrap();
            let expected = a
                .parse(&format!(
                    "({sign})*{n}^{n}*(-2/3)^{}*(s+2)^{}*(s+1)^{}",
                    2 * n - 2,
                    n - 1,
                    k * (n - 1)
                ))
                .unwrap();
            let (actual, stats) = discriminant_with_binary_statistics(&p, x).unwrap();
            let stats = stats.expect("the measured default discriminant envelope must be used");
            assert_eq!(stats.plan.parameters, vec![1]);
            assert!(stats.plan.degree_bounds[1] > 1024);
            assert!(stats.plan.output_bit_bound <= 96_000_000);
            assert_eq!(actual, expected);
            assert_eq!(actual.variables(), p.variables());
            assert_eq!(actual.degree(x), 0);
            // The parameter-dependent leading coefficient may vanish later.
            // The polynomial identity remains valid at that degree drop.
            assert!(
                actual
                    .replace(1, &symbolica::domains::rational::Rational::from(-2))
                    .is_zero()
            );
        }
    }
}

#[test]
fn repeated_factor_gives_exact_zero_without_division_artifacts() {
    let a = algebra(&["s", "x"]);
    let p = a.parse("(x^5+(s+1)^70)^2").unwrap();
    let (actual, stats) = discriminant_with_binary_statistics(&p, 1).unwrap();
    assert!(stats.is_some());
    assert!(actual.is_zero());
}

#[test]
fn two_parameter_discriminants_retain_signed_scale_and_degree_drop_loci() {
    let a = algebra(&["unused", "s", "x", "t", "tail"]);
    for (n, sign) in [(9, 1), (10, -1)] {
        let p = a
            .parse(&format!("-2/3*((s-t)*x^{n}+(s+t+1)^10)"))
            .unwrap();
        let expected = a
            .parse(&format!(
                "({sign})*{n}^{n}*(-2/3)^{}*(s-t)^{}*(s+t+1)^{}",
                2 * n - 2,
                n - 1,
                10 * (n - 1)
            ))
            .unwrap();
        let (actual, stats) = discriminant_with_binary_statistics(&p, 2).unwrap();
        let stats = stats.expect("two-parameter discriminant must reach the bounded kernel");
        assert_eq!(stats.plan.parameters, vec![1, 3]);
        assert!(stats.plan.output_bit_bound <= 96_000_000);
        assert_eq!(actual, expected);
        assert_eq!(actual, discriminant(&p, 2).unwrap());
        assert_eq!(actual.variables(), p.variables());
        // The formal identity retains both the vanishing leading coefficient
        // and the repeated-root locus; neither can be cancelled from the result.
        assert!(actual.replace(1, &1.into()).replace(3, &1.into()).is_zero());
        assert!(actual.replace(1, &1.into()).replace(3, &(-2).into()).is_zero());
    }
}

#[test]
fn two_parameter_repeated_factor_keeps_exact_zero_and_full_map() {
    let a = algebra(&["s", "x", "t", "unused"]);
    let p = a.parse("(x^5+(s+t+1)^10)^2").unwrap();
    let (actual, stats) = discriminant_with_binary_statistics(&p, 1).unwrap();
    assert_eq!(stats.unwrap().plan.parameters, vec![0, 2]);
    assert!(actual.is_zero());
    assert_eq!(actual.variables(), p.variables());
}

#[test]
fn three_parameter_discriminants_keep_the_ordinary_backend() {
    let a = algebra(&["s", "t", "u", "x"]);
    let p = a
        .parse("(s+1)*(t+1)*(u+1)*(1+x+x^2+x^3+x^4+x^5+x^6+x^7+x^8+x^9)")
        .unwrap();
    let (actual, stats) = discriminant_with_binary_statistics(&p, 3).unwrap();
    assert!(stats.is_none());
    assert_eq!(actual, a.parse("10^8*(s+1)^16*(t+1)^16*(u+1)^16").unwrap());
}

#[test]
fn ordinary_repeated_factor_discriminant_restores_mapless_native_zero() {
    let a = algebra(&["unused", "s", "x", "tail"]);
    // Too few terms for either optional strategy: this reaches native Ducos,
    // which returns a ring zero without a variable map for a shared factor.
    let p = a.parse("(x^3+s)^2").unwrap();
    let ordinary = discriminant(&p, 2).unwrap();
    assert!(ordinary.is_zero());
    assert_eq!(ordinary.variables(), p.variables());
    let (opted, stats) = discriminant_with_binary_statistics(&p, 2).unwrap();
    assert!(stats.is_none());
    assert!(opted.is_zero());
    assert_eq!(opted.variables(), p.variables());
}

#[test]
fn low_degree_interpolation_and_other_parameter_counts_keep_their_backends() {
    let a = algebra(&["s", "x", "t"]);
    for expression in [
        "0",
        "3",
        "s*x+1",
        "s*x^2+x+1",
        "x^4-s",
        "(x+s+1)^9+s*x+1",
        "x^5+s*x+t",
    ] {
        let p = a.parse(expression).unwrap();
        let (actual, stats) = discriminant_with_binary_statistics(&p, 1).unwrap();
        assert!(stats.is_none(), "unexpected segmentation for {expression}");
        assert_eq!(actual, discriminant(&p, 1).unwrap());
    }
    assert!(discriminant_with_binary_statistics(&a.parse("x^9+s").unwrap(), 3).is_err());
}

#[test]
fn discriminant_cache_counts_kernel_separately_and_reuses_exact_values() {
    let a = algebra(&["s", "x"]);
    let p = a.parse("x^9+(s+1)^130").unwrap();
    let expected = a.parse("9^9*(s+1)^1040").unwrap();
    let mut cache = ProjectionCache::new(
        &a,
        &OptimizedProjectionOptions {
            binary_resultants: true,
            ..Default::default()
        },
    );
    assert_eq!(cache.discriminant(&p, 1).unwrap(), canonical(&expected));
    assert_eq!(cache.report.cache.segmented_discriminants, 1);
    assert_eq!(cache.report.cache.segmented_resultants, 0);
    assert!(cache.report.cache.maximum_segmented_output_bits > 0);
    assert_eq!(cache.discriminant(&p, 1).unwrap(), canonical(&expected));
    assert_eq!(cache.report.cache.discriminant_hits, 1);
    assert_eq!(cache.report.cache.segmented_discriminants, 1);
}

#[test]
fn opt_in_local_proof_replay_still_rejects_missing_and_changed_cells() {
    let mut problem = symgcad::Problem {
        variables: vec!["s".into(), "x".into()],
        constraints: vec!["s>0".into()],
        split: vec!["x^5-s".into()],
        ..Default::default()
    };
    problem.solver.engine = "local".into();
    problem.solver.binary_resultants = true;
    problem.limits.wall_time_secs = 10.0;
    let result = symgcad::solver::solve(&problem).unwrap();
    assert_eq!(result.status, symgcad::output::Status::CompleteGeneric);
    assert!(symgcad::verify::verify(&result).unwrap().verified);
    let mut missing = result.clone();
    missing.cells.pop();
    assert!(symgcad::verify::verify(&missing).is_err());
    let mut changed = result;
    changed.cells[0].signs[0] *= -1;
    assert!(symgcad::verify::verify(&changed).is_err());
}
