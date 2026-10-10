use symbolica::prelude::*;
use symgcad::{
    algebra::{Algebra, rational},
    roots::{
        FiberRoot, isolate_union, refine_once, roots_in_open_interval, sector_sample,
        simplest_between,
    },
};

fn q(text: &str) -> Rational {
    rational(text).unwrap()
}

fn polynomial(expression: &str) -> UnivariatePolynomial<Q> {
    let algebra = Algebra::new(&["x".into()]).unwrap();
    algebra
        .specialize_univariate(&algebra.parse(expression).unwrap(), 0, &[])
        .unwrap()
}

#[test]
fn observed_root_union_preserves_shared_roots_and_propagates_cancellation() {
    let input = vec![
        (0, polynomial("(x^2-2)^2")),
        (1, polynomial("(x^2-2)*(x-3)")),
    ];
    let plain = isolate_union(input.clone()).unwrap();
    let mut events = Vec::new();
    let observed = symgcad::roots::isolate_union_observed(input.clone(), |phase, id, _, _| {
        events.push((phase.to_owned(), id));
        Ok(())
    })
    .unwrap();
    assert_eq!(plain.len(), 3);
    assert_eq!(observed.len(), plain.len());
    for (a, b) in plain.iter().zip(&observed) {
        assert_eq!(
            (a.polynomial, a.index, &a.interval, &a.fiber),
            (b.polynomial, b.index, &b.interval, &b.fiber)
        );
    }
    assert!(events.iter().any(|(phase, _)| phase == "separate_roots"));
    let stopped = symgcad::roots::isolate_union_observed(input, |_, _, _, _| {
        anyhow::bail!("diagnostic cancelled")
    });
    assert!(
        stopped
            .unwrap_err()
            .to_string()
            .contains("diagnostic cancelled")
    );
}

#[test]
fn root_queue_merges_overlapping_chains_and_keeps_the_lowest_source_id() {
    let input = (1..=24)
        .rev()
        .map(|i| {
            (
                i as usize,
                polynomial(&format!("(x^2-{i})*(x^2-{})", i + 1)),
            )
        })
        .collect::<Vec<_>>();
    let roots = isolate_union(input).unwrap();
    assert_eq!(roots.len(), 50);
    assert!(
        roots
            .windows(2)
            .all(|pair| pair[0].interval.1 < pair[1].interval.0)
    );
    for root in &roots {
        assert_eq!(
            roots_in_open_interval(&root.fiber, &q("-100"), &root.interval.0)
                + usize::from(
                    root.fiber.evaluate(&root.interval.0).is_zero()
                        && root.interval.0 != root.interval.1
                ),
            root.index
        );
        if root.interval.0 == root.interval.1 {
            assert!(root.fiber.evaluate(&root.interval.0).is_zero());
        } else {
            assert_eq!(
                roots_in_open_interval(&root.fiber, &root.interval.0, &root.interval.1),
                1
            );
        }
    }
    let exact_one = roots
        .iter()
        .find(|root| root.interval.0 <= q("1") && q("1") <= root.interval.1)
        .unwrap();
    assert_eq!(exact_one.polynomial, 1);
    let exact_four = roots
        .iter()
        .find(|root| root.interval.0 <= q("4") && q("4") <= root.interval.1)
        .unwrap();
    assert_eq!(exact_four.polynomial, 15);
}

#[test]
fn root_queue_separates_very_close_irrational_roots_without_merging() {
    let input = vec![
        (9, polynomial("x^2-2")),
        (3, polynomial("x^2-2-1/1000000000000000000000000000000")),
        (1, polynomial("(x^2-2)^3")),
    ];
    let roots = isolate_union(input).unwrap();
    assert_eq!(roots.len(), 4);
    assert_eq!(
        roots.iter().map(|r| r.polynomial).collect::<Vec<_>>(),
        vec![3, 1, 1, 3]
    );
    assert!(
        roots
            .windows(2)
            .all(|pair| pair[0].interval.1 < pair[1].interval.0)
    );
}

#[test]
fn collision_memo_uses_input_identity_even_when_external_ids_repeat() {
    let roots = isolate_union(vec![
        (7, polynomial("x^2-2")),
        (7, polynomial("x^2-2-1/1000000000000000000000000000000")),
        (7, polynomial("(x^2-2)^3")),
        (7, polynomial("(x^2-2-1/1000000000000000000000000000000)^2")),
    ])
    .unwrap();
    assert_eq!(roots.len(), 4);
    assert!(
        roots
            .windows(2)
            .all(|pair| pair[0].interval.1 < pair[1].interval.0)
    );
    for root in &roots {
        assert_eq!(
            roots_in_open_interval(&root.fiber, &root.interval.0, &root.interval.1),
            1
        );
    }
}

