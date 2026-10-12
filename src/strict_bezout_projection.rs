//! Strict-Bezout pair omission for the certified global projection.
//!
//! An exact identity p*L + q*M = k*S with rational k > 0, where S is strictly
//! positive on the open coordinate box B derived from the original strict
//! coordinate-affine constraints, proves that p and q have no common zero in
//! B: at such a zero the left side vanishes and the right side does not. S is
//! either H^2 for an H with an exact domain nonvanishing certificate (this
//! includes the local engine's uniformly signed H on a positive orthant), or
//! any S with an exact domain certificate of sign +1 (src/domain.rs).
//!
//! Why Res(p,q) may then be omitted in certified projection mode: every
//! non-rejected base cell C at the level of p and q lies in the projection of
//! B, because coordinate-affine factors are protected projection inputs whose
//! constraints reject every branch outside B at their own level. Over C, p and
//! q stay individually delineable (their leading coefficients and
//! discriminants are kept or certified nonzero on B). Their real root
//! functions inside the open x-interval of B never meet, and a root can enter
//! or leave that interval only where a retained endpoint resultant vanishes:
//! neither p nor q may be coordinate-affine, so their resultants with the
//! interval's affine faces are never omitted. The root order inside B, and
//! hence every accepted sector, is therefore invariant over C. Sectors outside
//! B are rejected by their coordinate constraint. Only the pair obligation is
//! omitted: p, q, their coefficients and discriminants all remain.
//!
//! Proposals are never premises. The producer checks the identity and
//! discovers a certificate; the verifier replays the identity and the
//! certificate independently and checks pair membership at that level.
use crate::{
    algebra::{Algebra, ExpansionCaps, Poly, canonical, rational},
    domain::{CertificateOptions, Domain, DomainCertificate, certify_nonzero, verify_certificate},
};
use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use symbolica::{domains::InternalOrdering, prelude::*};

pub const MAX_HINTS: usize = 128;
const MAX_EXPRESSION: usize = 64 * 1024;
const MAX_PRODUCTS: usize = 65_536;
const MAX_HINT_DEGREE: u64 = 64;
const HINT_CAPS: ExpansionCaps = ExpansionCaps {
    max_terms: 16_384,
    max_degree: MAX_HINT_DEGREE,
    max_coefficient_bits: 65_536,
};

/// A proposed identity p*L + q*M = k*S. Exactly one of `square` (S = square^2)
/// or `positive` (S itself) is given. Field names match the local engine's
/// strict-Bezout hint, so a local `square` hint is accepted verbatim.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionBezoutHint {
    pub schema_version: u32,
    pub polynomials: [String; 2],
    pub multipliers: [String; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub square: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub positive: Option<String>,
    pub multiplier: String,
}

/// Replayable evidence that Res(p,q) was omitted at one projection level.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionPairOmission {
    /// Intermediate output level, as for ProjectionRemoval.
    pub level: usize,
    /// Variable index in the stable original variable map.
    pub eliminated_variable: usize,
    /// The canonical basis polynomials, in canonical order, original map.
    pub polynomials: [String; 2],
    pub hint: ProjectionBezoutHint,
    /// Certificate for `square` (any sign) or `positive` (sign +1).
    pub certificate: DomainCertificate,
}

/// A checked identity with its discovered certificate.
#[derive(Clone, Debug)]
pub struct Prepared {
    pub hint: ProjectionBezoutHint,
    /// Canonical forms of p and q, ordered by the internal polynomial order.
    pub pair: [Poly; 2],
    pub certificate: DomainCertificate,
}

/// Lifting needs the box faces: never omit a pair involving one.
pub(crate) fn is_coordinate_affine(polynomial: &Poly) -> bool {
    let mut support = (0..polynomial.nvars()).filter(|&v| polynomial.degree(v) > 0);
    support.next().is_some_and(|v| polynomial.degree(v) == 1) && support.next().is_none()
}

fn bounded(p: &Poly) -> bool {
    p.nterms() <= 4096
        && p.exponents_iter()
            .all(|powers| powers.iter().map(|&e| u64::from(e)).sum::<u64>() <= MAX_HINT_DEGREE)
        && p.coefficients.iter().all(|c| {
            c.numerator_ref().significant_bits() <= 4096
                && c.denominator_ref().significant_bits() <= 4096
        })
}

/// The multiplier k as a plain signed decimal rational literal: Symbolica's
/// parser would evaluate an expression such as `7^(3^20)` before any cap.
/// 1,240 digits exceed the 4,096-bit height cap checked afterwards.
fn rational_literal(text: &str) -> Result<Rational> {
    let unsigned = text
        .strip_prefix('-')
        .or_else(|| text.strip_prefix('+'))
        .unwrap_or(text);
    let mut parts = unsigned.split('/');
    let numerator = parts.next().unwrap_or("");
    let denominator = parts.next();
    let digits =
        |s: &str| !s.is_empty() && s.len() <= 1240 && s.bytes().all(|b| b.is_ascii_digit());
    ensure!(
        digits(numerator) && denominator.is_none_or(digits) && parts.next().is_none(),
        "strict-Bezout multiplier must be a decimal rational literal"
    );
    rational(text)
}

