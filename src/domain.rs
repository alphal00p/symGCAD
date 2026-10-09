//! Exact, conservative nonvanishing certificates on supersets of the domain.
//!
//! Bounds come exclusively from original strict coordinate-affine constraints.
//! Failure to find a certificate means "unknown", never a sign assertion.

use crate::algebra::{Algebra, Poly, rational};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use symbolica::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordinateBounds {
    pub lower: Option<String>,
    pub upper: Option<String>,
    pub lower_constraint: Option<usize>,
    pub upper_constraint: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Domain {
    pub variables: Vec<String>,
    pub bounds: Vec<CoordinateBounds>,
    pub excluded_sign_constraints: Vec<usize>,
    pub empty: bool,
}

/// Each polynomial in original_constraints represents p > 0. The caller must
/// explicitly exclude target-sign constraints if these should not narrow a
/// shared F-positive/F-negative domain. Derived projection factors are not input.
pub fn derive_domain(
    algebra: &Algebra,
    original_constraints: &[Poly],
    excluded_sign_constraints: &[usize],
) -> Result<Domain> {
    ensure!(
        original_constraints
            .iter()
            .all(|p| p.variables() == &algebra.variables),
        "domain constraints have a different variable map"
    );
    let excluded: BTreeSet<_> = excluded_sign_constraints.iter().copied().collect();
    ensure!(
        excluded.iter().all(|&i| i < original_constraints.len()),
        "excluded constraint index out of range"
    );
    let mut bounds = vec![CoordinateBounds::default(); algebra.names.len()];
    for (index, p) in original_constraints.iter().enumerate() {
        if excluded.contains(&index) || p.is_constant() {
            continue;
        }
        let support: Vec<_> = (0..algebra.names.len())
            .filter(|&i| p.degree(i) > 0)
            .collect();
        if support.len() != 1 || p.degree(support[0]) != 1 {
            continue;
        }
        let variable = support[0];
        let mut exponent = vec![0; algebra.names.len()];
        exponent[variable] = 1;
        let slope = p
            .coefficient(&exponent)
            .expect("nonzero linear coefficient");
        let endpoint = -p.get_constant() / slope.clone();
        let bound = &mut bounds[variable];
        if slope.is_negative() {
            let tighter = match &bound.upper {
                None => true,
                Some(old) => endpoint < rational(old)?,
            };
            if tighter {
                bound.upper = Some(endpoint.to_string());
                bound.upper_constraint = Some(index);
            }
        } else {
            let tighter = match &bound.lower {
                None => true,
                Some(old) => endpoint > rational(old)?,
            };
            if tighter {
                bound.lower = Some(endpoint.to_string());
                bound.lower_constraint = Some(index);
            }
        }
    }
    let empty = bounds.iter().try_fold(false, |empty, b| -> Result<_> {
        Ok(empty
            || match (&b.lower, &b.upper) {
                (Some(l), Some(u)) => rational(l)? >= rational(u)?,
                _ => false,
            })
    })?;
    Ok(Domain {
        variables: algebra.names.clone(),
        bounds,
        excluded_sign_constraints: excluded.into_iter().collect(),
        empty,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertificateOptions {
    pub max_terms: usize,
    pub polya_order: u32,
    pub bernstein_depth: usize,
    pub bernstein_nodes: usize,
    pub bernstein_coefficients: usize,
    pub work_limit: usize,
}

impl Default for CertificateOptions {
    fn default() -> Self {
        Self {
            max_terms: 20_000,
            polya_order: 4,
            bernstein_depth: 4,
            bernstein_nodes: 63,
            bernstein_coefficients: 4096,
            work_limit: 1_000_000,
        }
    }
}

impl CertificateOptions {
    fn validate(&self) -> Result<()> {
        ensure!(
            self.max_terms > 0 && self.max_terms <= 100_000,
            "certificate term budget must be in 1..=100000"
        );
        ensure!(
            self.polya_order <= 4,
            "Polya multiplier order is capped at four"
        );
        ensure!(
            self.bernstein_depth <= 8 && self.bernstein_nodes > 0 && self.bernstein_nodes <= 511,
            "Bernstein subdivision is capped at depth eight and 511 nodes"
        );
        ensure!(
            self.bernstein_coefficients > 0 && self.bernstein_coefficients <= 65_536,
            "Bernstein coefficient budget must be in 1..=65536"
        );
        ensure!(
            self.work_limit > 0 && self.work_limit <= 10_000_000,
            "certificate arithmetic budget must be in 1..=10000000"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BernsteinProof {
    Leaf,
    /// Both children include the new interior seam. Original strict endpoints
    /// remain excluded, so boundary zeros do not invalidate an interior proof.
    Bisect {
        variable: usize,
        left: Box<BernsteinProof>,
        right: Box<BernsteinProof>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CertificateMethod {
    ShiftedCoefficients,
    Polya { order: u32 },
    Bernstein { proof: BernsteinProof },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainCertificate {
    pub version: u32,
    pub polynomial: String,
    pub domain: Domain,
    /// The sign on the entire certified domain, always +1 or -1.
    pub sign: i8,
    pub method: CertificateMethod,
    pub limits: CertificateOptions,
}

pub fn certify_nonzero(
    algebra: &Algebra,
    polynomial: &Poly,
    domain: &Domain,
    options: &CertificateOptions,
) -> Result<Option<DomainCertificate>> {
    options.validate()?;
    validate_domain(algebra, domain)?;
    ensure!(
        polynomial.variables() == &algebra.variables,
        "certificate polynomial has a different variable map"
    );
    if domain.empty || polynomial.is_zero() || polynomial.nterms() > options.max_terms {
        return Ok(None);
    }
    let make = |sign, method| DomainCertificate {
        version: 1,
        polynomial: algebra.format(polynomial),
        domain: domain.clone(),
        sign,
        method,
        limits: options.clone(),
    };
    if let Some((shifted, positive)) =
        shift_to_positive(algebra, polynomial, domain, options.max_terms)?
    {
        if let Some(sign) = coefficient_sign(&shifted, &positive) {
            return Ok(Some(make(sign, CertificateMethod::ShiftedCoefficients)));
        }
        let support = support(&shifted);
        if !support.is_empty()
            && support.iter().all(|&v| positive[v])
            && homogeneous_degree(&shifted, &support).is_some()
        {
            let sum = algebra.parse(
                &support
                    .iter()
                    .map(|&v| algebra.names[v].as_str())
                    .collect::<Vec<_>>()
                    .join("+"),
            )?;
            let mut multiplied = shifted;
            for order in 1..=options.polya_order {
                if multiplied.nterms().saturating_mul(sum.nterms()) > options.max_terms {
                    break;
                }
                multiplied = &multiplied * &sum;
                if let Some(sign) = coefficient_sign(&multiplied, &positive) {
                    return Ok(Some(make(sign, CertificateMethod::Polya { order })));
                }
            }
        }
    }
    if let Some((unit, active)) = to_unit_box(algebra, polynomial, domain, options.max_terms)? {
        let sample = vec![Rational::from((1, 2)); algebra.names.len()];
        let value = unit.replace_all(&sample);
        if value.is_zero() {
            return Ok(None);
        }
        let sign = if value.is_negative() { -1 } else { 1 };
        let state = BoxState::unit(active.len());
        let mut budget = ProofBudget::new(options);
        if let Some(proof) = build_bernstein(
            algebra,
            &unit,
            &active,
            &state,
            sign,
            0,
            options,
            &mut budget,
        )? {
            return Ok(Some(make(sign, CertificateMethod::Bernstein { proof })));
        }
    }
    Ok(None)
}

/// Check the expected polynomial, independently reconstruct the domain from
/// original constraints, then recompute every exact coefficient witness.
pub fn verify_certificate(
    algebra: &Algebra,
    polynomial: &Poly,
    original_constraints: &[Poly],
    excluded_sign_constraints: &[usize],
    certificate: &DomainCertificate,
) -> Result<()> {
    ensure!(
        certificate.version == 1,
        "unknown domain certificate version"
    );
    ensure!(
        certificate.sign == 1 || certificate.sign == -1,
        "invalid certified sign"
    );
    certificate.limits.validate()?;
    ensure!(
        algebra.parse(&certificate.polynomial)? == *polynomial,
        "certificate is for a different polynomial"
    );
    let domain = derive_domain(algebra, original_constraints, excluded_sign_constraints)?;
    ensure!(
        certificate.domain == domain && !domain.empty,
        "certificate domain does not match the original bounds"
    );
    let valid = match &certificate.method {
        CertificateMethod::ShiftedCoefficients => {
            shift_to_positive(algebra, polynomial, &domain, certificate.limits.max_terms)?
                .is_some_and(|(p, positive)| {
                    coefficient_sign(&p, &positive) == Some(certificate.sign)
                })
        }
        CertificateMethod::Polya { order } => {
            ensure!(
                *order > 0 && *order <= certificate.limits.polya_order,
                "invalid Polya order"
            );
            let Some((mut p, positive)) =
                shift_to_positive(algebra, polynomial, &domain, certificate.limits.max_terms)?
            else {
                anyhow::bail!("Polya affine transform exceeds certificate budget");
            };
            let active = support(&p);
            ensure!(
                !active.is_empty()
                    && active.iter().all(|&v| positive[v])
                    && homogeneous_degree(&p, &active).is_some(),
                "Polya polynomial is not homogeneous on positive coordinates"
            );
            let sum = algebra.parse(
                &active
                    .iter()
                    .map(|&v| algebra.names[v].as_str())
                    .collect::<Vec<_>>()
                    .join("+"),
            )?;
            for _ in 0..*order {
                ensure!(
                    p.nterms().saturating_mul(sum.nterms()) <= certificate.limits.max_terms,
                    "Polya term budget exceeded"
                );
                p = &p * &sum;
            }
            coefficient_sign(&p, &positive) == Some(certificate.sign)
        }
        CertificateMethod::Bernstein { proof } => {
            let Some((unit, active)) =
                to_unit_box(algebra, polynomial, &domain, certificate.limits.max_terms)?
            else {
                anyhow::bail!(
                    "Bernstein certificate requires finite rational bounds on its support"
                );
            };
            let mut budget = ProofBudget::new(&certificate.limits);
            check_bernstein(
                algebra,
                &unit,
                &active,
                &BoxState::unit(active.len()),
                certificate.sign,
                proof,
                0,
                &certificate.limits,
                &mut budget,
            )?
        }
    };
    ensure!(valid, "exact domain nonvanishing certificate failed");
    Ok(())
}

fn validate_domain(algebra: &Algebra, domain: &Domain) -> Result<()> {
    ensure!(
        domain.variables == algebra.names && domain.bounds.len() == algebra.names.len(),
        "domain has a different variable map"
    );
    let mut empty = false;
    for b in &domain.bounds {
        let lower = b.lower.as_deref().map(rational).transpose()?;
        let upper = b.upper.as_deref().map(rational).transpose()?;
        if let (Some(l), Some(u)) = (lower, upper) {
            empty |= l >= u;
        }
    }
    ensure!(domain.empty == empty, "inconsistent empty-domain flag");
    Ok(())
}

fn support(p: &Poly) -> Vec<usize> {
    (0..p.nvars()).filter(|&v| p.degree(v) > 0).collect()
}

fn affine_replace(
    algebra: &Algebra,
    p: &Poly,
    variable: usize,
    offset: &Rational,
    scale: &Rational,
    limit: usize,
) -> Result<Option<Poly>> {
    if p.degree(variable) == 0 {
        return Ok(Some(p.clone()));
    }
    let bound = if offset.is_zero() {
        p.nterms()
    } else {
        p.exponents_iter().fold(0usize, |sum, m| {
            sum.saturating_add(m[variable] as usize + 1)
        })
    };
    if bound > limit {
        return Ok(None);
    }
    let replacement = algebra
        .parse(&algebra.names[variable])?
        .mul_coeff(scale.clone())
        .add_constant(offset.clone());
    Ok(Some(p.replace_with_poly(variable, &replacement)))
}

fn shift_to_positive(
    algebra: &Algebra,
    polynomial: &Poly,
    domain: &Domain,
    limit: usize,
) -> Result<Option<(Poly, Vec<bool>)>> {
    let mut p = polynomial.clone();
    let mut positive = vec![false; algebra.names.len()];
    for (v, bounds) in domain.bounds.iter().enumerate() {
        let transform = if let Some(lower) = &bounds.lower {
            Some((rational(lower)?, Rational::one()))
        } else if let Some(upper) = &bounds.upper {
            Some((rational(upper)?, Rational::from(-1)))
        } else {
            None
        };
        if let Some((offset, scale)) = transform {
            positive[v] = true;
            let Some(next) = affine_replace(algebra, &p, v, &offset, &scale, limit)? else {
                return Ok(None);
            };
            p = next;
        }
    }
    Ok(Some((p, positive)))
}

/// Nonnegative monomials alone do not give strict positivity: at least one
/// signed term must stay strictly positive even when unrestricted variables
/// vanish. This excludes x^2 on R but accepts 1+x^2 and x^2 on x>0.
fn coefficient_sign(p: &Poly, positive: &[bool]) -> Option<i8> {
    if p.is_zero() {
        return None;
    }
    let sign = if p.coefficients[0].is_negative() {
        -1
    } else {
        1
    };
    let mut strict_term = false;
    for (c, m) in p.coefficients.iter().zip(p.exponents_iter()) {
        if c.is_negative() != (sign < 0) {
            return None;
        }
        if m.iter()
            .enumerate()
            .any(|(v, e)| !positive[v] && e % 2 != 0)
        {
            return None;
        }
        strict_term |= m.iter().enumerate().all(|(v, e)| positive[v] || *e == 0);
    }
    strict_term.then_some(sign)
}

pub fn homogeneous_degree(p: &Poly, variables: &[usize]) -> Option<u32> {
    if variables.iter().any(|&v| v >= p.nvars())
        || variables.iter().copied().collect::<BTreeSet<_>>().len() != variables.len()
    {
        return None;
    }
    let mut degree = None;
    for m in p.exponents_iter() {
        let current = variables
            .iter()
            .try_fold(0u32, |sum, &v| sum.checked_add(m[v]))?;
        if degree.is_some_and(|d| d != current) {
            return None;
        }
        degree = Some(current);
    }
    Some(degree.unwrap_or(0))
}

fn to_unit_box(
    algebra: &Algebra,
    polynomial: &Poly,
    domain: &Domain,
    limit: usize,
) -> Result<Option<(Poly, Vec<usize>)>> {
    let active = support(polynomial);
    if active.is_empty() {
        return Ok(None);
    }
    let mut p = polynomial.clone();
    for &v in &active {
        let (Some(lower), Some(upper)) = (&domain.bounds[v].lower, &domain.bounds[v].upper) else {
            return Ok(None);
        };
        let lower = rational(lower)?;
        let width = rational(upper)? - lower.clone();
        if width <= Rational::zero() {
            return Ok(None);
        }
        let Some(next) = affine_replace(algebra, &p, v, &lower, &width, limit)? else {
            return Ok(None);
        };
        p = next;
    }
    Ok(Some((p, active)))
}

#[derive(Clone)]
struct BoxState {
    lower: Vec<Rational>,
    upper: Vec<Rational>,
    lower_open: Vec<bool>,
    upper_open: Vec<bool>,
}
impl BoxState {
    fn unit(n: usize) -> Self {
        Self {
            lower: vec![Rational::zero(); n],
            upper: vec![Rational::one(); n],
            lower_open: vec![true; n],
            upper_open: vec![true; n],
        }
    }
    fn bisect(&self, axis: usize) -> (Self, Self) {
        let midpoint = (&self.lower[axis] + &self.upper[axis]) / Rational::from(2);
        let (mut left, mut right) = (self.clone(), self.clone());
        left.upper[axis] = midpoint.clone();
        left.upper_open[axis] = false;
        right.lower[axis] = midpoint;
        right.lower_open[axis] = false;
        (left, right)
    }
}

struct ProofBudget {
    work: usize,
    nodes: usize,
}
impl ProofBudget {
    fn new(options: &CertificateOptions) -> Self {
        Self {
            work: options.work_limit,
            nodes: options.bernstein_nodes,
        }
    }
    fn spend(&mut self, work: usize) -> bool {
        if work > self.work {
            self.work = 0;
            false
        } else {
            self.work -= work;
            true
        }
    }
}

fn bernstein_coefficients(
    p: &Poly,
    active: &[usize],
    options: &CertificateOptions,
    budget: &mut ProofBudget,
) -> Option<(Vec<Vec<u32>>, Vec<Rational>, Vec<u32>)> {
    let degrees: Vec<_> = active.iter().map(|&v| p.degree(v)).collect();
    let count = degrees
        .iter()
        .try_fold(1usize, |n, &d| n.checked_mul(d as usize + 1))?;
    if count > options.bernstein_coefficients {
        return None;
    }
    let mut indices = Vec::with_capacity(count);
    let mut coefficients = Vec::with_capacity(count);
    for i in 0..count {
        let mut remaining = i;
        let beta: Vec<_> = degrees
            .iter()
            .map(|&d| {
                let index = (remaining % (d as usize + 1)) as u32;
                remaining /= d as usize + 1;
                index
            })
            .collect();
        let mut b = Rational::zero();
        for (c, alpha) in p.coefficients.iter().zip(p.exponents_iter()) {
            if !budget.spend(active.len().max(1)) {
                return None;
            }
            if active.iter().enumerate().any(|(j, &v)| alpha[v] > beta[j]) {
                continue;
            }
            let mut term = c.clone();
            for (j, &v) in active.iter().enumerate() {
                for k in 0..alpha[v] {
                    if !budget.spend(1) {
                        return None;
                    }
                    term *= Rational::from(((beta[j] - k) as u64, (degrees[j] - k) as u64));
                }
            }
            b += term;
        }
        indices.push(beta);
        coefficients.push(b);
    }
    Some((indices, coefficients, degrees))
}

// It suffices to inspect the most constrained included faces: choose an
// endpoint index for each axis with an included endpoint, and leave originally
// open axes free. A witness Bernstein basis function on each such face stays
// positive on every less constrained face above it, including the box interior.
fn strict_faces(
    indices: &[Vec<u32>],
    coefficients: &[Rational],
    degrees: &[u32],
    state: &BoxState,
    fixed: &mut Vec<Option<u32>>,
    axis: usize,
    budget: &mut ProofBudget,
) -> bool {
    if axis == degrees.len() {
        for (index, coefficient) in indices.iter().zip(coefficients) {
            if !budget.spend(degrees.len().max(1)) {
                return false;
            }
            if !coefficient.is_zero()
                && fixed
                    .iter()
                    .zip(index)
                    .all(|(fixed, index)| fixed.is_none_or(|value| value == *index))
            {
                return true;
            }
        }
        return false;
    }
    let mut choices = Vec::new();
    if !state.lower_open[axis] {
        choices.push(Some(0));
    }
    if !state.upper_open[axis] {
        choices.push(Some(degrees[axis]));
    }
    if choices.is_empty() {
        choices.push(None);
    }
    for choice in choices {
        fixed.push(choice);
        let valid = strict_faces(
            indices,
            coefficients,
            degrees,
            state,
            fixed,
            axis + 1,
            budget,
        );
        fixed.pop();
        if !valid {
            return false;
        }
    }
    true
}

fn bernstein_leaf(
    algebra: &Algebra,
    unit: &Poly,
    active: &[usize],
    state: &BoxState,
    sign: i8,
    options: &CertificateOptions,
    budget: &mut ProofBudget,
) -> Result<bool> {
    let mut p = unit.clone();
    for (axis, &v) in active.iter().enumerate() {
        let width = &state.upper[axis] - &state.lower[axis];
        let Some(next) = affine_replace(
            algebra,
            &p,
            v,
            &state.lower[axis],
            &width,
            options.max_terms,
        )?
        else {
            return Ok(false);
        };
        p = next;
    }
    let Some((indices, coefficients, degrees)) =
        bernstein_coefficients(&p, active, options, budget)
    else {
        return Ok(false);
    };
    if coefficients
        .iter()
        .any(|c| !c.is_zero() && c.is_negative() != (sign < 0))
    {
        return Ok(false);
    }
    Ok(strict_faces(
        &indices,
        &coefficients,
        &degrees,
        state,
        &mut Vec::new(),
        0,
        budget,
    ))
}

fn build_bernstein(
    algebra: &Algebra,
    unit: &Poly,
    active: &[usize],
    state: &BoxState,
    sign: i8,
    depth: usize,
    options: &CertificateOptions,
    budget: &mut ProofBudget,
) -> Result<Option<BernsteinProof>> {
    if budget.nodes == 0 || budget.work == 0 {
        return Ok(None);
    }
    budget.nodes -= 1;
    if bernstein_leaf(algebra, unit, active, state, sign, options, budget)? {
        return Ok(Some(BernsteinProof::Leaf));
    }
    if depth >= options.bernstein_depth || active.is_empty() {
        return Ok(None);
    }
    let axis = depth % active.len();
    let (left, right) = state.bisect(axis);
    let Some(left) = build_bernstein(
        algebra,
        unit,
        active,
        &left,
        sign,
        depth + 1,
        options,
        budget,
    )?
    else {
        return Ok(None);
    };
    let Some(right) = build_bernstein(
        algebra,
        unit,
        active,
        &right,
        sign,
        depth + 1,
        options,
        budget,
    )?
    else {
        return Ok(None);
    };
    Ok(Some(BernsteinProof::Bisect {
        variable: active[axis],
        left: Box::new(left),
        right: Box::new(right),
    }))
}

fn check_bernstein(
    algebra: &Algebra,
    unit: &Poly,
    active: &[usize],
    state: &BoxState,
    sign: i8,
    proof: &BernsteinProof,
    depth: usize,
    options: &CertificateOptions,
    budget: &mut ProofBudget,
) -> Result<bool> {
    if depth > options.bernstein_depth || budget.nodes == 0 || budget.work == 0 {
        return Ok(false);
    }
    budget.nodes -= 1;
    match proof {
        BernsteinProof::Leaf => bernstein_leaf(algebra, unit, active, state, sign, options, budget),
        BernsteinProof::Bisect {
            variable,
            left,
            right,
        } => {
            let Some(axis) = active.iter().position(|v| v == variable) else {
                return Ok(false);
            };
            let (left_box, right_box) = state.bisect(axis);
            Ok(check_bernstein(
                algebra,
                unit,
                active,
                &left_box,
                sign,
                left,
                depth + 1,
                options,
                budget,
            )? && check_bernstein(
                algebra,
                unit,
                active,
                &right_box,
                sign,
                right,
                depth + 1,
                options,
                budget,
            )?)
        }
    }
}

/// An explicit projective normalization proposal. This helper does not change
/// solver behavior; reconstruction and cylindrical ordering must be checked by
/// the caller before an anchor is enabled.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomogeneousAnchor {
    pub anchor: String,
    pub ratio_variables: Vec<String>,
    pub degrees: Vec<u32>,
    pub normalized_polynomials: Vec<String>,
    /// Ratio coordinates are x_i / x_anchor; scale is the original x_anchor>0.
    pub forward_ratios: Vec<String>,
    /// Original x_i = scale * ratio_i, for all non-anchor Feynman coordinates.
    pub inverse_coordinates: Vec<String>,
}

pub fn homogeneous_anchor(
    algebra: &Algebra,
    all_constraints_and_splits: &[Poly],
    feynman_variables: &[usize],
    anchor: usize,
    domain: &Domain,
) -> Result<Option<HomogeneousAnchor>> {
    validate_domain(algebra, domain)?;
    let subset: BTreeSet<_> = feynman_variables.iter().copied().collect();
    ensure!(
        subset.len() == feynman_variables.len() && subset.iter().all(|&v| v < algebra.names.len()),
        "invalid Feynman variable subset"
    );
    ensure!(
        subset.contains(&anchor),
        "anchor must be a Feynman variable"
    );
    ensure!(
        all_constraints_and_splits
            .iter()
            .all(|p| p.variables() == &algebra.variables),
        "homogeneity input has a different variable map"
    );
    if domain.empty {
        return Ok(None);
    }
    let Some(lower) = &domain.bounds[anchor].lower else {
        return Ok(None);
    };
    if rational(lower)? < Rational::zero() {
        return Ok(None);
    }
    let Some(degrees) = all_constraints_and_splits
        .iter()
        .map(|p| homogeneous_degree(p, feynman_variables))
        .collect::<Option<Vec<_>>>()
    else {
        return Ok(None);
    };
    let ratios: Vec<_> = feynman_variables
        .iter()
        .copied()
        .filter(|&v| v != anchor)
        .collect();
    Ok(Some(HomogeneousAnchor {
        anchor: algebra.names[anchor].clone(),
        ratio_variables: ratios.iter().map(|&v| algebra.names[v].clone()).collect(),
        degrees,
        normalized_polynomials: all_constraints_and_splits
            .iter()
            .map(|p| algebra.format(&p.replace(anchor, &Rational::one())))
            .collect(),
        forward_ratios: ratios
            .iter()
            .map(|&v| format!("{}/{}", algebra.names[v], algebra.names[anchor]))
            .collect(),
        inverse_coordinates: ratios
            .iter()
            .map(|&v| format!("{}*{}", algebra.names[anchor], algebra.names[v]))
            .collect(),
    }))
}

/// Homogenize a ratio-coordinate boundary q into r^d q(x_i/r). For r>0
/// the root order is preserved by multiplication of the root coordinate by r.
/// This algebraic identity alone does not establish a complete cell cover.
pub fn restore_homogeneous_bound(
    polynomial: &Poly,
    ratio_variables: &[usize],
    anchor: usize,
) -> Result<Poly> {
    let subset: BTreeSet<_> = ratio_variables.iter().copied().collect();
    ensure!(
        anchor < polynomial.nvars() && polynomial.degree(anchor) == 0,
        "reduced boundary must be independent of the scale anchor"
    );
    ensure!(
        subset.len() == ratio_variables.len()
            && !subset.contains(&anchor)
            && subset.iter().all(|&v| v < polynomial.nvars()),
        "invalid ratio variable subset"
    );
    let degrees = polynomial
        .exponents_iter()
        .map(|m| {
            ratio_variables
                .iter()
                .try_fold(0u32, |sum, &v| sum.checked_add(m[v]))
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| anyhow::anyhow!("homogenized exponent overflow"))?;
    let degree = degrees.iter().copied().max().unwrap_or(0);
    ensure!(
        u64::from(degree) <= crate::algebra::MAX_DEGREE,
        "homogenized exponent exceeds the supported polynomial range"
    );
    let mut result = polynomial.zero();
    for ((coefficient, exponent), term_degree) in polynomial
        .coefficients
        .iter()
        .zip(polynomial.exponents_iter())
        .zip(degrees)
    {
        let mut exponent = exponent.to_vec();
        exponent[anchor] = degree - term_degree;
        result.append_monomial(coefficient.clone(), &exponent);
    }
    Ok(result)
}
