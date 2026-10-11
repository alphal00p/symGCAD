//! Bounded exact Descartes covers of the positive ray.
use super::{PositiveRayCase, prefix_value};
use crate::{
    algebra::{Algebra, Poly, canonical, rational},
    roots::{isolate_union, refine_once, sector_sample},
    solver::sign,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use symbolica::prelude::*;

const MAX_DEGREE: usize = 32;
// Independent of configurable SOURCE admission. Increasing source admission
// must never increase the cost bounds on the resulting exact guard set.
const MAX_GUARD_TERMS: usize = 4096;
const MAX_COEFFICIENT_SLOTS: usize = 256;
const MAX_GUARDS: usize = 256;
const MAX_COEFFICIENT_BITS: u64 = 512;
const MAX_SEPARATOR_BITS: u64 = 256;
const MAX_INTERVALS: usize = 64;
const MAX_DEPTH: usize = 32;
const MAX_ROOTS: usize = 8;
const MAX_ZERO_REFINEMENTS: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum PositiveIntervalEndpoint {
    Zero,
    Rational(String),
    PositiveInfinity,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositiveInterval {
    pub lower: PositiveIntervalEndpoint,
    pub upper: PositiveIntervalEndpoint,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositiveIntervalWitness {
    pub interval: PositiveInterval,
    /// All degree+1 transformed coefficients, in ascending exponent order.
    /// Zero means identically zero, never a vanishing nonzero sample guard.
    pub coefficient_signs: Vec<i8>,
}

/// A separate stock from global saturated witnesses: a bounded interval
/// budget (default two) plus two global templates per polynomial. Both are permanent
/// for this Builder, including failed tentative stacks. Downward-variable
/// induction therefore still gives a finite set of possible projection guards.
pub(super) struct WitnessStock {
    limit: usize,
    max_terms: usize,
    max_total_terms: usize,
    entries: HashMap<usize, Vec<Vec<PositiveInterval>>>,
    admitted: usize,
    reused: usize,
    budget_fallbacks: usize,
    source_term_fallbacks: usize,
}

impl Default for WitnessStock {
    fn default() -> Self {
        Self {
            limit: 2,
            max_terms: crate::config::DEFAULT_DESCARTES_INTERVAL_MAX_TERMS,
            max_total_terms: crate::config::DEFAULT_DESCARTES_INTERVAL_MAX_TOTAL_TERMS,
            entries: HashMap::new(),
            admitted: 0,
            reused: 0,
            budget_fallbacks: 0,
            source_term_fallbacks: 0,
        }
    }
}

impl WitnessStock {
    #[cfg(test)]
    pub(super) fn new(limit: usize) -> Result<Self> {
        Self::with_max_terms(limit, crate::config::DEFAULT_DESCARTES_INTERVAL_MAX_TERMS)
    }
    #[cfg(test)]
    pub(super) fn with_max_terms(limit: usize, max_terms: usize) -> Result<Self> {
        Self::with_budgets(
            limit,
            max_terms,
            crate::config::DEFAULT_DESCARTES_INTERVAL_MAX_TOTAL_TERMS,
        )
    }
    pub(super) fn with_budgets(
        limit: usize,
        max_terms: usize,
        max_total_terms: usize,
    ) -> Result<Self> {
        ensure!(
            (2..=8).contains(&limit),
            "interval template limit must be between two and eight"
        );
        crate::config::validate_descartes_interval_max_terms(max_terms)?;
        crate::config::validate_descartes_interval_max_total_terms(max_total_terms)?;
        Ok(Self {
            limit,
            max_terms,
            max_total_terms,
            ..Self::default()
        })
    }
    pub(super) fn attempt(
        &mut self,
        id: usize,
        algebra: &Algebra,
        p: &Poly,
        variable: usize,
        prefix: &[Rational],
    ) -> Result<Option<(PositiveRayCase, Vec<Poly>)>> {
        if p.nterms() > self.max_terms {
            self.source_term_fallbacks += 1;
            return Ok(None);
        }
        if !eligible(p, variable, self.max_terms) {
            return Ok(None);
        }
        let templates = self.entries.entry(id).or_default();
        for template in templates.iter() {
            if let Some(proof) = at_template(
                p,
                variable,
                prefix,
                template,
                self.max_terms,
                self.max_total_terms,
            )? {
                self.reused += 1;
                return Ok(Some(proof));
            }
        }
        if templates.len() >= self.limit {
            self.budget_fallbacks += 1;
            return Ok(None);
        }
        let Some(template) = discover(algebra, p, variable, prefix, self.max_terms)? else {
            return Ok(None);
        };
        let proof = at_template(
            p,
            variable,
            prefix,
            &template,
            self.max_terms,
            self.max_total_terms,
        )?;
        if proof.is_some() {
            templates.push(template);
            self.admitted += 1;
        }
        Ok(proof)
    }

    pub(super) fn report(&self, report: &mut super::LocalReport) {
        report.descartes_interval_templates = self.admitted;
        report.descartes_interval_template_limit = self.limit;
        report.descartes_interval_template_reuses = self.reused;
        report.descartes_interval_budget_fallbacks = self.budget_fallbacks;
        report.descartes_interval_source_term_fallbacks = self.source_term_fallbacks;
    }
}

fn eligible(p: &Poly, variable: usize, max_terms: usize) -> bool {
    (3..=MAX_DEGREE).contains(&(p.degree(variable) as usize)) && p.nterms() <= max_terms
}
fn height(x: &Rational) -> u64 {
    x.numerator_ref().significant_bits() + x.denominator_ref().significant_bits()
}
fn variations(signs: &[i8]) -> usize {
    signs
        .iter()
        .copied()
        .filter(|&s| s != 0)
        .fold((0, 0), |(last, n), s| {
            (s, n + usize::from(last != 0 && last != s))
        })
        .1
}
fn choose(n: usize, k: usize) -> u64 {
    if k > n {
        return 0;
    }
    let mut c = 1;
    for i in 1..=k.min(n - k) {
        c = c * (n + 1 - i) as u64 / i as u64;
    }
    c
}
fn powers(x: &Rational, d: usize) -> Vec<Rational> {
    let mut powers = vec![Rational::one()];
    for i in 1..=d {
        powers.push(&powers[i - 1] * x);
    }
    powers
}
/// Exact coefficient matrix for (1+u)^d p((a+b*u)/(1+u)), or p(a+u).
fn weights(d: usize, a: &Rational, b: Option<&Rational>) -> Vec<Vec<Rational>> {
    let ap = powers(a, d);
    let bp = b.map(|b| powers(b, d));
    let mut weights = vec![vec![Rational::zero(); d + 1]; d + 1];
    for (k, row) in weights.iter_mut().enumerate() {
        for (j, entry) in row.iter_mut().enumerate() {
            if let Some(bp) = &bp {
                for l in k.saturating_sub(d - j)..=j.min(k) {
                    *entry +=
                        (&ap[j - l] * &bp[l]) * Rational::from(choose(j, l) * choose(d - j, k - l));
                }
            } else if j >= k {
                *entry = &ap[j - k] * &Rational::from(choose(j, k));
            }
        }
    }
    weights
}

type NumericInterval = (Rational, Option<Rational>, usize);

/// Remove unnecessary discovery seams using only adjacent, already certified
/// intervals. Each original interval is visited once, so there are at most
/// N-1 union candidates. The symbolic checker still rebuilds every coefficient
/// of the resulting cover; this is only a bounded discovery heuristic.
fn coalesce_adjacent(
    f: &UnivariatePolynomial<Q>,
    intervals: Vec<NumericInterval>,
) -> (Vec<NumericInterval>, usize) {
    if f.is_zero() {
        return (intervals, 0);
    }
    let mut out: Vec<NumericInterval> = Vec::with_capacity(intervals.len());
    let mut attempts = 0;
    for next in intervals {
        let Some(previous) = out.last_mut() else {
            out.push(next);
            continue;
        };
        attempts += 1;
        let count = previous.2.saturating_add(next.2);
        // Even a seam that has no root must not join two one-root pieces:
        // the replayed interval theorem certifies only zero or one root.
        let adjacent = previous.1.as_ref() == Some(&next.0);
        let ordered = previous.0 >= Rational::zero()
            && next.0 > previous.0
            && next.1.as_ref().is_none_or(|b| *b > next.0);
        if count > 1
            || !adjacent
            || !ordered
            || f.evaluate(&previous.0).is_zero()
            || f.evaluate(&next.0).is_zero()
            || next.1.as_ref().is_some_and(|b| f.evaluate(b).is_zero())
        {
            out.push(next);
            continue;
        }
        let values = weights(f.degree(), &previous.0, next.1.as_ref())
            .iter()
            .map(|row| {
                row.iter()
                    .zip(f.coefficients())
                    .fold(Rational::zero(), |sum, (w, c)| sum + w * c)
            })
            .collect::<Vec<_>>();
        // A zero at this sample could be a nonzero symbolic coefficient that
        // nullifies. Conservatively keep the old pieces rather than proposing
        // a cover that the independent coefficient checker must decline.
        if values.iter().any(|c| c.is_zero())
            || variations(&values.iter().map(sign).collect::<Vec<_>>()) != count
        {
            out.push(next);
            continue;
        }
        previous.1 = next.1;
        previous.2 = count;
    }
    (out, attempts)
}

/// Zero and infinity have unique typed representations. Finite rationals must
/// be strictly positive; equality between neighbors is exact, not string-based.
fn cover(
    template: &[PositiveInterval],
    degree: usize,
) -> Result<Option<Vec<(Rational, Option<Rational>)>>> {
    if template.is_empty()
        || template.len() > MAX_INTERVALS
        || template.len().saturating_mul(degree + 1) > MAX_COEFFICIENT_SLOTS
    {
        return Ok(None);
    }
    let mut out = Vec::new();
    let mut previous = Rational::zero();
    for (i, interval) in template.iter().enumerate() {
        let a = match &interval.lower {
            PositiveIntervalEndpoint::Zero if i == 0 => Rational::zero(),
            PositiveIntervalEndpoint::Rational(s) if i != 0 => rational(s)?,
            _ => return Ok(None),
        };
        if a != previous || (i != 0 && a <= Rational::zero()) || height(&a) > MAX_SEPARATOR_BITS {
            return Ok(None);
        }
        let b = match &interval.upper {
            PositiveIntervalEndpoint::PositiveInfinity if i + 1 == template.len() => None,
            PositiveIntervalEndpoint::Rational(s) if i + 1 < template.len() => Some(rational(s)?),
            _ => return Ok(None),
        };
        if let Some(b) = &b {
            if *b <= a || height(b) > MAX_SEPARATOR_BITS {
                return Ok(None);
            }
            previous = b.clone();
        }
        out.push((a, b));
    }
    Ok(Some(out))
}

struct Guards {
    polynomials: Vec<Poly>,
    seen: HashSet<Poly>,
    terms: usize,
    max_total_terms: usize,
}
impl Default for Guards {
    fn default() -> Self {
        Self {
            polynomials: Vec::new(),
            seen: HashSet::new(),
            terms: 0,
            max_total_terms: crate::config::DEFAULT_DESCARTES_INTERVAL_MAX_TOTAL_TERMS,
        }
    }
}
impl Guards {
    fn add(&mut self, p: &Poly) -> bool {
        let p = canonical(p);
        if self.seen.contains(&p) {
            return true;
        }
        if p.nterms() > MAX_GUARD_TERMS
            || p.coefficients
                .iter()
                .any(|c| height(c) > MAX_COEFFICIENT_BITS)
            || self.polynomials.len() >= MAX_GUARDS
            || self.terms.saturating_add(p.nterms()) > self.max_total_terms
        {
            return false;
        }
        self.terms += p.nterms();
        self.seen.insert(p.clone());
        self.polynomials.push(p);
        true
    }
}

/// The checker invokes this on supplied intervals; it never reproduces the
/// producer's isolation or its stock history. Every coefficient identity is
/// reconstructed exactly from the original polynomial's coefficient list.
fn at_template(
    p: &Poly,
    variable: usize,
    prefix: &[Rational],
    template: &[PositiveInterval],
    max_terms: usize,
    max_total_terms: usize,
) -> Result<Option<(PositiveRayCase, Vec<Poly>)>> {
    crate::config::validate_descartes_interval_max_terms(max_terms)?;
    crate::config::validate_descartes_interval_max_total_terms(max_total_terms)?;
    if !eligible(p, variable, max_terms) {
        return Ok(None);
    }
    let degree = p.degree(variable) as usize;
    let Some(intervals) = cover(template, degree)? else {
        return Ok(None);
    };
    let mut guards = Guards {
        max_total_terms,
        ..Guards::default()
    };
    let endpoint = p.replace(variable, &Rational::zero());
    let leading = p.univariate_lcoeff(variable);
    // Keep these obligations explicit, including when coefficient collection
    // or structural simplification could otherwise omit a degree/endpoint.
    for guard in [&endpoint, &leading] {
        if prefix_value(guard, prefix)?.is_zero() || !guards.add(guard) {
            return Ok(None);
        }
    }
    let zero = p.zero();
    let mut coefficients = vec![zero.clone(); degree + 1];
    for (coefficient, exponent) in p.to_univariate_polynomial_list(variable) {
        coefficients[exponent as usize] = coefficient;
    }
    let mut witnesses = Vec::new();
    let mut roots = 0;
    for ((a, b), interval) in intervals.iter().zip(template) {
        if let Some(b) = b {
            let guard = p.replace(variable, b);
            if prefix_value(&guard, prefix)?.is_zero() || !guards.add(&guard) {
                return Ok(None);
            }
        }
        let mut signs = Vec::new();
        for row in weights(degree, a, b.as_ref()) {
            let coefficient = row
                .iter()
                .zip(&coefficients)
                .filter(|(w, _)| !w.is_zero())
                .fold(zero.clone(), |sum, (w, c)| {
                    sum + c.clone().mul_coeff(w.clone())
                });
            if coefficient.is_zero() {
                signs.push(0);
                continue;
            }
            let s = sign(&prefix_value(&coefficient, prefix)?);
            if s == 0 || !guards.add(&coefficient) {
                return Ok(None);
            }
            signs.push(s);
        }
        let count = variations(&signs);
        if count > 1 {
            return Ok(None);
        }
        roots += count;
        if roots > MAX_ROOTS {
            return Ok(None);
        }
        witnesses.push(PositiveIntervalWitness {
            interval: interval.clone(),
            coefficient_signs: signs,
        });
    }
    Ok(Some((
        PositiveRayCase::DescartesIntervals {
            intervals: witnesses,
        },
        guards.polynomials,
    )))
}

pub(super) fn positive_count(intervals: &[PositiveIntervalWitness]) -> usize {
    intervals
        .iter()
        .map(|i| variations(&i.coefficient_signs))
        .sum()
}

pub(super) fn verify_witness(
    p: &Poly,
    variable: usize,
    prefix: &[Rational],
    supplied: &PositiveRayCase,
    max_terms: usize,
    max_total_terms: usize,
) -> Result<Vec<Poly>> {
    crate::config::validate_descartes_interval_max_terms(max_terms)?;
    crate::config::validate_descartes_interval_max_total_terms(max_total_terms)?;
    let PositiveRayCase::DescartesIntervals { intervals } = supplied else {
        anyhow::bail!("expected interval Descartes witness");
    };
    let template = intervals
        .iter()
        .map(|w| w.interval.clone())
        .collect::<Vec<_>>();
    let Some((expected, guards)) =
        at_template(p, variable, prefix, &template, max_terms, max_total_terms)?
    else {
        anyhow::bail!("interval Descartes cover or coefficient signs are invalid");
    };
    ensure!(
        supplied == &expected,
        "interval Descartes coefficient signs were modified"
    );
    Ok(guards)
}

fn discover(
    algebra: &Algebra,
    p: &Poly,
    variable: usize,
    prefix: &[Rational],
    max_terms: usize,
) -> Result<Option<Vec<PositiveInterval>>> {
    crate::config::validate_descartes_interval_max_terms(max_terms)?;
    if !eligible(p, variable, max_terms) {
        return Ok(None);
    }
    let f = algebra.specialize_univariate(p, variable, prefix)?;
    let degree = p.degree(variable) as usize;
    if f.is_zero() || f.degree() != degree || f.evaluate(&Rational::zero()).is_zero() {
        return Ok(None);
    }
    let mut roots = isolate_union(vec![(0, f.clone())])?;
    let mut zero_refinements = 0;
    for root in &mut roots {
        while root.interval.0 <= Rational::zero() && root.interval.1 >= Rational::zero() {
            if zero_refinements == MAX_ZERO_REFINEMENTS {
                return Ok(None);
            }
            refine_once(root);
            zero_refinements += 1;
        }
    }
    roots.retain(|r| r.interval.0 > Rational::zero());
    if roots.len() > MAX_ROOTS {
        return Ok(None);
    }
    let mut boundaries = vec![Rational::zero()];
    boundaries.extend(
        roots
            .windows(2)
            .map(|r| sector_sample(Some(&r[0]), Some(&r[1]))),
    );
    let mut pending = VecDeque::new();
    for (i, a) in boundaries.iter().enumerate() {
        pending.push_back((a.clone(), boundaries.get(i + 1).cloned(), 0));
    }
    let mut accepted = Vec::new();
    let mut count = 0;
    while let Some((a, b, depth)) = pending.pop_front() {
        if height(&a) > MAX_SEPARATOR_BITS
            || b.as_ref().is_some_and(|b| height(b) > MAX_SEPARATOR_BITS)
        {
            return Ok(None);
        }
        if (accepted.len() + pending.len() + 1) * (degree + 1) > MAX_COEFFICIENT_SLOTS {
            return Ok(None);
        }
        let values = weights(degree, &a, b.as_ref())
            .iter()
            .map(|row| {
                row.iter()
                    .zip(f.coefficients())
                    .fold(Rational::zero(), |sum, (w, c)| sum + w * c)
            })
            .collect::<Vec<_>>();
        let signs = values.iter().map(sign).collect::<Vec<_>>();
        let v = variations(&signs);
        if v <= 1 {
            count += v;
            accepted.push((a, b, v));
            continue;
        }
        if accepted.len() + pending.len() + 2 > MAX_INTERVALS || depth >= MAX_DEPTH {
            return Ok(None);
        }
        let mut cut = if let Some(b) = &b {
            (&a + b) / Rational::from(2)
        } else {
            let after = roots
                .last()
                .map(|r| sector_sample(Some(r), None))
                .unwrap_or_else(Rational::one);
            if after > a {
                after
            } else {
                &a * &Rational::from(2) + Rational::one()
            }
        };
        let mut shifts = 0;
        while f.evaluate(&cut).is_zero() && shifts < MAX_DEPTH {
            cut = (&a + &cut) / Rational::from(2);
            shifts += 1;
        }
        if height(&cut) > MAX_SEPARATOR_BITS || f.evaluate(&cut).is_zero() {
            return Ok(None);
        }
        pending.push_back((a, Some(cut.clone()), depth + 1));
        pending.push_back((cut, b, depth + 1));
    }
    if count != roots.len() {
        return Ok(None);
    }
    accepted.sort_by(|a, b| a.0.cmp(&b.0));
    let original_intervals = accepted.len();
    let (accepted, attempts) = coalesce_adjacent(&f, accepted);
    debug_assert!(attempts <= original_intervals.saturating_sub(1));
    debug_assert_eq!(accepted.iter().map(|i| i.2).sum::<usize>(), count);
    Ok(Some(
        accepted
            .into_iter()
            .map(|(a, b, _)| PositiveInterval {
                lower: if a.is_zero() {
                    PositiveIntervalEndpoint::Zero
                } else {
                    PositiveIntervalEndpoint::Rational(a.to_string())
                },
                upper: b
                    .map(|b| PositiveIntervalEndpoint::Rational(b.to_string()))
                    .unwrap_or(PositiveIntervalEndpoint::PositiveInfinity),
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(text: &str) -> (Algebra, Poly) {
        let a = Algebra::new(&["s".into(), "z".into()]).unwrap();
        let p = a.parse(text).unwrap();
        (a, p)
    }

    fn numeric_interval(a: i64, b: Option<i64>, count: usize) -> NumericInterval {
        (Rational::from(a), b.map(Rational::from), count)
    }

    fn univariate(text: &str) -> UnivariatePolynomial<Q> {
        let (a, p) = setup(text);
        a.specialize_univariate(&p, 1, &[Rational::zero()]).unwrap()
    }

    #[test]
    fn adjacent_zero_one_one_zero_and_zero_zero_pieces_coalesce_exactly() {
        let f = univariate("(z-2)*(z+1)*(z+3)");
        for pieces in [
            vec![
                numeric_interval(0, Some(1), 0),
                numeric_interval(1, None, 1),
            ],
            vec![
                numeric_interval(0, Some(3), 1),
                numeric_interval(3, None, 0),
            ],
        ] {
            let (merged, attempts) = coalesce_adjacent(&f, pieces);
            assert_eq!(merged, vec![numeric_interval(0, None, 1)]);
            assert_eq!(attempts, 1);
        }
        let f = univariate("(z+1)*(z+2)*(z+3)");
        let pieces = (0..8)
            .map(|i| numeric_interval(i, (i != 7).then_some(i + 1), 0))
            .collect::<Vec<_>>();
        let (merged, attempts) = coalesce_adjacent(&f, pieces);
        assert_eq!(merged, vec![numeric_interval(0, None, 0)]);
        assert_eq!(attempts, 7); // One pass, including repeated accumulator merges.
    }

    #[test]
    fn coalescence_rejects_multiple_roots_and_unsaturated_union_bounds() {
        let two_roots = vec![
            numeric_interval(0, Some(2), 1),
            numeric_interval(2, None, 1),
        ];
        assert_eq!(
            coalesce_adjacent(&univariate("(z-1)*(z-3)*(z+1)"), two_roots.clone()).0,
            two_roots
        );
        // Each piece has zero variations, but the union has two. The true
        // root count alone is not sufficient for the supplied Descartes proof.
        let rootless = vec![
            numeric_interval(0, Some(1), 0),
            numeric_interval(1, None, 0),
        ];
        assert_eq!(
            coalesce_adjacent(&univariate("(z+2)*(z^2-z+1)"), rootless.clone()).0,
            rootless
        );
    }

    #[test]
    fn coalescence_keeps_strict_endpoints_adjacency_and_nullifying_coefficients() {
        let pieces = vec![
            numeric_interval(0, Some(1), 0),
            numeric_interval(1, None, 0),
        ];
        // Root-valued old seams must not be removed on untrusted count input.
        assert_eq!(
            coalesce_adjacent(&univariate("(z-1)*(z+1)*(z+2)"), pieces.clone()).0,
            pieces
        );
        let endpoint = vec![
            numeric_interval(0, Some(1), 0),
            numeric_interval(1, Some(2), 0),
        ];
        assert_eq!(
            coalesce_adjacent(&univariate("(z-2)*(z+1)*(z+3)"), endpoint.clone()).0,
            endpoint
        );
        let gap = vec![
            numeric_interval(0, Some(1), 0),
            numeric_interval(2, None, 0),
        ];
        assert_eq!(
            coalesce_adjacent(&univariate("(z+1)*(z+2)*(z+3)"), gap.clone()).0,
            gap
        );
        // At s=0 the union's z^2 coefficient vanishes, although the original
        // multivariate coefficient s is nonzero. Keep the valid smaller pieces.
        assert_eq!(
            coalesce_adjacent(&univariate("z^3+s*z^2+z+1"), pieces.clone()).0,
            pieces
        );
    }

    #[test]
    fn coalesced_cover_rebuilds_full_map_guards_and_rejects_tampering() {
        let a = Algebra::new(&["s".into(), "z".into(), "unused".into()]).unwrap();
        let p = a.parse("(z-1)*(z-3)*(z+1)*(z+2)+s*z").unwrap();
        let prefix = [Rational::zero()];
        let f = a.specialize_univariate(&p, 1, &prefix).unwrap();
        let pieces = vec![
            (Rational::zero(), Some(rational("1/2").unwrap()), 0),
            (rational("1/2").unwrap(), Some(Rational::from(2)), 1),
            numeric_interval(2, Some(4), 1),
            numeric_interval(4, None, 0),
        ];
        let (merged, attempts) = coalesce_adjacent(&f, pieces);
        assert_eq!(attempts, 3);
        assert_eq!(
            merged,
            vec![
                numeric_interval(0, Some(2), 1),
                numeric_interval(2, None, 1)
            ]
        );
        let template = vec![
            PositiveInterval {
                lower: PositiveIntervalEndpoint::Zero,
                upper: PositiveIntervalEndpoint::Rational("2".into()),
            },
            PositiveInterval {
                lower: PositiveIntervalEndpoint::Rational("2".into()),
                upper: PositiveIntervalEndpoint::PositiveInfinity,
            },
        ];
        let (case, guards) = at_template(&p, 1, &prefix, &template, 4096, 65536)
            .unwrap()
            .unwrap();
        assert!(guards.iter().all(|g| g.variables() == &a.variables));
        for g in [
            p.univariate_lcoeff(1),
            p.replace(1, &Rational::zero()),
            p.replace(1, &Rational::from(2)),
        ] {
            assert!(guards.contains(&canonical(&g)));
        }
        let restored = serde_json::from_slice(&serde_json::to_vec(&case).unwrap()).unwrap();
        assert_eq!(
            guards,
            verify_witness(&p, 1, &prefix, &restored, 4096, 65536).unwrap()
        );
        let PositiveRayCase::DescartesIntervals { mut intervals } = restored else {
            panic!()
        };
        assert_eq!(positive_count(&intervals), 2);
        intervals[0].coefficient_signs[0] *= -1;
        assert!(
            verify_witness(
                &p,
                1,
                &prefix,
                &PositiveRayCase::DescartesIntervals { intervals },
                4096,
                65536
            )
            .is_err()
        );
        let mut invalid = template;
        invalid[0].upper = PositiveIntervalEndpoint::Rational("1".into());
        invalid[1].lower = PositiveIntervalEndpoint::Rational("1".into());
        assert!(
            at_template(&p, 1, &prefix, &invalid, 4096, 65536)
                .unwrap()
                .is_none()
        );
    }

    fn oversized_source(width: u32) -> (Algebra, Poly) {
        let algebra = Algebra::new(&["a".into(), "b".into(), "z".into(), "unused".into()]).unwrap();
        let mut support = algebra.parse("0").unwrap();
        for a in 0..width {
            for b in 0..width {
                support.append_monomial(Rational::one(), &[a, b, 0, 0]);
            }
        }
        // Primitive in z: gcd(support,a)=1. At a=0,b=1 the global
        // Descartes bound is unsaturated (four variations, two roots).
        let p = &(&support * &algebra.parse("(z-1)*(z-2)*(z^2-z+1)").unwrap())
            + &algebra.parse("a").unwrap();
        (algebra, p)
    }

    #[test]
    fn source_budget_reaches_discovery_reuse_and_every_replayed_guard() {
        let (a, p) = oversized_source(33);
        assert_eq!(p.nterms(), 5445);
        let prefix = [Rational::zero(), Rational::one()];
        let mut small = WitnessStock::default();
        assert!(small.attempt(0, &a, &p, 2, &prefix).unwrap().is_none());
        assert!(small.entries.is_empty());
        let mut large = WitnessStock::with_max_terms(2, 8192).unwrap();
        let (case, guards) = large.attempt(0, &a, &p, 2, &prefix).unwrap().unwrap();
        let restored: PositiveRayCase =
            serde_json::from_slice(&serde_json::to_vec(&case).unwrap()).unwrap();
        assert_eq!(
            guards,
            verify_witness(&p, 2, &prefix, &restored, 8192, 65536).unwrap()
        );
        assert!(verify_witness(&p, 2, &prefix, &restored, 4096, 65536).is_err());
        for invalid in [0, 65537, usize::MAX] {
            assert!(WitnessStock::with_max_terms(2, invalid).is_err());
            assert!(verify_witness(&p, 2, &prefix, &restored, invalid, 65536).is_err());
        }
        assert!(large.attempt(0, &a, &p, 2, &prefix).unwrap().is_some());
        assert_eq!((large.admitted, large.reused), (1, 1));
        // Even a preexisting template cannot bypass a lowered source bound.
        large.max_terms = 4096;
        assert!(large.attempt(0, &a, &p, 2, &prefix).unwrap().is_none());
        assert_eq!(large.source_term_fallbacks, 1);
        let mut report = super::super::LocalReport::default();
        large.report(&mut report);
        assert_eq!(report.descartes_interval_source_term_fallbacks, 1);
        assert_eq!(report.descartes_interval_budget_fallbacks, 0);
        assert!(guards.iter().all(|g| g.variables() == &a.variables));
        for g in [p.replace(2, &Rational::zero()), p.univariate_lcoeff(2)] {
            assert!(guards.contains(&canonical(&g)));
        }
        let PositiveRayCase::DescartesIntervals { intervals } = &case else {
            panic!()
        };
        let template = intervals
            .iter()
            .map(|i| i.interval.clone())
            .collect::<Vec<_>>();
        let cover = cover(&template, 4).unwrap().unwrap();
        let coefficients = p.to_univariate_polynomial_list(2);
        for (lower, upper) in cover {
            if let Some(b) = &upper {
                assert!(guards.contains(&canonical(&p.replace(2, b))));
            }
            for row in weights(4, &lower, upper.as_ref()) {
                let c = coefficients.iter().fold(p.zero(), |sum, (c, j)| {
                    sum + c.clone().mul_coeff(row[*j as usize].clone())
                });
                if !c.is_zero() {
                    assert!(guards.contains(&canonical(&c)));
                }
            }
        }
        for mutation in 0..4 {
            let mut bad = case.clone();
            let PositiveRayCase::DescartesIntervals { intervals } = &mut bad else {
                panic!()
            };
            match mutation {
                0 => {
                    intervals.pop();
                }
                1 => intervals[0].coefficient_signs[0] *= -1,
                2 => intervals[1].interval.lower = PositiveIntervalEndpoint::Rational("99".into()),
                _ => {
                    intervals[0].interval.upper = PositiveIntervalEndpoint::Rational("1".into());
                    intervals[1].interval.lower = PositiveIntervalEndpoint::Rational("1".into());
                }
            }
            assert!(verify_witness(&p, 2, &prefix, &bad, 8192, 65536).is_err());
        }
    }

    #[test]
    fn larger_source_budget_never_raises_the_fixed_per_guard_limit() {
        let (a, p) = oversized_source(65);
        assert_eq!(p.nterms(), 21125);
        assert!(eligible(&p, 2, 32768));
        let endpoint = p.replace(2, &Rational::zero());
        assert_eq!(endpoint.nterms(), 4225);
        assert!(!Guards::default().add(&endpoint));
        let mut stock = WitnessStock::with_max_terms(2, 32768).unwrap();
        let prefix = [Rational::zero(), Rational::one()];
        assert!(discover(&a, &p, 2, &prefix, 32768).unwrap().is_some());
        assert!(stock.attempt(0, &a, &p, 2, &prefix).unwrap().is_none());
        assert_eq!(stock.admitted, 0);
        assert_eq!(stock.source_term_fallbacks, 0);
        assert!(stock.entries[&0].is_empty());
    }

    #[test]
    fn total_guard_budget_is_bound_to_discovery_reuse_and_exact_replay() {
        let algebra = Algebra::new(&["a".into(), "b".into(), "z".into(), "unused".into()]).unwrap();
        let mut support = algebra.parse("0").unwrap();
        for i in 0..51 {
            for j in 0..51 {
                support.append_monomial(Rational::one(), &[i, j, 0, 0]);
            }
        }
        let p = &support * &algebra.parse("(z-1)*(z-2)*(z^2-z+1)*(z+1)^20").unwrap()
            + algebra.parse("a*z^12").unwrap();
        let prefix = [Rational::zero(), Rational::one()];
        assert!(p.nterms() <= 65536);
        let mut default_budget = WitnessStock::with_budgets(2, 65536, 65536).unwrap();
        assert!(
            default_budget
                .attempt(0, &algebra, &p, 2, &prefix)
                .unwrap()
                .is_none()
        );
        assert_eq!(default_budget.admitted, 0);
        let mut larger = WitnessStock::with_budgets(2, 65536, 131072).unwrap();
        let (proof, guards) = larger
            .attempt(0, &algebra, &p, 2, &prefix)
            .unwrap()
            .unwrap();
        let total: usize = guards.iter().map(Poly::nterms).sum();
        assert!(
            (65537..=131072).contains(&total),
            "actual unique guard terms {total}"
        );
        assert!(
            guards
                .iter()
                .all(|g| g.nterms() <= MAX_GUARD_TERMS && g.variables() == &algebra.variables)
        );
        assert!(
            larger
                .attempt(0, &algebra, &p, 2, &prefix)
                .unwrap()
                .is_some()
        );
        assert_eq!((larger.admitted, larger.reused), (1, 1));
        let restored: PositiveRayCase =
            serde_json::from_slice(&serde_json::to_vec(&proof).unwrap()).unwrap();
        assert_eq!(
            guards,
            verify_witness(&p, 2, &prefix, &restored, 65536, 131072).unwrap()
        );
        assert!(verify_witness(&p, 2, &prefix, &restored, 65536, total - 1).is_err());
        // A stored template cannot bypass a tighter total-term policy.
        larger.max_total_terms = 65536;
        assert!(
            larger
                .attempt(0, &algebra, &p, 2, &prefix)
                .unwrap()
                .is_none()
        );
        assert_eq!(larger.admitted, 1);
        for guard in [p.replace(2, &Rational::zero()), p.univariate_lcoeff(2)] {
            assert!(guards.contains(&canonical(&guard)));
        }
        let PositiveRayCase::DescartesIntervals { intervals } = &proof else {
            panic!()
        };
        for interval in intervals {
            if let PositiveIntervalEndpoint::Rational(s) = &interval.interval.upper {
                assert!(guards.contains(&canonical(&p.replace(2, &rational(s).unwrap()))));
            }
        }
        for invalid in [0, 262145, usize::MAX] {
            assert!(WitnessStock::with_budgets(2, 65536, invalid).is_err());
            assert!(verify_witness(&p, 2, &prefix, &proof, 65536, invalid).is_err());
        }
        let mut changed = proof;
        let PositiveRayCase::DescartesIntervals { intervals } = &mut changed else {
            panic!()
        };
        intervals[0].coefficient_signs[0] *= -1;
        assert!(verify_witness(&p, 2, &prefix, &changed, 65536, 131072).is_err());
        // Enlarging aggregate cost never enlarges the per-guard limit.
        let (_, oversized) = oversized_source(65);
        assert!(
            !Guards {
                max_total_terms: 262144,
                ..Guards::default()
            }
            .add(&oversized.replace(2, &Rational::zero()))
        );
    }

    #[test]
    fn oversized_interval_source_reaches_raw_projection_and_independent_replay() {
        use super::super::{
            PositiveRayProjection, ProjectionOperations, positive_projection_raw, strict_bezout,
        };
        struct NoExpensiveProjection;
        impl ProjectionOperations for NoExpensiveProjection {
            fn discriminant(&mut self, _: &Poly, _: usize) -> Result<Poly> {
                anyhow::bail!("ordinary discriminant fallback requested")
            }
            fn resultant(&mut self, _: &Poly, _: &Poly, _: usize) -> Result<Poly> {
                anyhow::bail!("unexpected pair resultant")
            }
        }
        let (algebra, p) = oversized_source(33);
        let coordinate = algebra.parse("z").unwrap();
        let polynomials = vec![coordinate.clone(), p.clone()];
        let normalized = crate::problem::NormalizedProblem {
            algebra,
            positives: vec![coordinate],
            split: vec![p.clone()],
        };
        let prefix = [Rational::zero(), Rational::one()];
        let invoke = |limit, replay: Option<&PositiveRayProjection>| {
            let mut stock = WitnessStock::with_max_terms(2, limit).unwrap();
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
                false,
                4096,
                None,
                true,
                limit,
                65536,
                Some(&mut stock),
                replay,
                false,
                &mut None,
                &mut 0.0,
                &mut NoExpensiveProjection,
            )
        };
        assert!(
            invoke(4096, None)
                .unwrap_err()
                .to_string()
                .contains("ordinary discriminant fallback")
        );
        let (bearing, guards, _, _, _, proof) = invoke(8192, None).unwrap();
        assert_eq!(bearing, vec![1]);
        let (_, replayed, _, _, _, expected) = invoke(8192, Some(&proof)).unwrap();
        assert_eq!(expected, proof);
        assert_eq!(replayed, guards);
        assert!(invoke(4096, Some(&proof)).is_err());
        assert!(guards.contains(&canonical(&p.replace(2, &Rational::zero()))));
        assert!(guards.contains(&canonical(&p.univariate_lcoeff(2))));
    }

    #[test]
    fn coefficient_matrices_satisfy_exact_symbolic_transform_identities() {
        let algebra = Algebra::new(&["s".into(), "z".into(), "u".into()]).unwrap();
        let p = algebra.parse("(s^2+1)*z^5-(s-2)*z^3+3/7*z-5").unwrap();
        let u = algebra.parse("u").unwrap();
        let one = algebra.parse("1").unwrap();
        let coefficients = p.to_univariate_polynomial_list(1);
        let a = Rational::from((2, 3));
        for b in [None, Some(Rational::from((13, 5)))] {
            let matrix = weights(5, &a, b.as_ref());
            let actual = matrix.iter().enumerate().fold(p.zero(), |sum, (k, row)| {
                let c = coefficients.iter().fold(p.zero(), |sum, (c, j)| {
                    sum + c.clone().mul_coeff(row[*j as usize].clone())
                });
                sum + c * &u.pow(k)
            });
            let expected = coefficients.iter().fold(p.zero(), |sum, (c, j)| {
                let numerator = one.clone().mul_coeff(a.clone())
                    + u.clone().mul_coeff(b.clone().unwrap_or_else(Rational::one));
                let mut term = c * &numerator.pow(*j as usize);
                if b.is_some() {
                    term = term * &(&one + &u).pow(5 - *j as usize);
                }
                sum + term
            });
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn unsaturated_and_rootless_fibers_have_exact_full_covers() {
        for (text, count) in [
            ("(z-1)*(z-2)*(z^2-z+s)", 2),
            ("(z-1)*(z-2)*(z-3)*(z^2-z+s)", 3),
            ("(z^2-z+s)*(z^2+1)", 0),
            ("(z+1)^2*(z^2-z+1)", 0),
            ("(z-1)*(z-2)*(z^2+3*z+9/4)", 2),
        ] {
            let (a, p) = setup(text);
            let (case, _) = WitnessStock::default()
                .attempt(0, &a, &p, 1, &[Rational::one()])
                .unwrap()
                .unwrap_or_else(|| panic!("unexpected fallback for {text}"));
            let PositiveRayCase::DescartesIntervals { intervals } = &case else {
                panic!()
            };
            assert_eq!(positive_count(intervals), count);
            assert!(verify_witness(&p, 1, &[Rational::one()], &case, 4096, 65536).is_ok());
        }
    }

    #[test]
    fn provided_cover_rejects_gaps_seams_omissions_and_sign_tampering() {
        let (a, p) = setup("(z-1)*(z-2)*(z^2-z+s)");
        let (case, _) = WitnessStock::default()
            .attempt(0, &a, &p, 1, &[Rational::one()])
            .unwrap()
            .unwrap();
        for which in 0..7 {
            let mut changed = case.clone();
            let PositiveRayCase::DescartesIntervals { intervals } = &mut changed else {
                panic!()
            };
            assert!(intervals.len() >= 2);
            match which {
                0 => {
                    intervals.remove(0);
                }
                1 => intervals[1].interval.lower = PositiveIntervalEndpoint::Rational("999".into()),
                2 => {
                    // A seam exactly at the original root z=1 must be guarded
                    // and rejected, even if the cover remains contiguous.
                    intervals[0].interval.upper = PositiveIntervalEndpoint::Rational("1".into());
                    intervals[1].interval.lower = PositiveIntervalEndpoint::Rational("1".into());
                }
                3 => intervals[0].coefficient_signs[0] *= -1,
                4 => {
                    intervals[0].coefficient_signs.pop();
                }
                5 => intervals[0].interval.lower = PositiveIntervalEndpoint::Rational("0".into()),
                _ => {
                    intervals.pop();
                }
            }
            assert!(
                verify_witness(&p, 1, &[Rational::one()], &changed, 4096, 65536).is_err(),
                "tamper {which}"
            );
        }
    }

    #[test]
    fn sample_zero_coefficients_degree_drop_and_repeated_positive_roots_decline() {
        let template = vec![PositiveInterval {
            lower: PositiveIntervalEndpoint::Zero,
            upper: PositiveIntervalEndpoint::PositiveInfinity,
        }];
        let (_, p) = setup("z^3+(s-1)*z^2+z+1");
        assert!(
            at_template(&p, 1, &[Rational::one()], &template, 4096, 65536)
                .unwrap()
                .is_none()
        );
        let (_, p) = setup("z^3+z+1");
        assert!(
            at_template(&p, 1, &[Rational::one()], &template, 4096, 65536)
                .unwrap()
                .is_some()
        );
        for text in ["(s-1)*z^4+z^3+1", "(z-1)^2*(z^2+1)"] {
            let (a, p) = setup(text);
            assert!(
                WitnessStock::default()
                    .attempt(0, &a, &p, 1, &[Rational::one()])
                    .unwrap()
                    .is_none()
            );
        }
    }

    #[test]
    fn permanent_stock_reuses_nearby_samples_then_falls_back_near_irrational_collision() {
        let (a, p) = setup("(z^2-2)^2-s");
        let mut stock = WitnessStock::default();
        assert!(
            stock
                .attempt(0, &a, &p, 1, &[Rational::from((1, 2))])
                .unwrap()
                .is_some()
        );
        assert!(
            stock
                .attempt(0, &a, &p, 1, &[Rational::from((5001, 10000))])
                .unwrap()
                .is_some()
        );
        assert_eq!(stock.reused, 1);
        let mut s = Rational::from((1, 2));
        let mut fallback = false;
        for _ in 0..32 {
            s /= Rational::from(16);
            if stock.attempt(0, &a, &p, 1, &[s.clone()]).unwrap().is_none() {
                fallback = true;
                break;
            }
        }
        assert!(fallback);
        assert_eq!(stock.entries[&0].len(), 2);
        assert!(stock.budget_fallbacks > 0);
    }

    #[test]
    fn enlarged_stock_extends_but_never_replenishes_permanent_admissions() {
        let (a, p) = setup("(z^2-2)^2-s");
        let mut small = WitnessStock::new(2).unwrap();
        let mut large = WitnessStock::new(8).unwrap();
        let mut s = Rational::from((1, 2));
        let mut extra_success = false;
        for _ in 0..40 {
            let small_proof = small.attempt(0, &a, &p, 1, &[s.clone()]).unwrap();
            let large_proof = large.attempt(0, &a, &p, 1, &[s.clone()]).unwrap();
            extra_success |= small_proof.is_none() && large_proof.is_some();
            if let Some((case, guards)) = large_proof {
                assert_eq!(
                    guards,
                    verify_witness(&p, 1, &[s.clone()], &case, 4096, 65536).unwrap()
                );
            }
            if large.budget_fallbacks > 0 {
                break;
            }
            s /= Rational::from(16);
        }
        assert!(extra_success);
        assert_eq!(small.entries[&0].len(), 2);
        assert_eq!(large.entries[&0].len(), 8);
        assert_eq!(large.admitted, 8);
        assert!(large.budget_fallbacks > 0);
        let retained = large.entries[&0].clone();
        for _ in 0..3 {
            assert!(large.attempt(0, &a, &p, 1, &[s.clone()]).unwrap().is_none());
        }
        assert_eq!(large.entries[&0], retained);
        // Old covers remain reusable after exhaustion, without replenishment.
        assert!(
            large
                .attempt(0, &a, &p, 1, &[Rational::from((1, 2))])
                .unwrap()
                .is_some()
        );
        assert_eq!(large.admitted, 8);
        assert!(large.reused > 0);
        assert!(WitnessStock::new(1).is_err());
        assert!(WitnessStock::new(9).is_err());
    }

    #[test]
    fn guard_and_template_caps_decline_without_admission() {
        let (a, p) = setup("z^33-z+1");
        let mut stock = WitnessStock::default();
        assert!(
            stock
                .attempt(0, &a, &p, 1, &[Rational::one()])
                .unwrap()
                .is_none()
        );
        assert_eq!(stock.admitted, 0);
        let template = (0..9)
            .map(|i| PositiveInterval {
                lower: if i == 0 {
                    PositiveIntervalEndpoint::Zero
                } else {
                    PositiveIntervalEndpoint::Rational(i.to_string())
                },
                upper: if i == 8 {
                    PositiveIntervalEndpoint::PositiveInfinity
                } else {
                    PositiveIntervalEndpoint::Rational((i + 1).to_string())
                },
            })
            .collect::<Vec<_>>();
        assert!(cover(&template, 32).unwrap().is_none());
        let (_, p) = setup("z^3-z+1");
        let too_tall = Rational::from(2).pow(600);
        let tall_guard = p.mul_coeff(too_tall) + a.parse("s").unwrap();
        assert!(!Guards::default().add(&tall_guard));
    }

    #[test]
    fn positive_ordinals_include_roots_owned_by_another_union_polynomial() {
        let a = Algebra::new(&["z".into()]).unwrap();
        let p = a.parse("(z-1)*(z-2)*(z-3)").unwrap();
        let (case, _) = WitnessStock::default()
            .attempt(1, &a, &p, 0, &[])
            .unwrap()
            .unwrap();
        let first = a.parse("z-1").unwrap();
        let roots = isolate_union(vec![
            (0, a.specialize_univariate(&first, 0, &[]).unwrap()),
            (1, a.specialize_univariate(&p, 0, &[]).unwrap()),
        ])
        .unwrap();
        let ray = super::super::PositiveRayProjection {
            constraint: 0,
            coordinate: 99,
            separations: vec![],
            pair_witnesses: vec![],
            strict_bezout: vec![],
            rules: vec![super::super::PositiveRayRule {
                polynomial: 1,
                case,
            }],
        };
        let bounds = super::super::local_bounds(&roots, Some(&ray)).unwrap();
        assert_eq!(
            bounds
                .iter()
                .filter(|b| b.polynomial == 1)
                .map(|b| (b.index_domain, b.index))
                .collect::<Vec<_>>(),
            vec![
                (crate::output::RootIndexDomain::Positive, 1),
                (crate::output::RootIndexDomain::Positive, 2)
            ]
        );
    }
}
