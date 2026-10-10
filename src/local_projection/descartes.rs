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
    ) -> Result<Option<(PositiveRayCase, Vec<Poly>)>> {
        if !eligible(polynomial, variable, signs) {
            return Ok(None);
        }
        let templates = self.entries.entry(id).or_default();
        for separators in templates.iter() {
            if let Some(proof) = at_separators(polynomial, variable, prefix, signs, separators)? {
                self.reused += 1;
                return Ok(Some(proof));
            }
        }
        if templates.len() >= 2 {
            self.budget_fallbacks += 1;
            return Ok(None);
        }
        let proof = witness(algebra, polynomial, variable, prefix, signs)?;
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
    }
}

fn eligible(polynomial: &Poly, variable: usize, signs: &[i8]) -> bool {
    let count = signs.windows(2).filter(|w| w[0] != w[1]).count();
    polynomial.degree(variable) <= 32
        && polynomial.nterms() <= 4096
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
) -> Result<Option<(PositiveRayCase, Vec<Poly>)>> {
    let count = signs.windows(2).filter(|w| w[0] != w[1]).count();
    if !eligible(polynomial, variable, signs) || separators.len() + 1 != count {
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
) -> Result<Vec<Poly>> {
    let PositiveRayCase::DescartesWitnesses { separators, .. } = supplied else {
        anyhow::bail!("expected a supplied Descartes witness");
    };
    let Some((expected, guards)) = at_separators(polynomial, variable, prefix, signs, separators)?
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
) -> Result<Option<(PositiveRayCase, Vec<Poly>)>> {
    let count = coefficient_signs
        .windows(2)
        .filter(|w| w[0] != w[1])
        .count();
    if !eligible(polynomial, variable, coefficient_signs) {
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
    at_separators(polynomial, variable, prefix, coefficient_signs, &separators)
}

#[cfg(test)]
mod tests {
    use super::*;
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
        witness(&algebra, &p, 1, &prefix, &signs).unwrap()
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
            (0, roots[0].fiber.clone()),
            (1, roots[0].fiber.clone()),
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
            let (case, _) = witness(&algebra, &p, 1, &prefix, &signs).unwrap().unwrap();
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
                .attempt(7, &algebra, &p, 1, &prefix, &[1, -1, 1])
                .unwrap();
            missing |= result.is_none();
        }
        assert!(missing);
        assert_eq!(stock.entries[&7].len(), 2);
        assert_eq!(stock.admitted, 2);
        assert!(stock.budget_fallbacks > 0);
        assert!(
            stock
                .attempt(7, &algebra, &p, 1, &[Rational::from((1, 4))], &[1, -1, 1])
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
        assert!(verify_witness(&p, 1, &[Rational::from((9, 4))], &[1, -1, -1, 1], &case).is_ok());
    }
}
