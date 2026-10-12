//! Fixed separators between independently delineable positive root branches.
//!
//! Each different-owner edge of the complete pair ordering is guarded by both
//! evaluations at a rational separator. No branch can cross that separator on
//! a connected ancestor cell. Same-owner inequalities follow from individual
//! delineability, so the entire chain is preserved without a pair resultant.
use super::{PositivePairWitness, prefix_value, relevant_roots};
use crate::{
    algebra::{Algebra, Poly, rational},
    roots::{isolate_union, roots_in_open_interval, sector_sample},
    solver::sign,
};
use anyhow::{Result, ensure};
use std::collections::HashMap;
use symbolica::prelude::*;

/// Registry IDs are append-only canonical polynomials with a fixed main axis.
/// Still key the axis explicitly. Admission survives all tentative failures,
/// ancestor restarts and cache evictions: a pair can introduce only two finite
/// evaluation sets during the entire Builder lifetime.
#[derive(Default)]
pub(super) struct WitnessStock {
    entries: HashMap<(usize, usize, usize), Vec<PositivePairWitness>>,
    admitted: usize,
    reused: usize,
    budget_fallbacks: usize,
    /// Discovery-only admission policy; replay never consults it.
    policy: AdmissionPolicy,
    admitted_by_axis: HashMap<usize, usize>,
    cheap_resultant_declines: usize,
    axis_budget_declines: usize,
}

/// Separator guards p(s), q(s) are NEW sample-dependent polynomials. Each one
/// must be projected at lower levels and can pair with other cuts, so admitting
/// templates where the sample-independent resultant is small multiplies the cut
/// sets instead of saving work. Declining a template only selects the ordinary
/// resultant, which is always an admissible (stronger) certificate.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct AdmissionPolicy {
    /// Admit a new template only if the dense monomial bound of Res(p,q)
    /// exceeds this value. Zero admits every eligible pair (historical).
    pub min_resultant_terms: u64,
    /// Lifetime cap on newly admitted templates per lifting axis. Zero means
    /// unlimited (historical); the per-pair stock of two always applies.
    pub axis_budget: usize,
}

/// Dense upper bound on the number of monomials of Res_x(p,q), from the
/// Bezout-type total-degree bound deg_x(p)*tdeg'(q) + deg_x(q)*tdeg'(p), where
/// tdeg' ignores x, over the variables other than x that occur in p or q.
/// This is only a scheduling estimate; it never enters a certificate.
pub(super) fn resultant_term_bound(p: &Poly, q: &Poly, variable: usize) -> u64 {
    let rest = |f: &Poly| {
        f.exponents_iter()
            .map(|e| {
                e.iter()
                    .enumerate()
                    .filter(|&(i, _)| i != variable)
                    .map(|(_, &d)| u64::from(d))
                    .sum::<u64>()
            })
            .max()
            .unwrap_or(0)
    };
    let degree = u128::from(p.degree(variable))
        .saturating_mul(u128::from(rest(q)))
        .saturating_add(u128::from(q.degree(variable)).saturating_mul(u128::from(rest(p))));
    let others = (0..p.nvars())
        .filter(|&i| i != variable && (p.degree(i) > 0 || q.degree(i) > 0))
        .count() as u128;
    // binomial(degree + others, others), saturating at u64::MAX.
    let mut bound: u128 = 1;
    for k in 1..=others {
        bound = bound.saturating_mul(degree.saturating_add(k)) / k;
        if bound >= u128::from(u64::MAX) {
            return u64::MAX;
        }
    }
    bound as u64
}

impl WitnessStock {
    pub(super) fn with_policy(policy: AdmissionPolicy) -> Self {
        Self {
            policy,
            ..Self::default()
        }
    }

    pub(super) fn attempt(
        &mut self,
        algebra: &Algebra,
        polynomials: &[Poly],
        ids: [usize; 2],
        variable: usize,
        prefix: &[Rational],
    ) -> Result<Option<(PositivePairWitness, Vec<Poly>)>> {
        let ids = [ids[0].min(ids[1]), ids[0].max(ids[1])];
        if !eligible(polynomials, ids, variable) {
            return Ok(None);
        }
        let templates = self.entries.entry((variable, ids[0], ids[1])).or_default();
        for template in templates.iter() {
            if let Some(proof) = validate(algebra, polynomials, variable, prefix, template)? {
                self.reused += 1;
                return Ok(Some(proof));
            }
        }
        if templates.len() >= 2 {
            self.budget_fallbacks += 1;
            return Ok(None);
        }
        if self.policy.min_resultant_terms > 0
            && resultant_term_bound(&polynomials[ids[0]], &polynomials[ids[1]], variable)
                <= self.policy.min_resultant_terms
        {
            self.cheap_resultant_declines += 1;
            return Ok(None);
        }
        let axis_admitted = self.admitted_by_axis.entry(variable).or_default();
        if self.policy.axis_budget > 0 && *axis_admitted >= self.policy.axis_budget {
            self.axis_budget_declines += 1;
            return Ok(None);
        }
        let proof = discover(algebra, polynomials, ids, variable, prefix)?;
        if let Some((proof, _)) = &proof {
            // Consume the slot before a caller attempts lower-level guards.
            templates.push(proof.clone());
            self.admitted += 1;
            *axis_admitted += 1;
        }
        Ok(proof)
    }

