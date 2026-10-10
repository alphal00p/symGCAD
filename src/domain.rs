//! Exact, conservative nonvanishing certificates on supersets of the domain.
//!
//! Bounds come exclusively from original strict coordinate-affine constraints.
//! Failure to find a certificate means "unknown", never a sign assertion.

use crate::algebra::{Algebra, Poly, rational};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use symbolica::prelude::*;

mod quadratic_components;
mod weighted_squares;
pub use quadratic_components::QuadraticComponentSquare;
pub use weighted_squares::{WeightedResidualTerm, WeightedSquareTerm};

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
    /// Zero disables this independent portfolio member for ablations.
    #[serde(default = "default_sturm_degree")]
    pub univariate_sturm_degree: usize,
    /// Bounded binomial/affine-square attempts; zero preserves the older portfolio.
    #[serde(default)]
    pub square_residual_candidates: usize,
    /// Opt-in bounded positive-monomial-weighted binomial squares.
    #[serde(default, skip_serializing_if = "no_weighted_square_candidates")]
    pub weighted_square_candidates: usize,
    /// Opt-in exact PSD quadratic components with a strictly positive residual.
    #[serde(default, skip_serializing_if = "no_weighted_square_candidates")]
    pub quadratic_component_candidates: usize,
    /// Nonrecursive quadratic no-real-root attempts; historical proofs omit it.
    #[serde(default)]
    pub quadratic_no_real_roots_candidates: usize,
}

fn default_sturm_degree() -> usize {
    64
}

fn no_weighted_square_candidates(value: &usize) -> bool {
    *value == 0
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
            univariate_sturm_degree: default_sturm_degree(),
            square_residual_candidates: 16,
            weighted_square_candidates: 0,
            quadratic_component_candidates: 0,
            quadratic_no_real_roots_candidates: 4,
        }
    }
}

