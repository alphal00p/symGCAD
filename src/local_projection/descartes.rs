//! A bounded positive-ray root count proved by Descartes and exact witnesses.
use super::{PositiveRayCase, prefix_value};
use crate::{
    algebra::{Algebra, Poly},
    roots::{isolate_union, refine_once, sector_sample},
    solver::sign,
};
use anyhow::{Result, ensure};
use std::collections::HashMap;
use symbolica::prelude::*;

/// Interned polynomial IDs belong to one immutable map/append-only registry.
/// Admission slots persist for the Builder's entire lifetime: they are never
/// evicted or reset after a failed stack. Thus each polynomial introduces at
/// most two finite sets of rational-evaluation guards.
#[derive(Default)]
pub(super) struct WitnessStock {
    entries: HashMap<usize, Vec<Vec<String>>>,
    admitted: usize,
    reused: usize,
    budget_fallbacks: usize,
    source_term_fallbacks: usize,
}

impl WitnessStock {
    pub(super) fn attempt(
        &mut self,
        id: usize,
        algebra: &Algebra,
        polynomial: &Poly,
        variable: usize,
        prefix: &[Rational],
        signs: &[i8],
        max_terms: usize,
    ) -> Result<Option<(PositiveRayCase, Vec<Poly>)>> {
        crate::config::validate_descartes_witness_max_terms(max_terms)?;
        if polynomial.nterms() > max_terms {
            self.source_term_fallbacks += 1;
            return Ok(None);
        }
        if !eligible(polynomial, variable, signs, max_terms) {
            return Ok(None);
        }
        let templates = self.entries.entry(id).or_default();
        for separators in templates.iter() {
            if let Some(proof) =
                at_separators(polynomial, variable, prefix, signs, separators, max_terms)?
            {
                self.reused += 1;
                return Ok(Some(proof));
            }
        }
        if templates.len() >= 2 {
            self.budget_fallbacks += 1;
            return Ok(None);
        }
        let proof = witness(algebra, polynomial, variable, prefix, signs, max_terms)?;
        if let Some((PositiveRayCase::DescartesWitnesses { separators, .. }, _)) = &proof {
            templates.push(separators.clone());
            self.admitted += 1;
        }
        Ok(proof)
    }

    pub(super) fn report(&self, report: &mut super::LocalReport) {
        report.descartes_witness_templates = self.admitted;
        report.descartes_witness_template_reuses = self.reused;
        report.descartes_witness_budget_fallbacks = self.budget_fallbacks;
        report.descartes_witness_source_term_fallbacks = self.source_term_fallbacks;
    }
}

fn eligible(polynomial: &Poly, variable: usize, signs: &[i8], max_terms: usize) -> bool {
    let count = signs.windows(2).filter(|w| w[0] != w[1]).count();
    polynomial.degree(variable) <= 32
        && polynomial.nterms() <= max_terms
        && (2..=8).contains(&count)
        && signs.iter().all(|&s| [-1, 1].contains(&s))
}

/// Validate a supplied rational template directly. No root isolation or
/// producer history is used: strict ordered separators and alternating exact
/// evaluations saturate the independently reconstructed coefficient bound.
fn at_separators(
    polynomial: &Poly,
    variable: usize,
    prefix: &[Rational],
    signs: &[i8],
    separators: &[String],
    max_terms: usize,
) -> Result<Option<(PositiveRayCase, Vec<Poly>)>> {
    crate::config::validate_descartes_witness_max_terms(max_terms)?;
    let count = signs.windows(2).filter(|w| w[0] != w[1]).count();
    if !eligible(polynomial, variable, signs, max_terms) || separators.len() + 1 != count {
        return Ok(None);
    }
    let values = separators
        .iter()
        .map(|s| crate::algebra::rational(s))
        .collect::<Result<Vec<_>>>()?;
    if values.first().is_none_or(|x| x <= &Rational::zero())
        || values.windows(2).any(|w| w[0] >= w[1])
    {
        return Ok(None);
    }
    let endpoint = polynomial.replace(variable, &Rational::zero());
    let leading = polynomial.univariate_lcoeff(variable);
    let mut evaluation_signs = vec![sign(&prefix_value(&endpoint, prefix)?)];
    let mut guards = Vec::new();
    for value in values {
        let guard = polynomial.replace(variable, &value);
        evaluation_signs.push(sign(&prefix_value(&guard, prefix)?));
        guards.push(guard);
    }
    evaluation_signs.push(sign(&prefix_value(&leading, prefix)?));
    if evaluation_signs.contains(&0) || evaluation_signs.windows(2).any(|w| w[0] == w[1]) {
        return Ok(None);
    }
    Ok(Some((
        PositiveRayCase::DescartesWitnesses {
            coefficient_signs: signs.to_vec(),
            separators: separators.to_vec(),
            evaluation_signs,
        },
        guards,
    )))
}