    pub(super) fn report(&self, report: &mut super::LocalReport) {
        report.pair_witness_templates = self.admitted;
        report.pair_witness_template_reuses = self.reused;
        report.pair_witness_budget_fallbacks = self.budget_fallbacks;
        report.pair_witness_cheap_resultant_declines = self.cheap_resultant_declines;
        report.pair_witness_axis_budget_declines = self.axis_budget_declines;
    }
}

fn eligible(polynomials: &[Poly], ids: [usize; 2], variable: usize) -> bool {
    ids[0] < ids[1]
        && ids.iter().all(|&id| {
            polynomials.get(id).is_some_and(|p| {
                p.degree(variable) > 0 && p.degree(variable) <= 32 && p.nterms() <= 4096
            })
        })
}

fn fibers(
    algebra: &Algebra,
    polynomials: &[Poly],
    ids: [usize; 2],
    variable: usize,
    prefix: &[Rational],
) -> Result<Option<Vec<UnivariatePolynomial<Q>>>> {
    if !eligible(polynomials, ids, variable) {
        return Ok(None);
    }
    let fibers = ids
        .iter()
        .map(|&id| algebra.specialize_univariate(&polynomials[id], variable, prefix))
        .collect::<Result<Vec<_>>>()?;
    if fibers.iter().zip(ids).any(|(f, id)| {
        f.is_zero()
            || f.degree() != polynomials[id].degree(variable) as usize
            || f.evaluate(&Rational::zero()).is_zero()
    }) {
        return Ok(None);
    }
    Ok(Some(fibers))
}

fn positive_count(fiber: &UnivariatePolynomial<Q>) -> usize {
    let lc = fiber.coefficients().last().unwrap().abs();
    // Strict Cauchy bound; neither endpoint is a root by the prerequisites.
    let bound = fiber
        .coefficients()
        .iter()
        .take(fiber.degree())
        .map(|c| c.abs() / &lc)
        .max()
        .unwrap_or_else(Rational::zero)
        + Rational::from(2);
    roots_in_open_interval(fiber, &Rational::zero(), &bound)
}

/// Validate supplied separators without rerunning their discovery or trusting
/// stock history. Direct Sturm counts identify every edge's precise positive
/// ordinals; evaluation signs alone would not establish this at a new sample.
fn validate(
    algebra: &Algebra,
    polynomials: &[Poly],
    variable: usize,
    prefix: &[Rational],
    supplied: &PositivePairWitness,
) -> Result<Option<(PositivePairWitness, Vec<Poly>)>> {
    let ids = supplied.polynomials;
    let Some(fibers) = fibers(algebra, polynomials, ids, variable, prefix)? else {
        return Ok(None);
    };
    let counts = [positive_count(&fibers[0]), positive_count(&fibers[1])];
    if counts.contains(&0)
        || counts.iter().sum::<usize>() > 16
        || counts != supplied.positive_counts
        || supplied.branch_order.len() != counts.iter().sum::<usize>()
    {
        return Ok(None);
    }
    let mut seen = [0, 0];
    for &(id, ordinal) in &supplied.branch_order {
        let Some(owner) = ids.iter().position(|&candidate| candidate == id) else {
            return Ok(None);
        };
        if ordinal != seen[owner] || ordinal >= counts[owner] {
            return Ok(None);
        }
        seen[owner] += 1;
    }
    if seen != counts {
        return Ok(None);
    }
    let edges = supplied
        .branch_order
        .windows(2)
        .filter(|pair| pair[0].0 != pair[1].0)
        .collect::<Vec<_>>();
    if edges.is_empty() || edges.len() > 8 || supplied.separators.len() != edges.len() {
        return Ok(None);
    }
    let values = supplied
        .separators
        .iter()
        .map(|s| rational(s))
        .collect::<Result<Vec<_>>>()?;
    if values[0] <= Rational::zero()
        || values.windows(2).any(|w| w[0] >= w[1])
        || values.iter().any(|value| {
            value.numerator_ref().significant_bits() + value.denominator_ref().significant_bits()
                > 256
        })
    {
        return Ok(None);
    }
    let mut evaluation_signs = Vec::new();
    let mut guards = Vec::new();
    for (edge, value) in edges.iter().zip(values) {
        let left_owner = usize::from(edge[0].0 != ids[0]);
        let right_owner = 1 - left_owner;
        let signs = [
            sign(&fibers[0].evaluate(&value)),
            sign(&fibers[1].evaluate(&value)),
        ];
        if signs.contains(&0)
            || roots_in_open_interval(&fibers[left_owner], &Rational::zero(), &value)
                != edge[0].1 + 1
            || roots_in_open_interval(&fibers[right_owner], &Rational::zero(), &value) != edge[1].1
        {
            return Ok(None);
        }
        evaluation_signs.push(signs);
        for &id in &ids {
            let guard = polynomials[id].replace(variable, &value);
            ensure!(
                !prefix_value(&guard, prefix)?.is_zero(),
                "pair evaluation disagrees with exact fiber"
            );
            guards.push(guard);
        }
    }
    let mut expected = supplied.clone();
    expected.evaluation_signs = evaluation_signs;
    Ok(Some((expected, guards)))
}

