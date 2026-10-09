use symgcad::algebra::Algebra;
use symgcad::ordering::{
    OrderingMethod, brown_score, cheap_seed_orders, choose_order, rank_candidates, t1_score,
};

fn algebra(names: &[&str]) -> Algebra {
    Algebra::new(&names.iter().map(|s| (*s).into()).collect::<Vec<_>>()).unwrap()
}

#[test]
fn upstream_tie_breaks_and_projection_direction() {
    let a = algebra(&["x", "y"]);
    let p = vec![a.parse("x").unwrap(), a.parse("y").unwrap()];
    let groups = vec![vec![0, 1]];
    // Upstream MODS/T1 select the first minimum; SOTD selects the last.
    for method in [OrderingMethod::Mods, OrderingMethod::T1] {
        assert_eq!(
            choose_order(&a, &p, &groups, method).unwrap().order,
            vec![1, 0]
        );
    }
    assert_eq!(
        choose_order(&a, &p, &groups, OrderingMethod::Sotd)
            .unwrap()
            .order,
        vec![0, 1]
    );
}

#[test]
fn every_method_preserves_outer_parameter_groups_and_input() {
    let a = algebra(&["s", "t", "x", "y"]);
    let p = vec![a.parse("s*x+t*y+1").unwrap()];
    let before = p.clone();
    let groups = vec![vec![0, 1], vec![2, 3]];
    for method in [
        OrderingMethod::Given,
        OrderingMethod::Mods,
        OrderingMethod::Sotd,
        OrderingMethod::T1,
        OrderingMethod::Brown,
        OrderingMethod::Sparsity,
    ] {
        let order = choose_order(&a, &p, &groups, method).unwrap().order;
        assert!(order[..2].iter().all(|&v| v < 2));
        assert!(order[2..].iter().all(|&v| v >= 2));
    }
    assert_eq!(p, before);
    assert_eq!(groups, vec![vec![0, 1], vec![2, 3]]);
}

#[test]
fn t1_breaks_degree_ties_with_exact_average_exponents() {
    let a = algebra(&["x", "y"]);
    let p = vec![a.parse("x^2+x+y^2").unwrap()];
    assert_eq!(t1_score(&p, 0).0, t1_score(&p, 1).0);
    assert!(t1_score(&p, 1) < t1_score(&p, 0));
    assert_eq!(rank_candidates(&p, &[0, 1], OrderingMethod::T1), vec![1, 0]);
}

#[test]
fn brown_counts_terms_containing_the_variable() {
    let a = algebra(&["x", "y", "z"]);
    let p = vec![a.parse("x^2+y^2*z+x*z").unwrap()];
    assert_eq!(brown_score(&p, 0), (2, 2, 2));
    assert_eq!(brown_score(&p, 1), (2, 3, 1));
    assert_eq!(
        rank_candidates(&p, &[0, 1], OrderingMethod::Brown),
        vec![0, 1]
    );
}

#[test]
fn sparsity_prefers_path_endpoints_without_removing_polynomials() {
    let a = algebra(&["x", "y", "z", "w"]);
    let p = ["x*y+1", "y*z+1", "z*w+1"].map(|s| a.parse(s).unwrap());
    let order = rank_candidates(&p, &[1, 2, 0, 3], OrderingMethod::Sparsity);
    assert_eq!(&order[..2], &[0, 3]);
}

#[test]
fn empty_basis_has_deterministic_order_and_no_division_by_zero() {
    let a = algebra(&["x", "y"]);
    let p = vec![a.parse("1").unwrap()];
    assert_eq!(
        choose_order(&a, &p, &[vec![0, 1]], OrderingMethod::T1)
            .unwrap()
            .order,
        vec![1, 0]
    );
    assert_eq!(
        cheap_seed_orders(&[], &[vec![0], vec![1, 2]])[0],
        vec![0, 1, 2]
    );
}

#[test]
fn malformed_groups_are_errors() {
    let a = algebra(&["x", "y"]);
    assert!(choose_order(&a, &[], &[vec![0, 0]], OrderingMethod::Given).is_err());
    assert!(choose_order(&a, &[], &[vec![0, 2]], OrderingMethod::Given).is_err());
}

#[test]
fn reproduces_reference_melih_box_t1_order() {
    // The non-slow regression `test_melih_box` in the supplied gcad corpus.
    let a = algebra(&["x1", "x2", "x3", "x4", "x5", "x6", "x7"]);
    let f = a.parse("-4*x1*x4*x5+2*x1*x3*x6+2*x2*x3*x6+2*x2*x5*x6+2*x3*x5*x6+x1*x6^2+x2*x6^2+x5*x6^2+2*x2*x3*x7+2*x2*x4*x7+2*x2*x5*x7+2*x3*x5*x7+2*x2*x6*x7+2*x3*x6*x7+2*x5*x6*x7+x6^2*x7+x3*x7^2+x4*x7^2+x5*x7^2+x6*x7^2").unwrap();
    let mut constraints = vec![f];
    constraints.extend(a.names.iter().map(|v| a.parse(v).unwrap()));
    let result = choose_order(&a, &constraints, &[(0..7).collect()], OrderingMethod::T1).unwrap();
    assert_eq!(result.order, vec![6, 4, 5, 2, 3, 1, 0]);
}
