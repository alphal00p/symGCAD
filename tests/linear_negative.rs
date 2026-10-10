use symgcad::{
    Problem,
    algebra::{Algebra, rational},
    comparison::compare_generic,
    output::{RootIndexDomain, SolveResult, Status},
    solver::{selected_order, solve},
    verify::verify,
};

fn problem(target: &str, guide: Option<&str>) -> Problem {
    let mut p = Problem {
        variables: vec!["x".into(), "z".into()],
        constraints: vec!["x>0".into(), "z>0".into(), format!("({target})<0")],
        ..Problem::default()
    };
    p.solver.engine = "linear_negative".into();
    p.solver.ordering = "given".into();
    p.solver.negative_linear_guide = guide.map(str::to_owned);
    p.limits.wall_time_secs = 20.0;
    p
}

fn checked(p: &Problem) -> SolveResult {
    let result = solve(p).unwrap();
    assert_eq!(
        result.status,
        Status::CompleteGeneric,
        "{:?}",
        result.diagnostic
    );
    assert!(verify(&result).unwrap().verified);
    assert_eq!(selected_order(p).unwrap(), result.order);
    assert_eq!(result.problem.constraints, p.constraints);
    assert!(result.sign_filter.is_none());
    assert!(result.monotone.is_none());
    assert!(result.linear_negative.is_some());
    result
}

fn compare_baseline(p: &Problem, result: &SolveResult) {
    let mut baseline = p.clone();
    baseline.solver.engine = "baseline".into();
    baseline.solver.negative_linear_guide = None;
    let reference = solve(&baseline).unwrap();
    assert!(verify(&reference).unwrap().verified);
    assert_eq!(
        compare_generic(result, &reference, &p.limits)
            .unwrap()
            .equivalent,
        Some(true)
    );
}

fn rejected(result: &SolveResult) {
    assert!(verify(result).map_or(true, |v| !v.verified));
}

#[test]
fn exact_positive_guide_reduces_the_negative_target_to_its_slope() {
    // C=D=1 and B*C-A*D = x-(x-1)=1: no sampled sign is used.
    let p = problem("(x-1)*z+x", Some("z+1"));
    let result = checked(&p);
    assert_eq!(result.cells.len(), 1);
    assert!(result.cells[0].signs.is_empty());
    let root = result.cells[0].axes[1].lower.as_ref().unwrap();
    assert_eq!(root.index_domain, RootIndexDomain::RealDescending);
    assert_eq!(root.index, 0);
    assert!(result.cells[0].axes[1].upper.is_none());
    compare_baseline(&p, &result);
}

#[test]
fn strictly_negative_slope_crosses_zero_intercept_without_excluding_its_seam() {
    let p = problem("-(x+1)*z+x-1", None);
    let result = checked(&p);
    assert_eq!(result.cells.len(), 1);
    let cell = &result.cells[0];
    assert_eq!(cell.axes[0].sample, "1");
    let root = cell.axes[1].lower.as_ref().unwrap();
    assert_eq!(root.index_domain, RootIndexDomain::RealDescending);
    assert_eq!(root.index, 0);
    assert_eq!(rational(&root.lower).unwrap(), rational("0").unwrap());
    assert_eq!(rational(&root.upper).unwrap(), rational("0").unwrap());
    let algebra = Algebra::new(&result.order).unwrap();
    let intercept = algebra.parse("x-1").unwrap();
    assert!(result.exceptional_polynomials.iter().all(|&id| {
        let q = algebra.parse(&result.polynomials[id]).unwrap();
        q != intercept && q != -intercept.clone()
    }));
    compare_baseline(&p, &result);

    let mut bad = result.clone();
    bad.cells[0].axes[1].lower.as_mut().unwrap().index_domain = RootIndexDomain::Real;
    rejected(&bad);
    let mut bad = result.clone();
    bad.cells[0].axes[1].lower.as_mut().unwrap().index = 1;
    rejected(&bad);
    let mut bad = result.clone();
    bad.cells[0].axes[1].sample = "0".into();
    rejected(&bad);
}