#[test]
fn finite_bisection_delay_still_merges_many_coincident_branches() {
    let roots = isolate_union(
        (0..40)
            .rev()
            .map(|i| (i, polynomial(&format!("(x^2-2)*(x^2+{})", i + 1))))
            .collect(),
    )
    .unwrap();
    assert_eq!(roots.len(), 2);
    assert!(roots.iter().all(|root| root.polynomial == 0));
    assert!(roots[0].interval.1 < roots[1].interval.0);
}

#[test]
fn sturm_excludes_endpoint_roots_and_counts_repeated_roots_once() {
    let p = polynomial("x^2*(x-1)^2*(x+1)^3");
    for (lo, hi, count) in [
        ("-2", "2", 3),
        ("-1", "1", 1),
        ("-1", "0", 0),
        ("0", "1", 0),
        ("-2", "0", 1),
        ("0", "2", 1),
        ("0", "0", 0),
        ("2", "-2", 0),
    ] {
        assert_eq!(
            roots_in_open_interval(&p, &q(lo), &q(hi)),
            count,
            "({lo},{hi})"
        );
    }
}

#[test]
fn sturm_with_large_mixed_coefficients_matches_independent_real_isolation() {
    let base = polynomial("x^12-2*x^11+3*x^9-1001*x^7+17003*x^4-30001*x+11");
    let intervals = base.isolate_real_root_intervals();
    let positive = intervals.iter().filter(|(lo, _, _)| *lo >= q("0")).count();
    let negative = intervals.iter().filter(|(_, hi, _)| *hi <= q("0")).count();
    assert_eq!(positive + negative, intervals.len());
    // A strict Cauchy bound; denominators and negative scaling must not alter
    // the one-sided signs of a primitive Sturm remainder sequence.
    for scale in [
        "1",
        "-1",
        "137/10000000000000000000000001",
        "-29/100000000000000000000003",
    ] {
        let p = base.clone().mul_coeff(&q(scale));
        assert_eq!(roots_in_open_interval(&p, &q("0"), &q("30003")), positive);
        assert_eq!(roots_in_open_interval(&p, &q("-30003"), &q("0")), negative);
    }
}

#[test]
fn bisection_uses_one_sided_signs_at_other_roots() {
    let mut root = FiberRoot {
        polynomial: 0,
        index: 1,
        interval: (q("0"), q("3")),
        fiber: polynomial("x*(x-1)*(x-3)").into(),
    };
    for _ in 0..12 {
        refine_once(&mut root);
        assert!(root.interval.0 <= q("1") && root.interval.1 >= q("1"));
        if root.interval.0 != root.interval.1 {
            assert_eq!(
                roots_in_open_interval(&root.fiber, &root.interval.0, &root.interval.1),
                1
            );
        }
    }
}

#[test]
fn simple_real_roots_refine_correctly_with_repeated_nonreal_factors() {
    let p = polynomial("(x^2+1)^3*(x^2-2)");
    let mut roots = isolate_union(vec![(0, p.clone())]).unwrap();
    assert_eq!(roots.len(), 2);
    for root in &mut roots {
        assert_eq!(root.fiber.as_ref(), &p);
        for _ in 0..16 {
            refine_once(root);
            assert_eq!(
                roots_in_open_interval(&p, &root.interval.0, &root.interval.1),
                1
            );
        }
        assert!(root.interval.1.clone() - root.interval.0.clone() < q("1/1000"));
    }
}

#[test]
fn root_union_deduplicates_coincident_irrational_roots() {
    let roots = isolate_union(vec![
        (8, polynomial("(x^2-2)^2*(x-5)")),
        (2, polynomial("x^2-2")),
    ])
    .unwrap();
    assert_eq!(roots.len(), 3);
    assert_eq!(
        roots
            .iter()
            .map(|r| (r.polynomial, r.index))
            .collect::<Vec<_>>(),
        vec![(2, 0), (2, 1), (8, 2)]
    );
    assert!(roots.windows(2).all(|w| w[0].interval.1 < w[1].interval.0));
    assert!(roots[2].interval.0 <= q("5") && roots[2].interval.1 >= q("5"));
}

#[test]
fn close_distinct_irrational_roots_are_separated() {
    let roots = isolate_union(vec![
        (0, polynomial("x^2-2")),
        (1, polynomial("(x-1/1000000000000)^2-2")),
    ])
    .unwrap();
    assert_eq!(roots.len(), 4);
    assert_eq!(
        roots.iter().map(|r| r.polynomial).collect::<Vec<_>>(),
        vec![0, 1, 0, 1]
    );
    assert!(roots.windows(2).all(|w| w[0].interval.1 < w[1].interval.0));
    for root in roots {
        assert_eq!(
            roots_in_open_interval(&root.fiber, &root.interval.0, &root.interval.1),
            1
        );
    }
}

#[test]
fn root_union_handles_exact_zero_repetition_and_constants() {
    let roots = isolate_union(vec![
        (0, polynomial("x^8*(x-1)^4")),
        (1, polynomial("x^2")),
        (2, polynomial("7")),
    ])
    .unwrap();
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0].interval, (q("0"), q("0")));
    assert_eq!(roots[0].polynomial, 0);
    assert_eq!(roots[1].index, 1);
    assert!(isolate_union(vec![(0, polynomial("0"))]).is_err());
}