pub(super) fn verify_witness(
    polynomial: &Poly,
    variable: usize,
    prefix: &[Rational],
    signs: &[i8],
    supplied: &PositiveRayCase,
    max_terms: usize,
) -> Result<Vec<Poly>> {
    let PositiveRayCase::DescartesWitnesses { separators, .. } = supplied else {
        anyhow::bail!("expected a supplied Descartes witness");
    };
    let Some((expected, guards)) =
        at_separators(polynomial, variable, prefix, signs, separators, max_terms)?
    else {
        anyhow::bail!("supplied Descartes separators do not saturate the positive-root bound");
    };
    ensure!(
        supplied == &expected,
        "Descartes coefficient or evaluation signs were modified"
    );
    Ok(guards)
}

/// Strict coefficient guards fix the Descartes bound K, counting multiplicity.
/// Alternating signs at 0,c1,...,c(K-1),+infinity force at least K distinct
/// positive roots. Equality proves exactly K simple roots in disjoint fixed
/// intervals, continuously throughout every connected ancestor cell.
pub(super) fn witness(
    algebra: &Algebra,
    polynomial: &Poly,
    variable: usize,
    prefix: &[Rational],
    coefficient_signs: &[i8],
    max_terms: usize,
) -> Result<Option<(PositiveRayCase, Vec<Poly>)>> {
    crate::config::validate_descartes_witness_max_terms(max_terms)?;
    let count = coefficient_signs
        .windows(2)
        .filter(|w| w[0] != w[1])
        .count();
    if !eligible(polynomial, variable, coefficient_signs, max_terms) {
        return Ok(None);
    }
    let fiber = algebra.specialize_univariate(polynomial, variable, prefix)?;
    ensure!(
        !fiber.evaluate(&Rational::zero()).is_zero(),
        "Descartes witness requires a nonzero endpoint"
    );
    let mut roots = isolate_union(vec![(0, fiber.clone())])?;
    for root in &mut roots {
        while root.interval.0 <= Rational::zero() && root.interval.1 >= Rational::zero() {
            // The endpoint is strictly nonzero, so this always terminates.
            refine_once(root);
        }
    }
    roots.retain(|root| root.interval.0 > Rational::zero());
    if roots.len() != count {
        return Ok(None);
    }
    let mut separators = Vec::new();
    for pair in roots.windows(2) {
        let separator = sector_sample(Some(&pair[0]), Some(&pair[1]));
        ensure!(
            separator > Rational::zero(),
            "nonpositive Descartes separator"
        );
        separators.push(separator.to_string());
    }
    at_separators(
        polynomial,
        variable,
        prefix,
        coefficient_signs,
        &separators,
        max_terms,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DEFAULT_DESCARTES_WITNESS_MAX_TERMS;
    use crate::local_projection::{PositiveRayProjection, PositiveRayRule, local_bounds};
    use crate::output::RootIndexDomain;

    fn attempt(expression: &str, parameter: i64) -> Option<(PositiveRayCase, Vec<Poly>)> {
        let algebra = Algebra::new(&["a".into(), "z".into()]).unwrap();
        let p = algebra.parse(expression).unwrap();
        let prefix = [Rational::from(parameter)];
        let signs = p
            .to_univariate_polynomial_list(1)
            .iter()
            .map(|(c, _)| sign(&prefix_value(c, &prefix).unwrap()))
            .collect::<Vec<_>>();
        witness(
            &algebra,
            &p,
            1,
            &prefix,
            &signs,
            DEFAULT_DESCARTES_WITNESS_MAX_TERMS,
        )
        .unwrap()
    }

    #[test]
    fn exact_four_root_witness_and_safe_fallbacks() {
        let (case, guards) = attempt("(z-1)*(z-2)*(z-3)*(z-4)*(z^2+2*z+a)+1/1000", 1).unwrap();
        let PositiveRayCase::DescartesWitnesses {
            separators,
            evaluation_signs,
            ..
        } = case
        else {
            panic!()
        };
        assert_eq!(separators.len(), 3);
        assert_eq!(guards.len(), 3);
        assert_eq!(evaluation_signs, vec![1, -1, 1, -1, 1]);
        assert!(attempt("z^4-z^3+z^2-z+1", 1).is_none());
        assert!(attempt("(z-1)^2*(z-2)^2", 1).is_none());
        assert!(attempt("z^4-a*z^3+z^2-z+1", 0).is_none());
        assert!(attempt("z^34-z^33+z^2-z+1", 1).is_none());
    }

    #[test]
    fn positive_ordinals_count_roots_owned_by_other_polynomials() {
        let algebra = Algebra::new(&["z".into()]).unwrap();
        let polynomials = ["z-1", "(z-1)*(z-2)*(z-3)"];
        let roots = isolate_union(
            polynomials
                .iter()
                .enumerate()
                .map(|(id, p)| {
                    (
                        id,
                        algebra
                            .specialize_univariate(&algebra.parse(p).unwrap(), 0, &[])
                            .unwrap(),
                    )
                })
                .collect(),
        )
        .unwrap();
        let ray = PositiveRayProjection {
            constraint: 0,
            coordinate: 99,
            separations: vec![],
            pair_witnesses: vec![],
            strict_bezout: vec![],
            rules: vec![PositiveRayRule {
                polynomial: 1,
                case: PositiveRayCase::DescartesWitnesses {
                    coefficient_signs: vec![-1, 1, -1, 1],
                    separators: vec!["3/2".into(), "5/2".into()],
                    evaluation_signs: vec![-1, 1, -1, 1],
                },
            }],
        };
        let bounds = local_bounds(&roots, Some(&ray)).unwrap();
        assert_eq!(
            bounds
                .iter()
                .filter(|b| b.polynomial == 1)
                .map(|b| (b.index_domain, b.index))
                .collect::<Vec<_>>(),
            vec![
                (RootIndexDomain::Positive, 1),
                (RootIndexDomain::Positive, 2)
            ]
        );
        let coincident = isolate_union(vec![
            (0, roots[0].fiber.as_ref().clone()),
            (1, roots[0].fiber.as_ref().clone()),
        ])
        .unwrap();
        let one = PositiveRayProjection {
            rules: vec![PositiveRayRule {
                polynomial: 1,
                case: PositiveRayCase::DescartesOne {
                    coefficient_signs: vec![-1, 1],
                },
            }],
            ..ray
        };
        assert!(local_bounds(&coincident, Some(&one)).is_ok());
    }

    #[test]
    fn repeated_negative_roots_do_not_obstruct_positive_delineability() {
        let algebra = Algebra::new(&["a".into(), "z".into()]).unwrap();
        let p = algebra.parse("(z-1)*(z-2)*(z^2+3*z+a)").unwrap();
        for value in ["17/8", "9/4", "5/2"] {
            let prefix = [crate::algebra::rational(value).unwrap()];
            let signs = p
                .to_univariate_polynomial_list(1)
                .iter()
                .map(|(c, _)| sign(&prefix_value(c, &prefix).unwrap()))
                .collect::<Vec<_>>();
            let (case, _) = witness(
                &algebra,
                &p,
                1,
                &prefix,
                &signs,
                DEFAULT_DESCARTES_WITNESS_MAX_TERMS,
            )
            .unwrap()
            .unwrap();
            let PositiveRayCase::DescartesWitnesses {
                separators,
                evaluation_signs,
                ..
            } = case
            else {
                panic!()
            };
            assert_eq!(separators, vec!["3/2"]);
            assert_eq!(evaluation_signs, vec![1, -1, 1]);
        }
    }

    #[test]
    fn finite_stock_falls_back_near_an_irrational_positive_collision() {
        let algebra = Algebra::new(&["a".into(), "z".into()]).unwrap();
        let p = algebra.parse("(z^2-2)^2-a").unwrap();
        let mut stock = WitnessStock::default();
        let mut missing = false;
        for value in ["1/4", "1/100", "1/10000", "1/100000000"] {
            let prefix = [crate::algebra::rational(value).unwrap()];
            let result = stock
                .attempt(
                    7,
                    &algebra,
                    &p,
                    1,
                    &prefix,
                    &[1, -1, 1],
                    DEFAULT_DESCARTES_WITNESS_MAX_TERMS,
                )
                .unwrap();
            missing |= result.is_none();
        }
        assert!(missing);
        assert_eq!(stock.entries[&7].len(), 2);
        assert_eq!(stock.admitted, 2);
        assert!(stock.budget_fallbacks > 0);
        assert!(
            stock
                .attempt(
                    7,
                    &algebra,
                    &p,
                    1,
                    &[Rational::from((1, 4))],
                    &[1, -1, 1],
                    DEFAULT_DESCARTES_WITNESS_MAX_TERMS
                )
                .unwrap()
                .is_some()
        );
        assert_eq!(stock.admitted, 2);
        assert!(stock.reused > 0);
    }

    #[test]
    fn supplied_valid_separators_need_not_match_the_discovery_heuristic() {
        let algebra = Algebra::new(&["a".into(), "z".into()]).unwrap();
        let p = algebra.parse("(z-1)*(z-2)*(z^2+3*z+a)").unwrap();
        let case = PositiveRayCase::DescartesWitnesses {
            coefficient_signs: vec![1, -1, -1, 1],
            separators: vec!["7/4".into()],
            evaluation_signs: vec![1, -1, 1],
        };
        assert!(
            verify_witness(
                &p,
                1,
                &[Rational::from((9, 4))],
                &[1, -1, -1, 1],
                &case,
                DEFAULT_DESCARTES_WITNESS_MAX_TERMS
            )
            .is_ok()
        );
    }

    fn oversized_primitive_source() -> (Algebra, Poly) {
        let algebra = Algebra::new(&["a".into(), "b".into(), "z".into(), "unused".into()]).unwrap();
        let mut s = algebra.parse("0").unwrap();
        for a in 0..=32u32 {
            for b in 0..=32u32 {
                s.append_monomial(Rational::one(), &[a, b, 0, 0]);
            }
        }
        // Primitive in z: gcd(s,a)=1. At a=0,b=1 this retains two simple
        // positive roots and a repeated negative root, with no degree drop.
        let p = &(&s * &algebra.parse("(z-1)*(z-2)*(z^2+3*z+9/4)").unwrap())
            + &algebra.parse("a").unwrap();
        assert_eq!(p.nterms(), 4356);
        (algebra, p)
    }

    #[test]
    fn explicit_source_budget_applies_to_discovery_reuse_and_independent_replay() {
        let (algebra, p) = oversized_primitive_source();
        let prefix = [Rational::zero(), Rational::one()];
        let signs = p
            .to_univariate_polynomial_list(2)
            .iter()
            .map(|(c, _)| sign(&prefix_value(c, &prefix).unwrap()))
            .collect::<Vec<_>>();
        let mut stock = WitnessStock::default();
        assert!(
            stock
                .attempt(7, &algebra, &p, 2, &prefix, &signs, 4096)
                .unwrap()
                .is_none()
        );
        assert!(stock.entries.is_empty());
        let (case, guards) = stock
            .attempt(7, &algebra, &p, 2, &prefix, &signs, 8192)
            .unwrap()
            .unwrap();
        let restored: PositiveRayCase =
            serde_json::from_slice(&serde_json::to_vec(&case).unwrap()).unwrap();
        assert_eq!(
            verify_witness(&p, 2, &prefix, &signs, &restored, 8192).unwrap(),
            guards
        );
        assert!(verify_witness(&p, 2, &prefix, &signs, &restored, 4096).is_err());
        for limit in [0, 65537, usize::MAX] {
            assert!(verify_witness(&p, 2, &prefix, &signs, &restored, limit).is_err());
            assert!(
                stock
                    .attempt(7, &algebra, &p, 2, &prefix, &signs, limit)
                    .is_err()
            );
        }
        // A previously admitted template cannot bypass a smaller source cap.
        assert!(
            stock
                .attempt(7, &algebra, &p, 2, &prefix, &signs, 4096)
                .unwrap()
                .is_none()
        );
        assert!(
            stock
                .attempt(7, &algebra, &p, 2, &prefix, &signs, 8192)
                .unwrap()
                .is_some()
        );
        assert_eq!(stock.admitted, 1);
        assert_eq!(stock.reused, 1);
        assert_eq!(stock.source_term_fallbacks, 2);
        assert_eq!(stock.budget_fallbacks, 0);
        let mut report = super::super::LocalReport::default();
        stock.report(&mut report);
        assert_eq!(report.descartes_witness_source_term_fallbacks, 2);
        let mut changed = restored;
        if let PositiveRayCase::DescartesWitnesses { separators, .. } = &mut changed {
            separators[0] = "0".into();
        } else {
            panic!();
        }
        assert!(verify_witness(&p, 2, &prefix, &signs, &changed, 8192).is_err());
    }

    #[test]
    fn enlarged_budget_preserves_all_raw_projection_guards_and_verifier_replay() {
        use super::super::{ProjectionOperations, positive_projection_raw, strict_bezout};
        struct NoExpensiveProjection;
        impl ProjectionOperations for NoExpensiveProjection {
            fn discriminant(&mut self, _: &Poly, _: usize) -> Result<Poly> {
                anyhow::bail!("ordinary discriminant fallback requested")
            }
            fn resultant(&mut self, _: &Poly, _: &Poly, _: usize) -> Result<Poly> {
                anyhow::bail!("unexpected pair resultant")
            }
        }
        let (algebra, p) = oversized_primitive_source();
        let coordinate = algebra.parse("z").unwrap();
        let polynomials = vec![coordinate.clone(), p.clone()];
        let normalized = crate::problem::NormalizedProblem {
            algebra,
            positives: vec![coordinate],
            split: vec![p.clone()],
        };
        let prefix = [Rational::zero(), Rational::one()];
        let mut stock = WitnessStock::default();
        let mut invoke = |limit, replay: Option<&PositiveRayProjection>| {
            positive_projection_raw(
                &normalized,
                &polynomials,
                &[0, 1],
                2,
                &prefix,
                0,
                0,
                false,
                2,
                false,
                None,
                false,
                &mut strict_bezout::Registry::default(),
                false,
                true,
                limit,
                Some(&mut stock),
                false,
                crate::config::DEFAULT_DESCARTES_INTERVAL_MAX_TERMS,
                crate::config::DEFAULT_DESCARTES_INTERVAL_MAX_TOTAL_TERMS,
                None,
                replay,
                false,
                &mut None,
                &mut 0.0,
                &mut NoExpensiveProjection,
            )
        };
        let declined = invoke(4096, None).unwrap_err();
        assert!(
            declined
                .to_string()
                .contains("ordinary discriminant fallback")
        );
        let (bearing, raw, _, _, _, proof) = invoke(8192, None).unwrap();
        assert_eq!(bearing, vec![1]);
        for (coefficient, _) in p.to_univariate_polynomial_list(2) {
            assert!(raw.contains(&coefficient));
        }
        assert!(raw.contains(&p.univariate_lcoeff(2)));
        assert!(raw.contains(&p.replace(2, &Rational::zero())));
        assert!(raw.contains(&p.replace(2, &Rational::from((3, 2)))));
        let (_, replayed, _, _, _, expected) = invoke(8192, Some(&proof)).unwrap();
        assert_eq!(expected, proof);
        assert_eq!(replayed, raw);
        assert!(invoke(4096, Some(&proof)).is_err());
    }
}
