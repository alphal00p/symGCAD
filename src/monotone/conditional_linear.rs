//! A sufficient condition for endpoint-only quadratic peeling. All arithmetic
//! and both strict nonvanishing certificates are exact Symbolica operations.

use super::{coefficient, positive_axis_constraint};
use crate::{
    algebra::Poly,
    domain::{CertificateOptions, Domain, DomainCertificate, certify_nonzero, verify_certificate},
    problem::NormalizedProblem,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use symbolica::prelude::*;

const MAX_CANDIDATES: usize = 128;
const MAX_ACTIVE_AXES: usize = 8;
const MAX_MAP_AXES: usize = 64;
const MAX_INPUT_TERMS: usize = 256;
const MAX_DIFFERENCE_TERMS: usize = 2048;
const MAX_TOTAL_DEGREE: u128 = 8;
const MAX_COEFFICIENT_BITS: u64 = 512;
const MAX_PRODUCT_WORK: u128 = 2 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConditionalLinearEvidence {
    /// A polynomial in the unpeeled base variables, independent of this axis.
    pub multiplier: String,
    pub positive_multiplier: DomainCertificate,
    /// Strict positivity of the reconstructed h*b-c, never a stored identity.
    pub positive_difference: DomainCertificate,
}

fn bounded(p: &Poly, terms: usize, degree: u128) -> bool {
    p.nvars() <= MAX_MAP_AXES
        && p.nterms() <= terms
        && p.coefficients.iter().all(|c| {
            c.numerator_ref().significant_bits() <= MAX_COEFFICIENT_BITS
                && c.denominator_ref().significant_bits() <= MAX_COEFFICIENT_BITS
        })
        && p.exponents_iter()
            .all(|m| m.iter().map(|&e| u128::from(e)).sum::<u128>() <= degree)
}

fn input_bounded(p: &Poly) -> bool {
    bounded(p, MAX_INPUT_TERMS, MAX_TOTAL_DEGREE)
        && (0..p.nvars()).filter(|&v| p.degree(v) != 0).count() <= MAX_ACTIVE_AXES
}

/// Enumerate small positive coordinate sums without assuming a graph topology.
/// Variables absent from the quadratic leading coefficient are tried together
/// and with one further axis first; then all subsets are ordered by size.
fn masks(p: &Poly, axis: usize, axes: &[usize]) -> Vec<usize> {
    let leading = coefficient(p, axis, 2);
    let complement = axes.iter().enumerate().fold(0usize, |mask, (i, &v)| {
        mask | (usize::from(leading.degree(v) == 0) << i)
    });
    let constant = 1usize << axes.len();
    let mut result = vec![constant];
    if complement != 0 {
        result.push(complement);
        result.push(complement | constant);
        result.extend((0..axes.len()).map(|i| complement | (1 << i)));
        result.extend((0..axes.len()).map(|i| complement | (1 << i) | constant));
    }
    let mut rest: Vec<_> = (1..(1usize << (axes.len() + 1))).collect();
    rest.sort_by_key(|&mask| (mask.count_ones(), mask));
    result.extend(rest);
    let mut seen = BTreeSet::new();
    result.retain(|&mask| seen.insert(mask));
    result.truncate(MAX_CANDIDATES);
    result
}

pub(super) fn discover(
    normalized: &NormalizedProblem,
    p: &Poly,
    axis: usize,
    domain: &Domain,
    options: &CertificateOptions,
) -> Result<Option<ConditionalLinearEvidence>> {
    if !input_bounded(p) || p.degree(axis) != 2 {
        return Ok(None);
    }
    let axes: Vec<_> = (0..p.nvars())
        .filter(|&v| {
            v != axis
                && p.degree(v) != 0
                && normalized
                    .positives
                    .iter()
                    .any(|c| positive_axis_constraint(c, v))
        })
        .collect();
    let b = coefficient(p, axis, 1);
    let c = coefficient(p, axis, 0);
    let mut work = 0u128;
    for mask in masks(p, axis, &axes) {
        let mut h = if mask & (1 << axes.len()) != 0 {
            p.one()
        } else {
            p.zero()
        };
        for (i, &variable) in axes.iter().enumerate() {
            if mask & (1 << i) != 0 {
                let mut exponent = vec![0; p.nvars()];
                exponent[variable] = 1;
                h.append_monomial(Rational::one(), &exponent);
            }
        }
        work += p.nvars() as u128 * (h.nterms() as u128 * b.nterms() as u128 + c.nterms() as u128);
        if work > MAX_PRODUCT_WORK {
            break;
        }
        // Degree <=8 and a linear multiplier bound exponent addition before
        // multiplication. The cap is independent of Symbolica's exponent type.
        let difference = &h * &b - c.clone();
        if !bounded(&difference, MAX_DIFFERENCE_TERMS, MAX_TOTAL_DEGREE + 1) {
            continue;
        }
        let Some(positive_difference) =
            certify_nonzero(&normalized.algebra, &difference, domain, options)?
                .filter(|certificate| certificate.sign == 1)
        else {
            continue;
        };
        let Some(positive_multiplier) = certify_nonzero(&normalized.algebra, &h, domain, options)?
            .filter(|certificate| certificate.sign == 1)
        else {
            continue;
        };
        return Ok(Some(ConditionalLinearEvidence {
            multiplier: normalized.algebra.format(&h),
            positive_multiplier,
            positive_difference,
        }));
    }
    Ok(None)
}

pub(super) fn verify(
    normalized: &NormalizedProblem,
    p: &Poly,
    axis: usize,
    evidence: &ConditionalLinearEvidence,
) -> Result<()> {
    ensure!(
        input_bounded(p) && p.degree(axis) == 2,
        "conditional quadratic input exceeds proof bounds"
    );
    let h = normalized.algebra.parse(&evidence.multiplier)?;
    ensure!(
        bounded(&h, MAX_ACTIVE_AXES, 1)
            && h.degree(axis) == 0
            && (0..p.nvars()).all(|v| h.degree(v) == 0 || p.degree(v) > 0),
        "conditional multiplier has unsupported degree or support"
    );
    ensure!(
        evidence.positive_multiplier.sign == 1 && evidence.positive_difference.sign == 1,
        "conditional coefficient certificates must be strictly positive"
    );
    verify_certificate(
        &normalized.algebra,
        &h,
        &normalized.positives,
        &[],
        &evidence.positive_multiplier,
    )?;
    let difference = &h * &coefficient(p, axis, 1) - coefficient(p, axis, 0);
    ensure!(
        bounded(&difference, MAX_DIFFERENCE_TERMS, MAX_TOTAL_DEGREE + 1),
        "conditional difference exceeds proof bounds"
    );
    verify_certificate(
        &normalized.algebra,
        &difference,
        &normalized.positives,
        &[],
        &evidence.positive_difference,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Problem, problem::normalize};

    fn find(expression: &str) -> Result<Option<ConditionalLinearEvidence>> {
        let problem = Problem {
            variables: vec!["x".into(), "z".into()],
            constraints: vec!["x>0".into(), "z>0".into()],
            split: vec![expression.into()],
            ..Problem::default()
        };
        let n = normalize(&problem, &problem.variables)?;
        let domain = crate::domain::derive_domain(&n.algebra, &n.positives, &[])?;
        discover(&n, &n.split[0], 1, &domain, &CertificateOptions::default())
    }

    #[test]
    fn discovery_is_bounded_before_polynomial_products() {
        assert!(find("z^2+(x^9-1)*z+x^10").unwrap().is_none());
        let huge = format!("z^2+({}-x)*z-1", Integer::from(2).pow(513));
        assert!(find(&huge).unwrap().is_none());
        let algebra = crate::algebra::Algebra::new(&["x".into(), "z".into()]).unwrap();
        let p = algebra.parse("z^2+(x-1)*z+x^2-2*x").unwrap();
        assert_eq!(masks(&p, 1, &[0]), vec![2, 1, 3]);
        let generated = masks(&p, 1, &[0; MAX_ACTIVE_AXES]);
        assert!(generated.len() <= MAX_CANDIDATES);
        assert_eq!(
            generated.iter().collect::<BTreeSet<_>>().len(),
            generated.len()
        );
    }

    #[test]
    fn failed_strict_or_two_positive_root_conditions_are_not_certified() {
        assert!(find("z^2-z+x").unwrap().is_none());
        assert!(find("z^2-z-1").unwrap().is_none());
        assert!(find("z^2+(x-1)*z+x*(x-1)").unwrap().is_none());
    }

    #[test]
    fn original_fixed_a381_and_ax381_have_two_exact_conditional_peels() {
        for source in [
            include_str!("../../tests/fixtures/a381_fixed.toml"),
            include_str!("../../tests/fixtures/ax381_fixed.toml"),
        ] {
            let mut problem = Problem::from_toml(source).unwrap();
            problem.solver.engine = "monotone".into();
            problem.solver.ordering = "brown".into();
            let n = normalize(&problem, &problem.variables).unwrap();
            let mut p = n.split[0].clone();
            let mut steps = Vec::new();
            for name in ["x3", "x5"] {
                let variable = problem.variables.iter().position(|v| v == name).unwrap();
                let step = super::super::candidate(&n, &p, variable).unwrap().unwrap();
                let evidence = step.conditional_linear.as_ref().unwrap();
                assert!(step.linear.is_none());
                verify(&n, &p, variable, evidence).unwrap();
                p = coefficient(&p, variable, 0);
                steps.push(step);
            }
            let order = ["x0", "x2", "x4", "x6", "x8", "x5", "x3"]
                .map(String::from)
                .to_vec();
            let base = super::super::check_steps(&problem, &order, &steps).unwrap();
            assert_eq!(base.variables.len(), 5);

            // Scale fixing removes x0 from the support; discovery must also
            // try the affine multiplier 1+x2+x8, not only homogeneous sums.
            let anchored = n.split[0].replace(0, &Rational::one());
            let variable = problem.variables.iter().position(|v| v == "x3").unwrap();
            let step = super::super::candidate(&n, &anchored, variable)
                .unwrap()
                .unwrap();
            verify(
                &n,
                &anchored,
                variable,
                step.conditional_linear.as_ref().unwrap(),
            )
            .unwrap();

            let anchor = problem.variables.iter().position(|v| v == "x8").unwrap();
            let mut anchored = n.split[0].replace(anchor, &Rational::one());
            for name in ["x3", "x5"] {
                let axis = problem.variables.iter().position(|v| v == name).unwrap();
                let step = super::super::candidate(&n, &anchored, axis)
                    .unwrap()
                    .unwrap();
                verify(
                    &n,
                    &anchored,
                    axis,
                    step.conditional_linear.as_ref().unwrap(),
                )
                .unwrap();
                anchored = coefficient(&anchored, axis, 0);
            }
        }
    }
}