pub(super) fn verify_witness(
    algebra: &Algebra,
    polynomials: &[Poly],
    ids: [usize; 2],
    variable: usize,
    prefix: &[Rational],
    supplied: &PositivePairWitness,
) -> Result<Vec<Poly>> {
    let ids = [ids[0].min(ids[1]), ids[0].max(ids[1])];
    ensure!(
        supplied.polynomials == ids,
        "pair witness names the wrong cuts"
    );
    let Some((expected, guards)) = validate(algebra, polynomials, variable, prefix, supplied)?
    else {
        anyhow::bail!("pair separators do not certify the complete positive branch order");
    };
    ensure!(supplied == &expected, "pair evaluation signs were modified");
    Ok(guards)
}

fn discover(
    algebra: &Algebra,
    polynomials: &[Poly],
    ids: [usize; 2],
    variable: usize,
    prefix: &[Rational],
) -> Result<Option<(PositivePairWitness, Vec<Poly>)>> {
    let Some(fibers) = fibers(algebra, polynomials, ids, variable, prefix)? else {
        return Ok(None);
    };
    let counts = [positive_count(&fibers[0]), positive_count(&fibers[1])];
    if counts.contains(&0) || counts.iter().sum::<usize>() > 16 {
        return Ok(None);
    }
    let union = relevant_roots(isolate_union(ids.into_iter().zip(fibers).collect())?, true)?;
    if union.len() != counts.iter().sum::<usize>() {
        // Merging loses an owner at a shared positive root. Ordinary resultant
        // refinement must resolve that coincidence; never infer its ordering.
        return Ok(None);
    }
    let mut seen = [0, 0];
    let branch_order = union
        .iter()
        .map(|root| {
            let owner = usize::from(root.polynomial != ids[0]);
            let ordinal = seen[owner];
            seen[owner] += 1;
            (root.polynomial, ordinal)
        })
        .collect();
    let separators = union
        .windows(2)
        .filter(|pair| pair[0].polynomial != pair[1].polynomial)
        .map(|pair| sector_sample(Some(&pair[0]), Some(&pair[1])).to_string())
        .collect::<Vec<_>>();
    if separators.len() > 8 {
        return Ok(None);
    }
    validate(
        algebra,
        polynomials,
        variable,
        prefix,
        &PositivePairWitness {
            polynomials: ids,
            positive_counts: counts,
            branch_order,
            separators,
            evaluation_signs: Vec::new(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(expressions: [&str; 2]) -> (Algebra, Vec<Poly>) {
        let algebra = Algebra::new(&["a".into(), "z".into()]).unwrap();
        let polynomials = expressions
            .iter()
            .map(|p| algebra.parse(p).unwrap())
            .collect();
        (algebra, polynomials)
    }

    #[test]
    fn interleaving_needs_every_cross_owner_edge_and_positive_ordinal() {
        let (algebra, polynomials) = setup(["(z-1)*(z-3)", "(z-2)*(z-a)"]);
        let prefix = [Rational::from(4)];
        let (proof, guards) = discover(&algebra, &polynomials, [0, 1], 1, &prefix)
            .unwrap()
            .unwrap();
        assert_eq!(proof.positive_counts, [2, 2]);
        assert_eq!(proof.branch_order, vec![(0, 0), (1, 0), (0, 1), (1, 1)]);
        assert_eq!(proof.separators.len(), 3);
        assert_eq!(guards.len(), 6);
        verify_witness(&algebra, &polynomials, [0, 1], 1, &prefix, &proof).unwrap();
        // One separator between roots 1 and 2 cannot prevent the moving root
        // crossing root 3; the complete chain rejects this truncated evidence.
        let mut bad = proof.clone();
        bad.separators.truncate(1);
        bad.evaluation_signs.truncate(1);
        assert!(verify_witness(&algebra, &polynomials, [0, 1], 1, &prefix, &bad).is_err());
        assert!(
            validate(
                &algebra,
                &polynomials,
                1,
                &[rational("5/2").unwrap()],
                &proof
            )
            .unwrap()
            .is_none()
        );
        for change in 0..6 {
            let mut bad = proof.clone();
            match change {
                0 => bad.branch_order[2] = bad.branch_order[0],
                1 => bad.positive_counts[0] += 1,
                2 => bad.separators[0] = "1".into(),
                3 => bad.evaluation_signs[0][0] *= -1,
                4 => bad.separators.swap(0, 1),
                _ => bad.branch_order[0].0 = 99,
            }
            assert!(verify_witness(&algebra, &polynomials, [0, 1], 1, &prefix, &bad).is_err());
        }
    }

    #[test]
    fn common_negative_roots_are_harmless_but_shared_positive_roots_decline() {
        let (algebra, polynomials) = setup(["z^2-1", "z^2-z-2"]);
        let (proof, _) = discover(&algebra, &polynomials, [0, 1], 1, &[Rational::zero()])
            .unwrap()
            .unwrap();
        assert_eq!(proof.positive_counts, [1, 1]);
        assert_eq!(proof.branch_order, vec![(0, 0), (1, 0)]);
        assert_eq!(proof.separators, vec!["3/2"]);
        let (_, shared) = setup(["z^2-1", "z^2+z-2"]);
        assert!(
            discover(&algebra, &shared, [0, 1], 1, &[Rational::zero()])
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn lifetime_pair_stock_is_finite_and_rechecks_each_reused_edge() {
        let (algebra, polynomials) = setup(["(z-a)^2-2", "(z-a-1)^2-3"]);
        let mut stock = WitnessStock::default();
        let first = stock
            .attempt(&algebra, &polynomials, [1, 0], 1, &[Rational::from(3)])
            .unwrap()
            .unwrap();
        assert!(
            stock
                .attempt(&algebra, &polynomials, [0, 1], 1, &[Rational::from(3)])
                .unwrap()
                .is_some()
        );
        let nearby = stock
            .attempt(
                &algebra,
                &polynomials,
                [0, 1],
                1,
                &[rational("301/100").unwrap()],
            )
            .unwrap()
            .unwrap();
        assert_eq!(nearby.0.separators, first.0.separators);
        verify_witness(
            &algebra,
            &polynomials,
            [0, 1],
            1,
            &[rational("301/100").unwrap()],
            &nearby.0,
        )
        .unwrap();
        assert!(
            stock
                .attempt(&algebra, &polynomials, [0, 1], 1, &[Rational::from(10)])
                .unwrap()
                .is_some()
        );
        assert!(
            stock
                .attempt(&algebra, &polynomials, [0, 1], 1, &[Rational::from(100)])
                .unwrap()
                .is_none()
        );
        assert_eq!(stock.admitted, 2);
        assert_eq!(stock.reused, 2);
        assert_eq!(stock.budget_fallbacks, 1);
        // Discovery history is absent from independent checking.
        verify_witness(
            &algebra,
            &polynomials,
            [0, 1],
            1,
            &[Rational::from(3)],
            &first.0,
        )
        .unwrap();
    }

    #[test]
    fn caps_and_zero_endpoint_use_ordinary_pair_fallback() {
        for expressions in [
            ["z^33-1", "z^2-2"],
            ["z*(z-1)", "z^2-2"],
            ["z^2+1", "z^2-2"],
            [
                "(z-1)*(z-3)*(z-5)*(z-7)*(z-9)",
                "(z-2)*(z-4)*(z-6)*(z-8)*(z-10)",
            ],
            [
                "(z-1)*(z-2)*(z-3)*(z-4)*(z-5)*(z-6)*(z-7)*(z-8)*(z-9)",
                "(z-10)*(z-11)*(z-12)*(z-13)*(z-14)*(z-15)*(z-16)*(z-17)",
            ],
        ] {
            let (algebra, polynomials) = setup(expressions);
            assert!(
                discover(&algebra, &polynomials, [0, 1], 1, &[Rational::zero()])
                    .unwrap()
                    .is_none()
            );
        }
    }
}