/// Exact identity check without any discovery. Returns the canonical pair,
/// the certified kernel (H or S) and whether S = H^2.
fn identity(algebra: &Algebra, hint: &ProjectionBezoutHint) -> Result<([Poly; 2], Poly, bool)> {
    ensure!(
        hint.schema_version == 1,
        "unsupported projection strict-Bezout schema"
    );
    let (kernel, squared) = match (&hint.square, &hint.positive) {
        (Some(square), None) => (square, true),
        (None, Some(positive)) => (positive, false),
        _ => anyhow::bail!("strict-Bezout hint needs exactly one of square or positive"),
    };
    ensure!(
        hint.polynomials
            .iter()
            .chain(&hint.multipliers)
            .chain([kernel, &hint.multiplier])
            .all(|text| !text.is_empty() && text.len() <= MAX_EXPRESSION),
        "strict-Bezout expression byte cap"
    );
    // Hints and replayed evidence are untrusted text: bound the expansion
    // before Symbolica performs it, then apply the polynomial caps.
    let parse = |text: &str| -> Result<Poly> {
        let p = algebra.parse_with_expansion_caps(text, HINT_CAPS)?;
        ensure!(bounded(&p), "strict-Bezout polynomial resource cap");
        Ok(p)
    };
    let p = parse(&hint.polynomials[0])?;
    let q = parse(&hint.polynomials[1])?;
    let l = parse(&hint.multipliers[0])?;
    let m = parse(&hint.multipliers[1])?;
    let kernel = parse(kernel)?;
    let k = rational_literal(&hint.multiplier)?;
    ensure!(
        k > Rational::zero()
            && k.numerator_ref().significant_bits() <= 4096
            && k.denominator_ref().significant_bits() <= 4096,
        "strict-Bezout multiplier must be bounded and positive"
    );
    ensure!(
        !p.is_constant() && !q.is_constant(),
        "strict-Bezout pair contains a constant"
    );
    let mut pair = [canonical(&p), canonical(&q)];
    ensure!(pair[0] != pair[1], "strict-Bezout pair is not distinct");
    ensure!(
        !pair.iter().any(is_coordinate_affine),
        "strict-Bezout pair cannot contain a coordinate-affine domain face"
    );
    ensure!(!kernel.is_zero(), "strict-Bezout kernel is zero");
    for (a, b) in [(&p, &l), (&q, &m), (&kernel, &kernel)] {
        ensure!(
            a.nterms().saturating_mul(b.nterms()) <= MAX_PRODUCTS,
            "strict-Bezout identity multiplication cap"
        );
    }
    let left = &(&p * &l) + &(&q * &m);
    let s = if squared {
        &kernel * &kernel
    } else {
        kernel.clone()
    };
    ensure!(
        left == s.mul_coeff(k),
        "strict-Bezout polynomial identity is false"
    );
    if pair[0].internal_cmp(&pair[1]).is_gt() {
        pair.swap(0, 1);
    }
    Ok((pair, kernel, squared))
}

/// Check the identity, then discover an exact certificate on `domain`.
pub fn prepare(
    algebra: &Algebra,
    domain: &Domain,
    options: &CertificateOptions,
    hint: &ProjectionBezoutHint,
) -> Result<Prepared> {
    let (pair, kernel, squared) = identity(algebra, hint)?;
    let certificate = certify_nonzero(algebra, &kernel, domain, options)?
        .ok_or_else(|| anyhow!("no exact nonvanishing certificate for the strict-Bezout kernel"))?;
    ensure!(
        squared || certificate.sign == 1,
        "strict-Bezout positive kernel is not certified strictly positive"
    );
    Ok(Prepared {
        hint: hint.clone(),
        pair,
        certificate,
    })
}

impl Prepared {
    pub fn evidence(
        &self,
        algebra: &Algebra,
        level: usize,
        eliminated_variable: usize,
    ) -> ProjectionPairOmission {
        ProjectionPairOmission {
            level,
            eliminated_variable,
            polynomials: [algebra.format(&self.pair[0]), algebra.format(&self.pair[1])],
            hint: self.hint.clone(),
            certificate: self.certificate.clone(),
        }
    }
}

