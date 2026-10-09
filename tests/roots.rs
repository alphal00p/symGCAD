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
fn bisection_uses_one_sided_signs_at_other_roots() {
    let mut root = FiberRoot {
        polynomial: 0,
        index: 1,
        interval: (q("0"), q("3")),
        fiber: polynomial("x*(x-1)*(x-3)"),
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