impl CertificateOptions {
    fn validate(&self) -> Result<()> {
        ensure!(
            self.quadratic_component_candidates <= 32,
            "quadratic component candidates are capped at 32"
        );
        ensure!(
            self.weighted_square_candidates <= 72,
            "weighted square candidates are capped at 72"
        );
        ensure!(
            self.quadratic_no_real_roots_candidates <= 16,
            "quadratic no-real-root candidates are capped at 16"
        );
        ensure!(
            self.square_residual_candidates <= 64,
            "square residual candidates are capped at 64"
        );
        ensure!(
            self.univariate_sturm_degree <= 64,
            "Sturm degree budget is capped at 64"
        );
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
    /// After the recorded bound orientations, sign*p is exactly a positive
    /// rational multiple of one binomial square plus a strictly positive
    /// coefficient-sign residual. The original Domain is never weakened.
    SquarePlusPositiveResidual {
        upper_variables: Vec<usize>,
        variables: [usize; 2],
        weight: String,
        residual: String,
    },
    /// A single shifted coordinate and an exact rational constant center.
    /// This also covers binomial squares after fixing a homogeneous anchor.
    AffineSquarePlusPositiveResidual {
        upper_variables: Vec<usize>,
        variable: usize,
        weight: String,
        center: String,
        residual: String,
    },
    /// A bounded sum of positive monomial-weighted binomial squares plus
    /// a strictly positive residual on the original shifted domain.
    WeightedSquaresPlusPositiveResidual {
        upper_variables: Vec<usize>,
        squares: Vec<WeightedSquareTerm>,
        residual: Vec<WeightedResidualTerm>,
    },
    /// Exact rational linear-form squares from PSD quadratic components,
    /// plus a nonzero positive-coefficient residual on the original domain.
    QuadraticComponentsPlusPositiveResidual {
        upper_variables: Vec<usize>,
        squares: Vec<QuadraticComponentSquare>,
        residual: Vec<WeightedResidualTerm>,
    },
    /// On the original box, a has the recorded sign and 4ac-b^2 is strictly
    /// positive. Hence a*z^2+b*z+c has no real roots and has the sign of a.
    /// Both children must use other methods: the maximum quadratic depth is one.
    QuadraticNoRealRoots {
        variable: usize,
        leading: Box<DomainCertificate>,
        negative_discriminant: Box<DomainCertificate>,
    },
    Polya {
        order: u32,
    },
    /// Exact LDL^T pivots for the signed quadratic matrix. An augmented
    /// matrix uses the constant coordinate 1; otherwise the domain must
    /// exclude the origin of the polynomial's active coordinates.
    QuadraticLdl {
        augmented: bool,
        diagonal: Vec<String>,
    },
    Bernstein {
        proof: BernsteinProof,
    },
    /// Exact open-interval Sturm count, with a strict Cauchy bound replacing
    /// infinite endpoints. All polynomial arithmetic is performed by Symbolica.
    UnivariateSturm {
        variable: usize,
    },
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
    if let Some(proof) = quadratic_ldl(polynomial, domain, options)? {
        return Ok(Some(make(
            proof.sign,
            CertificateMethod::QuadraticLdl {
                augmented: proof.augmented,
                diagonal: proof.diagonal,
            },
        )));
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
    if let Some((variable, sign)) = univariate_sturm(polynomial, domain, options)? {
        return Ok(Some(make(
            sign,
            CertificateMethod::UnivariateSturm { variable },
        )));
    }
    if let Some((sign, method)) = square_residual(algebra, polynomial, domain, options)? {
        return Ok(Some(make(sign, method)));
    }
    if let Some((sign, method)) = weighted_squares::discover(algebra, polynomial, domain, options)?
    {
        return Ok(Some(make(sign, method)));
    }
    if let Some((sign, method)) =
        quadratic_components::discover(algebra, polynomial, domain, options)?
    {
        return Ok(Some(make(sign, method)));
    }
    if let Some((sign, method)) = quadratic_no_real_roots(algebra, polynomial, domain, options)? {
        return Ok(Some(make(sign, method)));
    }
    Ok(None)
}

fn univariate_sturm(
    polynomial: &Poly,
    domain: &Domain,
    options: &CertificateOptions,
) -> Result<Option<(usize, i8)>> {
    let active = support(polynomial);
    if active.len() != 1 || polynomial.nterms() > options.max_terms {
        return Ok(None);
    }
    let variable = active[0];
    let degree = polynomial.degree(variable) as usize;
    // Rational Sturm remainders can grow rapidly. This is a conservative
    // portfolio attempt; the process supervisor still enforces hard limits.
    if degree > options.univariate_sturm_degree || degree.saturating_pow(3) > options.work_limit {
        return Ok(None);
    }
    let p = polynomial.to_univariate_from_univariate(variable);
    let leading = p.coefficients().last().unwrap().abs();
    let radius = p
        .coefficients()
        .iter()
        .take(degree)
        .map(|c| c.abs() / &leading)
        .max()
        .unwrap_or_else(Rational::zero)
        + Rational::from(2);
    let bounds = &domain.bounds[variable];
    let lower = bounds.lower.as_deref().map(rational).transpose()?;
    let upper = bounds.upper.as_deref().map(rational).transpose()?;
    let lo = lower
        .clone()
        .map_or_else(|| -radius.clone(), |x| x.max(-radius.clone()));
    let hi = upper
        .clone()
        .map_or_else(|| radius.clone(), |x| x.min(radius.clone()));
    // A failed nonvanishing attempt needs no Sturm chain. Exact interior
    // zeros or opposite signs are inexpensive counterexamples to strict
    // nonvanishing. This bounded screen only rejects certificate attempts;
    // it never establishes positivity or removes a projection factor.
    if univariate_sign_counterexample(&p, &lo, &hi) {
        return Ok(None);
    }
    if crate::roots::roots_in_open_interval(&p, &lo, &hi) != 0 {
        return Ok(None);
    }
    // Endpoint zeros are permitted only at the original strict boundaries.
    // The artificial Cauchy endpoints lie strictly outside every real root.
    let sample = match (lower, upper) {
        (Some(l), Some(u)) => (l + u) / Rational::from(2),
        (Some(l), None) => l + Rational::one(),
        (None, Some(u)) => u - Rational::one(),
        (None, None) => Rational::zero(),
    };
    let value = p.evaluate(&sample);
    ensure!(
        !value.is_zero(),
        "zero sample contradicts the Sturm nonvanishing count"
    );
    Ok(Some((variable, if value.is_negative() { -1 } else { 1 })))
}

fn univariate_sign_counterexample(
    p: &symbolica::poly::univariate::UnivariatePolynomial<Q>,
    lo: &Rational,
    hi: &Rational,
) -> bool {
    let mut previous = 0;
    let mut check = |x: Rational| {
        if &x <= lo || &x >= hi {
            return false;
        }
        let value = p.evaluate(&x);
        if value.is_zero() {
            return true;
        }
        let sign = if value.is_negative() { -1 } else { 1 };
        if previous != 0 && sign != previous {
            return true;
        }
        previous = sign;
        false
    };
    if check((lo + hi) / Rational::from(2)) || check(Rational::zero()) {
        return true;
    }
    let mut step = Rational::one();
    for _ in 0..10 {
        let inverse = Rational::one() / &step;
        for distance in [&step, &inverse] {
            // Both original-domain ends and the origin matter for shifted
            // rays and finite boxes. Endpoint zeros themselves are excluded.
            for point in [
                lo + distance,
                hi - distance,
                distance.clone(),
                -distance.clone(),
            ] {
                if check(point) {
                    return true;
                }
            }
        }
        step *= Rational::from(2);
    }
    false
}

#[cfg(test)]
mod sturm_screen_tests {
    use super::*;

    #[test]
    fn sampled_counterexamples_only_decline_and_respect_open_boundaries() {
        let algebra = Algebra::new(&["x".into()]).unwrap();
        let p = |text| {
            algebra
                .parse(text)
                .unwrap()
                .to_univariate_from_univariate(0)
        };
        let zero = Rational::zero();
        let one = Rational::one();
        assert!(univariate_sign_counterexample(
            &p("(x-1)^2"),
            &zero,
            &Rational::from(2)
        ));
        assert!(univariate_sign_counterexample(
            &p("x^2-2"),
            &zero,
            &Rational::from(3)
        ));
        assert!(!univariate_sign_counterexample(&p("x*(x-1)"), &zero, &one));
        assert!(!univariate_sign_counterexample(
            &p("(x-1)^2+1/1000000000000"),
            &zero,
            &Rational::from(2)
        ));
        // A negative pocket can evade every bounded sample. The screen is
        // deliberately inconclusive; the exact Sturm obligation still finds it.
        let narrow = p("(x-1/3)^2-1/10000000000000000000000000000000000000000");
        assert!(!univariate_sign_counterexample(&narrow, &zero, &one));
        assert_eq!(
            crate::roots::roots_in_open_interval(&narrow, &zero, &one),
            2
        );
    }
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
    if matches!(
        certificate.method,
        CertificateMethod::WeightedSquaresPlusPositiveResidual { .. }
            | CertificateMethod::QuadraticComponentsPlusPositiveResidual { .. }
    ) {
        ensure!(
            polynomial.variables() == &algebra.variables,
            "certificate polynomial has a different variable map"
        );
        ensure!(
            weighted_squares::bounded(polynomial),
            "bounded square polynomial budget"
        );
        ensure!(
            certificate.polynomial == algebra.format(polynomial),
            "certificate is for a different polynomial"
        );
    } else {
        ensure!(
            algebra.parse(&certificate.polynomial)? == *polynomial,
            "certificate is for a different polynomial"
        );
    }
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
        CertificateMethod::SquarePlusPositiveResidual {
            upper_variables,
            variables,
            weight,
            residual,
        } => {
            ensure!(
                certificate.limits.square_residual_candidates > 0,
                "square residual certificate is disabled"
            );
            ensure!(
                variables[0] < variables[1] && variables[1] < algebra.names.len(),
                "invalid square coordinate indices"
            );
            let weight = rational(weight)?;
            ensure!(weight > Rational::zero(), "square weight must be positive");
            let limit = certificate.limits.max_terms.min(SQUARE_MAX_TERMS);
            ensure!(
                polynomial.nterms() <= limit && support(polynomial).len() <= SQUARE_MAX_VARIABLES,
                "square residual certificate exceeds its polynomial budget"
            );
            let signed = polynomial
                .clone()
                .mul_coeff(Rational::from(certificate.sign as i64));
            let (shifted, positive) =
                shift_to_positive_oriented(algebra, &signed, &domain, upper_variables, limit)?
                    .ok_or_else(|| {
                        anyhow::anyhow!("square residual affine transform exceeds budget")
                    })?;
            let square = binomial_square(algebra, *variables, &weight)?;
            let residual = algebra.parse(residual)?;
            ensure!(
                residual.nterms() <= limit,
                "square residual exceeds term budget"
            );
            ensure!(
                &square + &residual == shifted,
                "square residual identity does not reconstruct the polynomial"
            );
            coefficient_sign(&residual, &positive) == Some(1)
        }
        CertificateMethod::AffineSquarePlusPositiveResidual {
            upper_variables,
            variable,
            weight,
            center,
            residual,
        } => {
            ensure!(
                certificate.limits.square_residual_candidates > 0,
                "affine square residual certificate is disabled"
            );
            ensure!(
                *variable < algebra.names.len(),
                "invalid affine square coordinate"
            );
            let weight = rational(weight)?;
            let center = rational(center)?;
            ensure!(weight > Rational::zero(), "square weight must be positive");
            ensure!(
                rational_height(&weight) <= AFFINE_SQUARE_MAX_HEIGHT
                    && rational_height(&center) <= AFFINE_SQUARE_MAX_HEIGHT,
                "affine square rational exceeds height budget"
            );
            let limit = certificate.limits.max_terms.min(SQUARE_MAX_TERMS);
            ensure!(
                polynomial.nterms() <= limit
                    && support(polynomial).len() <= SQUARE_MAX_VARIABLES
                    && affine_square_coefficients_bounded(polynomial),
                "affine square residual certificate exceeds its polynomial budget"
            );
            let signed = polynomial
                .clone()
                .mul_coeff(Rational::from(certificate.sign as i64));
            let (shifted, positive) =
                shift_to_positive_oriented(algebra, &signed, &domain, upper_variables, limit)?
                    .ok_or_else(|| {
                        anyhow::anyhow!("affine square residual transform exceeds budget")
                    })?;
            ensure!(
                affine_square_coefficients_bounded(&shifted),
                "shifted affine square polynomial exceeds height budget"
            );
            let square = affine_square(algebra, *variable, &weight, &center)?;
            let residual = algebra.parse(residual)?;
            ensure!(
                residual.nterms() <= limit && affine_square_coefficients_bounded(&residual),
                "affine square residual exceeds term or height budget"
            );
            ensure!(
                &square + &residual == shifted,
                "affine square residual identity does not reconstruct the polynomial"
            );
            coefficient_sign(&residual, &positive) == Some(1)
        }
        CertificateMethod::WeightedSquaresPlusPositiveResidual {
            upper_variables,
            squares,
            residual,
        } => weighted_squares::verify(
            algebra,
            polynomial,
            &domain,
            certificate.sign,
            upper_variables,
            squares,
            residual,
            &certificate.limits,
        )?,
        CertificateMethod::QuadraticComponentsPlusPositiveResidual {
            upper_variables,
            squares,
            residual,
        } => quadratic_components::verify(
            algebra,
            polynomial,
            &domain,
            certificate.sign,
            upper_variables,
            squares,
            residual,
            &certificate.limits,
        )?,
        CertificateMethod::QuadraticNoRealRoots {
            variable,
            leading,
            negative_discriminant,
        } => {
            ensure!(
                certificate.limits.quadratic_no_real_roots_candidates > 0,
                "quadratic no-real-root certificate is disabled"
            );
            // Reject nested variants before recursing into either child. This
            // enforces a proof-depth cap independently of discovery settings.
            ensure!(
                !matches!(
                    leading.method,
                    CertificateMethod::QuadraticNoRealRoots { .. }
                ) && !matches!(
                    negative_discriminant.method,
                    CertificateMethod::QuadraticNoRealRoots { .. }
                ),
                "nested quadratic no-real-root certificates exceed depth one"
            );
            let (a, b, c, minus_d) =
                quadratic_no_root_parts(polynomial, *variable, certificate.limits.max_terms)?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "quadratic no-real-root polynomial exceeds degree or work caps"
                        )
                    })?;
            let z = algebra.parse(&algebra.names[*variable])?;
            ensure!(
                &(&a * &z) * &z + &b * &z + c == *polynomial,
                "quadratic coefficient identity does not reconstruct the polynomial"
            );
            ensure!(
                a.degree(*variable) == 0 && minus_d.degree(*variable) == 0,
                "quadratic coefficient proofs depend on their fiber axis"
            );
            verify_certificate(
                algebra,
                &a,
                original_constraints,
                excluded_sign_constraints,
                leading,
            )?;
            verify_certificate(
                algebra,
                &minus_d,
                original_constraints,
                excluded_sign_constraints,
                negative_discriminant,
            )?;
            leading.sign == certificate.sign && negative_discriminant.sign == 1
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
        CertificateMethod::QuadraticLdl {
            augmented,
            diagonal,
        } => quadratic_ldl(polynomial, &domain, &certificate.limits)?.is_some_and(|proof| {
            proof.sign == certificate.sign
                && proof.augmented == *augmented
                && proof.diagonal == *diagonal
        }),
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
        CertificateMethod::UnivariateSturm { variable } => {
            univariate_sturm(polynomial, &domain, &certificate.limits)?
                == Some((*variable, certificate.sign))
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
    // Positive orthants already use these coordinates. Reconstructing the
    // same sparse polynomial through substitution can be quadratic in its
    // support size; retain the identical value and variable map directly.
    if offset.is_zero() && *scale == Rational::one() {
        return Ok(Some(p.clone()));
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
    shift_to_positive_oriented(algebra, polynomial, domain, &[], limit)
}

fn shift_to_positive_oriented(
    algebra: &Algebra,
    polynomial: &Poly,
    domain: &Domain,
    upper_variables: &[usize],
    limit: usize,
) -> Result<Option<(Poly, Vec<bool>)>> {
    ensure!(
        upper_variables.len() <= SQUARE_MAX_VARIABLES
            && upper_variables.windows(2).all(|w| w[0] < w[1])
            && upper_variables
                .iter()
                .all(|&v| { v < domain.bounds.len() && domain.bounds[v].upper.is_some() }),
        "invalid upper-bound orientation axes"
    );
    let mut p = polynomial.clone();
    let mut positive = vec![false; algebra.names.len()];
    for (v, bounds) in domain.bounds.iter().enumerate() {
        // Choosing an upper orientation ignores a simultaneous lower bound
        // only in this proof's coefficient argument. Every original-domain
        // point still maps to a strictly positive shifted coordinate.
        let transform = if upper_variables.binary_search(&v).is_ok() {
            Some((
                rational(bounds.upper.as_ref().unwrap())?,
                Rational::from(-1),
            ))
        } else if let Some(lower) = &bounds.lower {
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

const SQUARE_MAX_VARIABLES: usize = 16;
const SQUARE_MAX_TERMS: usize = 128;
const SQUARE_MAX_ORIENTATIONS: usize = 16;
const AFFINE_SQUARE_MAX_HEIGHT: u64 = 256;
const AFFINE_SQUARE_MAX_COEFFICIENT_HEIGHT: u64 = 512;

fn rational_height(value: &Rational) -> u64 {
    value.numerator_ref().significant_bits() + value.denominator_ref().significant_bits()
}

fn affine_square_coefficients_bounded(polynomial: &Poly) -> bool {
    polynomial
        .coefficients
        .iter()
        .all(|c| rational_height(c) <= AFFINE_SQUARE_MAX_COEFFICIENT_HEIGHT)
}

fn binomial_square(algebra: &Algebra, variables: [usize; 2], weight: &Rational) -> Result<Poly> {
    let difference = &algebra.parse(&algebra.names[variables[0]])?
        - &algebra.parse(&algebra.names[variables[1]])?;
    Ok((&difference * &difference).mul_coeff(weight.clone()))
}

fn affine_square(
    algebra: &Algebra,
    variable: usize,
    weight: &Rational,
    center: &Rational,
) -> Result<Poly> {
    let difference = algebra
        .parse(&algebra.names[variable])?
        .add_constant(-center.clone());
    Ok((&difference * &difference).mul_coeff(weight.clone()))
}

/// Conservative discovery, separate from proof replay. Candidate weights are
/// rational minima of two positive constant diagonal coefficients; no square
/// roots, fitted coefficients, or general SOS search enter the certificate.
/// Affine candidates complete one constant-coefficient diagonal square; their
/// rational center is -b/(2a). They share the binomial candidate/work budgets.
fn square_residual(
    algebra: &Algebra,
    polynomial: &Poly,
    domain: &Domain,
    options: &CertificateOptions,
) -> Result<Option<(i8, CertificateMethod)>> {
    let limit = options.max_terms.min(SQUARE_MAX_TERMS);
    if options.square_residual_candidates == 0 || polynomial.nterms() > limit {
        return Ok(None);
    }
    let active = support(polynomial);
    if active.is_empty() || active.len() > SQUARE_MAX_VARIABLES {
        return Ok(None);
    }
    let alternatives: Vec<_> = active
        .iter()
        .copied()
        .filter(|&v| domain.bounds[v].lower.is_some() && domain.bounds[v].upper.is_some())
        .collect();
    let mut orientations = vec![vec![]];
    for &v in alternatives.iter().take(SQUARE_MAX_ORIENTATIONS - 2) {
        orientations.push(vec![v]);
    }
    if alternatives.len() > 1 {
        orientations.push(alternatives);
    }
    let mut attempts = 0usize;
    let mut budget = ProofBudget::new(options);
    for upper_variables in orientations {
        if !budget.spend(polynomial.nterms().saturating_mul(algebra.names.len())) {
            return Ok(None);
        }
        let Some((shifted, positive)) =
            shift_to_positive_oriented(algebra, polynomial, domain, &upper_variables, limit)?
        else {
            continue;
        };
        for sign in [1i8, -1] {
            let signed = shifted.clone().mul_coeff(Rational::from(sign as i64));
            let mut diagonal = Vec::new();
            for &v in &active {
                let mut powers = vec![0; algebra.names.len()];
                powers[v] = 2;
                if let Some(c) = signed.coefficient(&powers) {
                    if c > Rational::zero() {
                        diagonal.push((v, c));
                    }
                }
            }
            for (i, (first, a)) in diagonal.iter().enumerate() {
                for (second, b) in diagonal.iter().skip(i + 1) {
                    let mut powers = vec![0; algebra.names.len()];
                    powers[*first] = 1;
                    powers[*second] = 1;
                    if !signed.coefficient(&powers).is_some_and(|c| c.is_negative()) {
                        continue;
                    }
                    if attempts >= options.square_residual_candidates
                        || !budget.spend(signed.nterms().saturating_add(3))
                    {
                        return Ok(None);
                    }
                    attempts += 1;
                    let variables = [*first, *second];
                    let weight = a.min(b).clone();
                    let square = binomial_square(algebra, variables, &weight)?;
                    let residual = &signed - &square;
                    if residual.nterms() <= limit
                        && coefficient_sign(&residual, &positive) == Some(1)
                    {
                        return Ok(Some((
                            sign,
                            CertificateMethod::SquarePlusPositiveResidual {
                                upper_variables,
                                variables,
                                weight: weight.to_string(),
                                residual: algebra.format(&residual),
                            },
                        )));
                    }
                }
            }
            if !affine_square_coefficients_bounded(polynomial)
                || !affine_square_coefficients_bounded(&signed)
            {
                continue;
            }
            for (variable, a) in diagonal {
                let mut powers = vec![0; algebra.names.len()];
                powers[variable] = 1;
                let Some(b) = signed.coefficient(&powers) else {
                    continue;
                };
                let center = -b / (a.clone() * Rational::from(2));
                if rational_height(&a) > AFFINE_SQUARE_MAX_HEIGHT
                    || rational_height(&center) > AFFINE_SQUARE_MAX_HEIGHT
                {
                    continue;
                }
                if attempts >= options.square_residual_candidates
                    || !budget.spend(signed.nterms().saturating_add(3))
                {
                    return Ok(None);
                }
                attempts += 1;
                let square = affine_square(algebra, variable, &a, &center)?;
                let residual = &signed - &square;
                if residual.nterms() <= limit
                    && affine_square_coefficients_bounded(&residual)
                    && coefficient_sign(&residual, &positive) == Some(1)
                {
                    return Ok(Some((
                        sign,
                        CertificateMethod::AffineSquarePlusPositiveResidual {
                            upper_variables,
                            variable,
                            weight: a.to_string(),
                            center: center.to_string(),
                            residual: algebra.format(&residual),
                        },
                    )));
                }
            }
        }
    }
    Ok(None)
}

const QUADRATIC_NO_ROOT_MAX_TERMS: usize = 128;
const QUADRATIC_NO_ROOT_MAX_VARIABLES: usize = 16;
const QUADRATIC_NO_ROOT_MAX_AXIS_DEGREE: u32 = 32;
const QUADRATIC_NO_ROOT_PRODUCT_BUDGET: usize = 4096;
const QUADRATIC_NO_ROOT_INPUT_HEIGHT: u64 = 512;
const QUADRATIC_NO_ROOT_OUTPUT_HEIGHT: u64 = 2048;

fn quadratic_coefficient(p: &Poly, variable: usize, degree: u32) -> Poly {
    let mut out = p.zero();
    for (c, powers) in p.coefficients.iter().zip(p.exponents_iter()) {
        if powers[variable] == degree {
            let mut powers = powers.to_vec();
            powers[variable] = 0;
            out.append_monomial(c.clone(), &powers);
        }
    }
    out
}

fn quadratic_no_root_input_bounded(p: &Poly, max_terms: usize) -> bool {
    p.nterms() <= max_terms.min(QUADRATIC_NO_ROOT_MAX_TERMS)
        && support(p).len() <= QUADRATIC_NO_ROOT_MAX_VARIABLES
        && (0..p.nvars()).all(|v| p.degree(v) <= QUADRATIC_NO_ROOT_MAX_AXIS_DEGREE)
        && p.coefficients
            .iter()
            .all(|c| rational_height(c) <= QUADRATIC_NO_ROOT_INPUT_HEIGHT)
}

/// Exact coefficient extraction and signed discriminant. The degree/product
/// caps are checked before polynomial multiplication and by replay as well.
fn quadratic_no_root_parts(
    p: &Poly,
    variable: usize,
    max_terms: usize,
) -> Result<Option<(Poly, Poly, Poly, Poly)>> {
    ensure!(
        variable < p.nvars(),
        "invalid quadratic no-real-root coordinate"
    );
    if p.degree(variable) != 2 || !quadratic_no_root_input_bounded(p, max_terms) {
        return Ok(None);
    }
    let a = quadratic_coefficient(p, variable, 2);
    let b = quadratic_coefficient(p, variable, 1);
    let c = quadratic_coefficient(p, variable, 0);
    let products = a
        .nterms()
        .saturating_mul(c.nterms())
        .saturating_add(b.nterms().saturating_mul(b.nterms()));
    if products > QUADRATIC_NO_ROOT_PRODUCT_BUDGET {
        return Ok(None);
    }
    let minus_d = (&a * &c).mul_coeff(Rational::from(4)) - &b * &b;
    if minus_d.nterms() > max_terms.min(QUADRATIC_NO_ROOT_PRODUCT_BUDGET)
        || minus_d
            .coefficients
            .iter()
            .any(|c| rational_height(c) > QUADRATIC_NO_ROOT_OUTPUT_HEIGHT)
    {
        return Ok(None);
    }
    Ok(Some((a, b, c, minus_d)))
}

fn quadratic_no_real_roots(
    algebra: &Algebra,
    polynomial: &Poly,
    domain: &Domain,
    options: &CertificateOptions,
) -> Result<Option<(i8, CertificateMethod)>> {
    if options.quadratic_no_real_roots_candidates == 0
        || !quadratic_no_root_input_bounded(polynomial, options.max_terms)
    {
        return Ok(None);
    }
    let mut leaf_options = options.clone();
    leaf_options.quadratic_no_real_roots_candidates = 0;
    let mut attempts = 0;
    let mut budget = ProofBudget::new(options);
    for variable in 0..algebra.names.len() {
        if polynomial.degree(variable) != 2 {
            continue;
        }
        if attempts >= options.quadratic_no_real_roots_candidates
            || !budget.spend(polynomial.nterms().saturating_mul(polynomial.nterms()))
        {
            break;
        }
        attempts += 1;
        let Some((a, _, _, minus_d)) =
            quadratic_no_root_parts(polynomial, variable, options.max_terms)?
        else {
            continue;
        };
        let Some(leading) = certify_nonzero(algebra, &a, domain, &leaf_options)? else {
            continue;
        };
        let Some(negative_discriminant) =
            certify_nonzero(algebra, &minus_d, domain, &leaf_options)?
        else {
            continue;
        };
        if negative_discriminant.sign != 1 {
            continue;
        }
        return Ok(Some((
            leading.sign,
            CertificateMethod::QuadraticNoRealRoots {
                variable,
                leading: Box::new(leading),
                negative_discriminant: Box::new(negative_discriminant),
            },
        )));
    }
    Ok(None)
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

struct QuadraticProof {
    sign: i8,
    augmented: bool,
    diagonal: Vec<String>,
}

/// Reconstruct q(x) = z^T A z exactly and certify sign*A positive definite.
/// The entries and all Schur complements are rational; no numerical tolerance
/// or sampled eigenvalue enters the proof. Only positive pivots are accepted,
/// so semidefinite forms remain inconclusive even if their nullspace might
/// miss the domain.
fn quadratic_ldl(
    polynomial: &Poly,
    domain: &Domain,
    options: &CertificateOptions,
) -> Result<Option<QuadraticProof>> {
    let active = support(polynomial);
    if active.is_empty() || polynomial.nterms() > options.max_terms {
        return Ok(None);
    }
    let mut budget = ProofBudget::new(options);
    let mut augmented = false;
    for powers in polynomial.exponents_iter() {
        if !budget.spend(powers.len()) {
            return Ok(None);
        }
        let Some(degree) = powers.iter().try_fold(0u32, |sum, &e| sum.checked_add(e)) else {
            return Ok(None);
        };
        if degree > 2 {
            return Ok(None);
        }
        augmented |= degree < 2;
    }
    if !augmented {
        // Positive definiteness gives zero only at the origin in these active
        // coordinates. A bound on an inactive variable cannot exclude it.
        let mut excludes_origin = false;
        for &v in &active {
            let b = &domain.bounds[v];
            excludes_origin |= match &b.lower {
                Some(lower) => rational(lower)? >= Rational::zero(),
                None => false,
            } || match &b.upper {
                Some(upper) => rational(upper)? <= Rational::zero(),
                None => false,
            };
        }
        if !excludes_origin {
            return Ok(None);
        }
    }
    let dimension = active.len() + usize::from(augmented);
    if !budget.spend(dimension.saturating_mul(dimension)) {
        return Ok(None);
    }
    let mut matrix = vec![vec![Rational::zero(); dimension]; dimension];
    for (coefficient, powers) in polynomial
        .coefficients
        .iter()
        .zip(polynomial.exponents_iter())
    {
        if !budget.spend(active.len()) {
            return Ok(None);
        }
        let factors: Vec<_> = active
            .iter()
            .enumerate()
            .flat_map(|(i, &v)| std::iter::repeat_n(i, powers[v] as usize))
            .collect();
        let (i, j) = match factors.as_slice() {
            [] => (active.len(), active.len()),
            [i] => (*i, active.len()),
            [i, j] => (*i, *j),
            _ => unreachable!("total degree checked above"),
        };
        let entry = if i == j {
            coefficient.clone()
        } else {
            coefficient.clone() / Rational::from(2)
        };
        matrix[i][j] += entry.clone();
        if i != j {
            matrix[j][i] += entry;
        }
    }
    // A positive/negative definite matrix has diagonals of that strict sign.
    // The first diagonal therefore fixes the only possible orientation.
    if matrix[0][0].is_zero() {
        return Ok(None);
    }
    let sign = if matrix[0][0].is_negative() { -1 } else { 1 };
    if sign < 0 {
        for row in &mut matrix {
            for entry in row {
                *entry = -entry.clone();
            }
        }
    }
    let mut diagonal = Vec::with_capacity(dimension);
    for k in 0..dimension {
        if !budget.spend(1) {
            return Ok(None);
        }
        let pivot = matrix[k][k].clone();
        if pivot <= Rational::zero() {
            return Ok(None);
        }
        diagonal.push(pivot.to_string());
        // Store only the lower triangle of each exact Schur complement.
        // Entries in column k stay intact until all updates use them.
        for i in k + 1..dimension {
            for j in k + 1..=i {
                if !budget.spend(3) {
                    return Ok(None);
                }
                let correction = &matrix[i][k] * &matrix[j][k] / pivot.clone();
                matrix[i][j] -= correction;
            }
        }
    }
    Ok(Some(QuadraticProof {
        sign,
        augmented,
        diagonal,
    }))
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

#[cfg(test)]
mod affine_identity_tests {
    use super::*;

    #[test]
    fn identity_shift_preserves_polynomial_map_and_existing_term_cap() {
        let algebra = Algebra::new(&["x".into(), "y".into()]).unwrap();
        let p = algebra.parse("x^20-x*y+3/7").unwrap();
        let result = affine_replace(
            &algebra,
            &p,
            0,
            &Rational::zero(),
            &Rational::one(),
            p.nterms(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(result, p);
        assert_eq!(result.variables(), &algebra.variables);
        assert!(
            affine_replace(
                &algebra,
                &p,
                0,
                &Rational::zero(),
                &Rational::one(),
                p.nterms() - 1,
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn nonidentity_affine_transforms_keep_their_exact_effect() {
        let algebra = Algebra::new(&["x".into(), "y".into()]).unwrap();
        let p = algebra.parse("x^2-x*y+3/7").unwrap();
        for (offset, scale, expected) in [(0, 2, "4*x^2-2*x*y+3/7"), (2, -1, "(2-x)^2-(2-x)*y+3/7")]
        {
            let actual = affine_replace(
                &algebra,
                &p,
                0,
                &Rational::from(offset),
                &Rational::from(scale),
                32,
            )
            .unwrap()
            .unwrap();
            assert_eq!(actual, algebra.parse(expected).unwrap());
        }
    }
}