#[test]
fn rational_roots_at_vas_interval_endpoints_remain_exact() {
    let p = polynomial("4*x^2-8*x");
    let raw = p.isolate_real_root_intervals();
    let roots = isolate_union(vec![(0, p.clone())]).unwrap();
    assert_eq!(roots.len(), 2, "raw: {raw:?}");
    for root in roots {
        let (lo, hi) = &root.interval;
        assert!(
            if lo == hi {
                p.evaluate(lo).is_zero()
            } else {
                roots_in_open_interval(&p, lo, hi) == 1
            },
            "raw={raw:?}, invalid={root:?}"
        );
    }
}

#[test]
fn root_isolation_rejects_backend_exponent_narrowing() {
    let algebra = Algebra::new(&["x".into()]).unwrap();
    let p = algebra.parse("x^65536+1").unwrap();
    assert!(algebra.specialize_univariate(&p, 0, &[]).is_err());
    // Library callers can also supply their own univariate polynomial.
    let error = isolate_union(vec![(0, p.to_univariate_from_univariate(0))])
        .unwrap_err()
        .to_string();
    assert!(error.contains("u16 exponent range"));
}

#[test]
fn isolated_rational_roots_keep_distinct_ordinals_for_scaled_fibers() {
    let values = [q("-2"), q("-1/2"), q("0"), q("1/3"), q("1"), q("2")];
    for mask in 1usize..(1 << values.len()) {
        let expected = values
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, v)| v.clone())
            .collect::<Vec<_>>();
        let expression = format!(
            "4*{}",
            expected
                .iter()
                .map(|v| format!("(x-({v}))"))
                .collect::<Vec<_>>()
                .join("*")
        );
        let p = polynomial(&expression);
        let roots = isolate_union(vec![(0, p.clone())]).unwrap();
        assert_eq!(roots.len(), expected.len(), "{expression}");
        for (i, root) in roots.iter().enumerate() {
            let (lo, hi) = &root.interval;
            assert_eq!(root.index, i);
            assert!(lo <= &expected[i] && &expected[i] <= hi);
            assert!(if lo == hi {
                p.evaluate(lo).is_zero()
            } else {
                roots_in_open_interval(&p, lo, hi) == 1
            });
        }
        for gap in 0..=roots.len() {
            let sample = sector_sample(gap.checked_sub(1).map(|i| &roots[i]), roots.get(gap));
            assert!(gap == 0 || sample > expected[gap - 1]);
            assert!(gap == roots.len() || sample < expected[gap]);
        }
    }
}

#[test]
fn samples_are_strictly_between_all_fiber_roots() {
    let roots = isolate_union(vec![(0, polynomial("(x+2)*x*(x-1/3)*(x-1/2)*(x-2)"))]).unwrap();
    let expected = [q("-2"), q("0"), q("1/3"), q("1/2"), q("2")];
    for i in 0..=roots.len() {
        let lower = i.checked_sub(1).map(|j| &roots[j]);
        let sample = sector_sample(lower, roots.get(i));
        if i > 0 {
            assert!(sample > expected[i - 1]);
        }
        if i < roots.len() {
            assert!(sample < expected[i]);
        }
    }
    assert_eq!(sector_sample(None, None), q("0"));
}

#[test]
fn continued_fraction_samples_stay_in_closed_intervals() {
    for (lo, hi) in [
        ("-7/3", "-13/6"),
        ("-1/7", "1/13"),
        ("1/3", "1/2"),
        ("7/13", "7/13"),
        ("0", "0"),
        ("1000000000001/1000000000000", "1000000000002/1000000000000"),
    ] {
        let lo = q(lo);
        let hi = q(hi);
        let sample = simplest_between(&lo, &hi);
        assert!(lo <= sample && sample <= hi);
    }
    assert_eq!(simplest_between(&q("1/3"), &q("1/2")), q("1/2"));
}

#[test]
fn sturm_matches_known_roots_for_every_small_repeated_factor_subset() {
    let candidates = [-2, -1, 0, 1, 2];
    for mask in 0u32..32 {
        let selected = candidates
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .collect::<Vec<_>>();
        let expression = if selected.is_empty() {
            "1".into()
        } else {
            selected
                .iter()
                .map(|(i, root)| format!("(x-({root}))^{}", i % 3 + 1))
                .collect::<Vec<_>>()
                .join("*")
        };
        let p = polynomial(&expression);
        for lo in -3..3 {
            for hi in lo + 1..=3 {
                let expected = selected
                    .iter()
                    .filter(|(_, root)| lo < **root && **root < hi)
                    .count();
                assert_eq!(
                    roots_in_open_interval(&p, &Rational::from(lo), &Rational::from(hi)),
                    expected,
                    "{expression}, ({lo},{hi})"
                );
            }
        }
    }
}