#[test]
fn original_negative_gluza30_has_one_cell_and_four_checked_restorations() {
    let mut p = Problem::from_toml(include_str!("fixtures/gluza30_negative.toml")).unwrap();
    let guide: serde_json::Value =
        serde_json::from_str(include_str!("../reference/gluza30_rayleigh_guide.json")).unwrap();
    p.solver.engine = "linear_negative".into();
    p.solver.negative_linear_guide = Some(guide["guide"].as_str().unwrap().into());
    p.solver.order = serde_json::from_value(guide["lifting_order"].clone()).unwrap();
    p.limits.wall_time_secs = 20.0;
    let result = checked(&p);
    assert_eq!(result.cells.len(), 1);
    assert_eq!(result.order, p.solver.order);
    assert_eq!(result.cells[0].axes.len(), 7);
    assert_eq!(
        result.cells[0]
            .axes
            .iter()
            .filter(|axis| {
                axis.lower
                    .as_ref()
                    .is_some_and(|root| root.index_domain == RootIndexDomain::RealDescending)
            })
            .count(),
        4
    );
    assert!(result.cells[0].axes.iter().all(|axis| axis.upper.is_none()));
    assert!(result.cells[0].signs.is_empty());
    // The actual chain's last peeled x5 axis crosses its zero-intercept seam
    // x6+x7=1. The same composite selector works on and on both sides of it.
    let algebra = Algebra::new(&result.order).unwrap();
    let x5_axis = result.order.iter().position(|v| v == "x5").unwrap();
    let bound = result.cells[0].axes[x5_axis].lower.as_ref().unwrap();
    let polynomial = algebra
        .parse(&result.polynomials[bound.polynomial])
        .unwrap();
    for (coordinate, expected, distinct) in [("1/4", "1/2", 2), ("1/2", "0", 1), ("1", "0", 2)] {
        let prefix = [
            rational("1").unwrap(),
            rational(coordinate).unwrap(),
            rational(coordinate).unwrap(),
        ];
        let fiber = algebra
            .specialize_univariate(&polynomial, x5_axis, &prefix)
            .unwrap();
        let roots = symgcad::roots::isolate_union(vec![(0, fiber)]).unwrap();
        assert_eq!(roots.len(), distinct);
        let expected = rational(expected).unwrap();
        let largest = roots.last().unwrap();
        assert!(largest.interval.0 <= expected && expected <= largest.interval.1);
    }
    let decoded: SolveResult =
        serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    assert!(verify(&decoded).unwrap().verified);
}

#[test]
fn exact_cell_inventory_boundaries_and_order_are_mandatory() {
    let p = problem("(x-1)*z+x", Some("z+1"));
    let result = checked(&p);
    let mut bad = result.clone();
    bad.cells.clear();
    rejected(&bad);
    let mut bad = result.clone();
    bad.cells.push(result.cells[0].clone());
    rejected(&bad);
    let mut bad = result.clone();
    bad.order.reverse();
    rejected(&bad);
    let mut bad = result.clone();
    bad.cells[0].axes[1].lower = None;
    rejected(&bad);
    let mut bad = result.clone();
    let boundary = bad.cells[0].axes[1].lower.as_ref().unwrap().polynomial;
    bad.polynomials[boundary] = "z".into();
    rejected(&bad);
}