/// Independent replay: identity, certificate against the original domain,
/// and the recorded canonical pair. Returns the pair to omit. The caller
/// checks level, elimination axis and membership in that level's basis.
pub fn verify(
    algebra: &Algebra,
    original_constraints: &[Poly],
    excluded_sign_constraints: &[usize],
    omission: &ProjectionPairOmission,
) -> Result<[Poly; 2]> {
    let (pair, kernel, squared) = identity(algebra, &omission.hint)?;
    verify_certificate(
        algebra,
        &kernel,
        original_constraints,
        excluded_sign_constraints,
        &omission.certificate,
    )?;
    ensure!(
        squared || omission.certificate.sign == 1,
        "strict-Bezout positive kernel is not certified strictly positive"
    );
    ensure!(
        omission.polynomials == [algebra.format(&pair[0]), algebra.format(&pair[1])],
        "strict-Bezout omission names a different pair"
    );
    Ok(pair)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::derive_domain;

    fn setup() -> (Algebra, Vec<Poly>, Domain) {
        let algebra = Algebra::new(&["x".into(), "y".into()]).unwrap();
        let constraints = vec![algebra.parse("x").unwrap(), algebra.parse("y").unwrap()];
        let domain = derive_domain(&algebra, &constraints, &[]).unwrap();
        (algebra, constraints, domain)
    }

    fn hint() -> ProjectionBezoutHint {
        ProjectionBezoutHint {
            schema_version: 1,
            polynomials: ["y^2-x".into(), "2*y^2+2*x*y+x^2-x".into()],
            multipliers: ["-1".into(), "1".into()],
            square: Some("x+y".into()),
            positive: None,
            multiplier: "1".into(),
        }
    }

    #[test]
    fn square_and_positive_kernels_are_certified_and_replayed() {
        let (algebra, constraints, domain) = setup();
        let options = CertificateOptions::default();
        let prepared = prepare(&algebra, &domain, &options, &hint()).unwrap();
        let evidence = prepared.evidence(&algebra, 0, 1);
        assert_eq!(
            verify(&algebra, &constraints, &[], &evidence).unwrap(),
            prepared.pair
        );
        // S itself, positive on the open quadrant: (x+y)^2 = x^2+2xy+y^2.
        let mut positive = hint();
        positive.square = None;
        positive.positive = Some("x^2+2*x*y+y^2".into());
        let prepared = prepare(&algebra, &domain, &options, &positive).unwrap();
        assert_eq!(prepared.certificate.sign, 1);
        verify(
            &algebra,
            &constraints,
            &[],
            &prepared.evidence(&algebra, 0, 1),
        )
        .unwrap();
        // Negative S is certified nonzero but not positive; k must be > 0.
        let mut negative = positive.clone();
        negative.positive = Some("-x^2-2*x*y-y^2".into());
        negative.multipliers = ["1".into(), "-1".into()];
        assert!(
            prepare(&algebra, &domain, &options, &negative)
                .unwrap_err()
                .to_string()
                .contains("strictly positive")
        );
        let mut negative_k = hint();
        negative_k.multiplier = "-1".into();
        negative_k.multipliers = ["1".into(), "-1".into()];
        assert!(prepare(&algebra, &domain, &options, &negative_k).is_err());
    }

    #[test]
    fn mutated_identities_kernels_and_pairs_are_rejected() {
        let (algebra, constraints, domain) = setup();
        let options = CertificateOptions::default();
        let mut altered = hint();
        altered.multipliers[0] = "-2".into();
        assert!(
            prepare(&algebra, &domain, &options, &altered)
                .unwrap_err()
                .to_string()
                .contains("identity is false")
        );
        let mut altered = hint();
        altered.multiplier = "2".into();
        assert!(prepare(&algebra, &domain, &options, &altered).is_err());
        // A mixed-sign kernel vanishes inside the quadrant: no certificate.
        let mut mixed = hint();
        mixed.polynomials[1] = "2*y^2-2*x*y+x^2-x".into();
        mixed.square = Some("x-y".into());
        assert!(
            prepare(&algebra, &domain, &options, &mixed)
                .unwrap_err()
                .to_string()
                .contains("certificate")
        );
        let mut both = hint();
        both.positive = Some("1".into());
        assert!(prepare(&algebra, &domain, &options, &both).is_err());
        // A coordinate face may never be omitted: x*1 + q*0 = 1*x, x > 0.
        let face = ProjectionBezoutHint {
            schema_version: 1,
            polynomials: ["x".into(), "y^2-x".into()],
            multipliers: ["1".into(), "0".into()],
            square: None,
            positive: Some("x".into()),
            multiplier: "1".into(),
        };
        assert!(
            prepare(&algebra, &domain, &options, &face)
                .unwrap_err()
                .to_string()
                .contains("coordinate-affine")
        );
        // Replay rejects changed evidence.
        let prepared = prepare(&algebra, &domain, &options, &hint()).unwrap();
        let good = prepared.evidence(&algebra, 0, 1);
        let mut wrong_pair = good.clone();
        wrong_pair.polynomials[1] = "x^2+y".into();
        assert!(verify(&algebra, &constraints, &[], &wrong_pair).is_err());
        let mut wrong_identity = good.clone();
        wrong_identity.hint.multipliers[1] = "2".into();
        assert!(verify(&algebra, &constraints, &[], &wrong_identity).is_err());
        let mut wrong_certificate = good.clone();
        wrong_certificate.certificate.sign = -wrong_certificate.certificate.sign;
        assert!(verify(&algebra, &constraints, &[], &wrong_certificate).is_err());
        let mut wrong_kernel = good.clone();
        wrong_kernel.certificate.polynomial = "x+2*y".into();
        assert!(verify(&algebra, &constraints, &[], &wrong_kernel).is_err());
        // The certificate's domain must be the original one.
        let shifted = vec![algebra.parse("x-1").unwrap(), algebra.parse("y").unwrap()];
        assert!(verify(&algebra, &shifted, &[], &good).is_err());
    }
}