#[test]
fn largest_root_selectors_do_not_change_historical_global_proof_semantics() {
    let mut p = problem("(x-1)*z+x", Some("z+1"));
    p.solver.engine = "baseline".into();
    p.solver.negative_linear_guide = None;
    let result = solve(&p).unwrap();
    assert!(verify(&result).unwrap().verified);
    let value = serde_json::to_value(&result).unwrap();
    assert!(value.get("linear_negative").is_none());
    let roundtrip: SolveResult = serde_json::from_value(value).unwrap();
    assert!(verify(&roundtrip).unwrap().verified);
    let mut bad = result.clone();
    let root = bad
        .cells
        .iter_mut()
        .flat_map(|cell| &mut cell.axes)
        .find_map(|axis| axis.lower.as_mut().or(axis.upper.as_mut()))
        .unwrap();
    root.index_domain = RootIndexDomain::RealDescending;
    rejected(&bad);

    // Change the trace and exported roots together, so rejection cannot rely
    // only on their agreeing with one another.
    fn replace_selectors(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(fields) => {
                if fields.contains_key("polynomial") && fields.contains_key("index") {
                    fields.insert("index_domain".into(), serde_json::json!("real_descending"));
                }
                for field in fields.values_mut() {
                    replace_selectors(field);
                }
            }
            serde_json::Value::Array(values) => values.iter_mut().for_each(replace_selectors),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(&result).unwrap();
    replace_selectors(&mut value);
    rejected(&serde_json::from_value(value).unwrap());
}

#[test]
fn multiple_base_components_are_restored_once_and_in_the_proved_order() {
    let slope = "(x-1)*(x-2)*(x-3)";
    let p = problem(&format!("({slope})*z+({slope})+1"), Some("z+1"));
    let result = checked(&p);
    assert_eq!(result.cells.len(), 2);
    compare_baseline(&p, &result);
    let mut bad = result.clone();
    bad.cells.swap(0, 1);
    rejected(&bad);
    let mut bad = result.clone();
    bad.cells[1] = bad.cells[0].clone();
    rejected(&bad);

    let mut limited = p.clone();
    limited.limits.max_cells = Some(1);
    let mut result = solve(&limited).unwrap();
    assert_eq!(result.status, Status::Incomplete);
    assert_eq!(
        result.linear_negative.as_ref().unwrap().base.status,
        Status::Incomplete
    );
    rejected(&result);
    result.status = Status::CompleteGeneric;
    rejected(&result);

    limited.limits.wall_time_secs = 1e-9;
    let result = solve(&limited).unwrap();
    assert_eq!(result.status, Status::Incomplete);
    rejected(&result);
}

#[test]
fn original_group_and_fixed_order_obligations_survive_the_new_engine() {
    let mut p = problem("(x-1)*z+x", Some("z+1"));
    p.variables.reverse();
    p.variable_groups = vec![vec!["x".into()], vec!["z".into()]];
    p.solver.ordering = "brown".into();
    let result = checked(&p);
    assert_eq!(result.order, vec!["x", "z"]);
    p.solver.order = vec!["x".into(), "z".into()];
    let result = checked(&p);
    assert_eq!(result.order, p.solver.order);
    let mut bad = result.clone();
    bad.problem.solver.order.reverse();
    rejected(&bad);
    let mut bad = result.clone();
    bad.problem.variable_groups.reverse();
    rejected(&bad);

    p.variable_groups.clear();
    p.solver.order = vec!["z".into(), "x".into()];
    let result = checked(&p);
    assert_eq!(result.order, p.solver.order);
    assert!(result.linear_negative.as_ref().unwrap().steps.is_empty());
    compare_baseline(&p, &result);
}

#[test]
fn a_nonnegative_square_may_vanish_inside_the_base_without_an_exception() {
    let p = problem("(x-2)*z+(x-2)+(x-1)^2", Some("z+1"));
    let result = checked(&p);
    compare_baseline(&p, &result);
    let algebra = Algebra::new(&result.order).unwrap();
    let square_zero = algebra.parse("x-1").unwrap();
    assert!(result.exceptional_polynomials.iter().all(|&id| {
        let q = algebra.parse(&result.polynomials[id]).unwrap();
        q != square_zero && q != -square_zero.clone()
    }));

    // The comparison may vanish identically too: strict C,D do not require a
    // strictly positive Rayleigh difference.
    let p = problem("(x-1)*(z+1)", Some("z+1"));
    let result = checked(&p);
    assert_eq!(result.cells.len(), 1);
    compare_baseline(&p, &result);
}

#[test]
fn coefficient_guide_square_and_domain_evidence_are_independently_replayed() {
    let p = problem("(x-1)*z+x", Some("z+1"));
    let result = checked(&p);
    assert_eq!(result.linear_negative.as_ref().unwrap().steps.len(), 1);
    let original = serde_json::to_value(&result).unwrap();
    for (path, replacement) in [
        ("/linear_negative/target_constraint", serde_json::json!(0)),
        ("/linear_negative/steps/0/variable", serde_json::json!("x")),
        (
            "/linear_negative/steps/0/polynomial",
            serde_json::json!("(x-1)*z+x+1"),
        ),
        ("/linear_negative/steps/0/leading", serde_json::json!("x-2")),
        (
            "/linear_negative/steps/0/endpoint",
            serde_json::json!("x+1"),
        ),
        (
            "/linear_negative/steps/0/rule/guide",
            serde_json::json!("z^2+1"),
        ),
        (
            "/linear_negative/steps/0/rule/guide",
            serde_json::json!("z+w"),
        ),
        (
            "/linear_negative/steps/0/rule/leading/polynomial",
            serde_json::json!("z"),
        ),
        (
            "/linear_negative/steps/0/rule/leading/sign",
            serde_json::json!(-1),
        ),
        (
            "/linear_negative/steps/0/rule/endpoint/polynomial",
            serde_json::json!("x"),
        ),
        (
            "/linear_negative/steps/0/rule/endpoint/domain/bounds/0/lower",
            serde_json::json!("1"),
        ),
        (
            "/linear_negative/steps/0/rule/multiplier",
            serde_json::json!("-1"),
        ),
        (
            "/linear_negative/steps/0/rule/multiplier",
            serde_json::json!("0"),
        ),
        (
            "/linear_negative/steps/0/rule/square",
            serde_json::json!("x"),
        ),
        (
            "/linear_negative/steps/0/rule/square",
            serde_json::json!("z"),
        ),
    ] {
        let mut forged = original.clone();
        *forged
            .pointer_mut(path)
            .unwrap_or_else(|| panic!("missing evidence field {path}")) = replacement;
        let forged: SolveResult = serde_json::from_value(forged).unwrap();
        assert!(verify(&forged).is_err(), "accepted modification at {path}");
    }

    let mut bad = result.clone();
    bad.linear_negative.as_mut().unwrap().base.cells.clear();
    rejected(&bad);
    let mut bad = result.clone();
    *bad.linear_negative
        .as_mut()
        .unwrap()
        .base
        .problem
        .constraints
        .last_mut()
        .unwrap() = "x>1".into();
    rejected(&bad);
    let mut bad = result.clone();
    bad.linear_negative
        .as_mut()
        .unwrap()
        .base
        .problem
        .limits
        .wall_time_secs = p.limits.wall_time_secs + 1.0;
    rejected(&bad);

    let direct = checked(&problem("-(x+1)*z+x-1", None));
    let mut value = serde_json::to_value(direct).unwrap();
    *value
        .pointer_mut("/linear_negative/steps/0/rule/certificate/sign")
        .unwrap() = serde_json::json!(1);
    rejected(&serde_json::from_value(value).unwrap());
}

#[test]
fn unsupported_domains_fail_explicitly_and_inapplicable_guides_do_not_prune() {
    let p = problem("(x-1)*z+x", Some("z+1"));
    for mutate in 0..4 {
        let mut unsupported = p.clone();
        match mutate {
            0 => unsupported.constraints.push("z<2".into()),
            1 => unsupported.split.push("x-1".into()),
            2 => {
                unsupported.solver.homogeneous_block = unsupported.variables.clone();
                unsupported.solver.homogeneous_anchor = Some("x".into());
            }
            _ => unsupported.constraints.retain(|q| q != "z>0"),
        }
        assert!(
            solve(&unsupported).is_err(),
            "accepted unsupported mode {mutate}"
        );
    }

    // On x>1 this target has an admissible bounded negative fiber although
    // its slope is positive. The unproved guide must not discard that region.
    let p = problem("(x-1)*z-x", Some("z+1"));
    let result = checked(&p);
    assert!(result.linear_negative.as_ref().unwrap().steps.is_empty());
    compare_baseline(&p, &result);
    let mut invalid = p;
    invalid.solver.negative_linear_guide = Some("z+w".into());
    assert!(solve(&invalid).is_err());
}

#[test]
fn exact_rational_parameters_normalize_both_target_and_proposed_guide() {
    let mut p = problem("a*(x-1)*z+a*x", Some("c*(z+1)"));
    p.parameters.insert("a".into(), "3/7".into());
    p.parameters.insert("c".into(), "5/11".into());
    let result = checked(&p);
    assert_eq!(result.cells.len(), 1);
    assert_eq!(result.problem.parameters, p.parameters);
    assert!(
        result
            .linear_negative
            .as_ref()
            .unwrap()
            .base
            .problem
            .parameters
            .is_empty()
    );
    compare_baseline(&p, &result);

    let reference = checked(&problem("(x-1)*z+x", Some("z+1")));
    assert_eq!(
        compare_generic(&result, &reference, &p.limits)
            .unwrap()
            .equivalent,
        Some(true)
    );
    for (name, value) in [("a", "-3/7"), ("c", "-5/11")] {
        let mut bad = result.clone();
        bad.problem.parameters.insert(name.into(), value.into());
        rejected(&bad);
    }
}

#[test]
fn constant_false_and_strict_zero_constraints_remain_in_the_verified_base() {
    for contradiction in ["false", "0>0", "1<0"] {
        let mut p = problem("(x-1)*z+x", Some("z+1"));
        p.constraints.insert(0, contradiction.into());
        let result = checked(&p);
        assert!(result.cells.is_empty());
        let evidence = result.linear_negative.as_ref().unwrap();
        assert_eq!(evidence.target_constraint, 3);
        assert!(evidence.base.cells.is_empty());
        assert!(verify(&evidence.base).unwrap().verified);
        compare_baseline(&p, &result);
        let mut bad = result.clone();
        bad.linear_negative
            .as_mut()
            .unwrap()
            .base
            .problem
            .constraints
            .remove(0);
        rejected(&bad);
    }
}

#[test]
fn original_negative_gluza33_has_three_cells_and_two_nonunit_square_identities() {
    let mut p = Problem::from_toml(include_str!("fixtures/gluza33_negative.toml")).unwrap();
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("../reference/gluza33_rayleigh_guide.json")).unwrap();
    p.solver.engine = "linear_negative".into();
    p.solver.negative_linear_guide = Some(reference["guide"].as_str().unwrap().into());
    p.solver.order = serde_json::from_value(reference["lifting_order"].clone()).unwrap();
    p.limits.wall_time_secs = 20.0;
    let result = checked(&p);
    assert_eq!(result.cells.len(), 3);
    assert_eq!(result.order, p.solver.order);
    let evidence = result.linear_negative.as_ref().unwrap();
    assert_eq!(
        evidence
            .steps
            .iter()
            .map(|s| s.variable.as_str())
            .collect::<Vec<_>>(),
        vec!["x3", "x5"]
    );
    assert!(verify(&evidence.base).unwrap().verified);
    assert_eq!(evidence.base.cells.len(), 3);

    // Reconstruct from the original normalized target and guide, independently
    // of the recorded coefficient strings and certificate replay.
    let normalized = symgcad::problem::normalize(&p, &p.variables).unwrap();
    let algebra = &normalized.algebra;
    let mut current = -normalized.positives[evidence.target_constraint].clone();
    let mut guide = algebra.parse(reference["guide"].as_str().unwrap()).unwrap();
    for step in &evidence.steps {
        let axis = p
            .variables
            .iter()
            .position(|v| v == &step.variable)
            .unwrap();
        let z = algebra.parse(&step.variable).unwrap();
        let a = current.derivative(axis);
        let az = &a * &z;
        let b = &current - &az;
        let c = guide.derivative(axis);
        let cz = &c * &z;
        let d = &guide - &cz;
        assert_eq!(algebra.parse(&step.polynomial).unwrap(), current);
        assert_eq!(algebra.parse(&step.leading).unwrap(), a);
        assert_eq!(algebra.parse(&step.endpoint).unwrap(), b);
        let symgcad::linear_negative::LinearNegativeRule::PositiveGuide {
            leading,
            endpoint,
            multiplier,
            square,
            ..
        } = &step.rule
        else {
            panic!("Gluza33 requires two positive-guide rules")
        };
        assert_eq!(rational(multiplier).unwrap(), rational("10").unwrap());
        assert_eq!(algebra.parse(&leading.polynomial).unwrap(), c);
        assert_eq!(algebra.parse(&endpoint.polynomial).unwrap(), d);
        let bc = &b * &c;
        let ad = &a * &d;
        let square = algebra.parse(square).unwrap();
        assert_eq!(
            &bc - &ad,
            &square.pow(2) * &current.constant(rational("10").unwrap())
        );
        current = a;
        guide = c;
    }
    let cone_expression = "(x0+x2+x7)^2-8*(x0+x2+x7)*(x1+x4+x6)+(x1+x4+x6)^2";
    let cone = algebra.parse(cone_expression).unwrap();
    assert_eq!(cone.nterms(), 21);
    assert_eq!(current, cone);
    assert_eq!(
        current,
        algebra
            .parse(reference["residual"].as_str().unwrap())
            .unwrap()
    );
    assert_eq!(
        guide,
        algebra
            .parse(reference["residual_guide"].as_str().unwrap())
            .unwrap()
    );
    let base = symgcad::problem::normalize(&evidence.base.problem, &evidence.base.order).unwrap();
    assert_eq!(
        base.positives.last().unwrap(),
        &(-base.algebra.parse(cone_expression).unwrap())
    );
    assert_eq!(
        result
            .cells
            .iter()
            .flat_map(|cell| &cell.axes)
            .filter(|axis| {
                axis.lower
                    .as_ref()
                    .is_some_and(|bound| bound.index_domain == RootIndexDomain::RealDescending)
            })
            .count(),
        6
    );
    let roundtrip: SolveResult =
        serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    assert!(verify(&roundtrip).unwrap().verified);

    let mut bad = result;
    let symgcad::linear_negative::LinearNegativeRule::PositiveGuide { multiplier, .. } =
        &mut bad.linear_negative.as_mut().unwrap().steps[0].rule
    else {
        unreachable!()
    };
    *multiplier = "1".into();
    rejected(&bad);
}
