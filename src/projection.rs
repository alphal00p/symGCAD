//! The original GCAD generic projection operator (SFRP after each PR step).

use std::{collections::HashSet, time::Instant};

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use symbolica::domains::InternalOrdering;
use symbolica::prelude::Rational;

use crate::algebra::{Algebra, MAX_DEGREE, Poly, canonical, factors};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectionStep {
    pub variable: usize,
    pub input_count: usize,
    pub output_count: usize,
    pub input_terms: usize,
    pub output_terms: usize,
    pub elapsed_seconds: f64,
}

#[derive(Clone, Debug)]
pub struct Projection {
    /// Level k contains only polynomials with positive degree in variable k.
    /// The full Algebra variable map is retained at every level.
    pub levels: Vec<Vec<Poly>>,
    pub steps: Vec<ProjectionStep>,
}

/// Guard exponent growth before resultant construction. Symbolica's dense
/// coefficient views require signed-32-bit degrees even for u32 exponents.
fn check_resultant_degrees(left: &Poly, right: &Poly, variable: usize) -> Result<()> {
    let m = left.degree(variable) as u64;
    let n = right.degree(variable) as u64;
    ensure!(
        m <= MAX_DEGREE && n <= MAX_DEGREE,
        "resultant elimination degree exceeds supported range"
    );
    for i in 0..left.nvars() {
        if i == variable {
            continue;
        }
        let bound = m * right.degree(i) as u64 + n * left.degree(i) as u64;
        ensure!(
            bound <= MAX_DEGREE,
            "resultant degree bound exceeds supported exponent range in variable {i}"
        );
    }
    Ok(())
}

/// Deterministic resultant, preserving its exact rational scale and sign.
/// Bounded bivariate inputs may use exact interpolation; all others use Ducos.
pub fn exact_resultant(left: &Poly, right: &Poly, variable: usize) -> Result<Poly> {
    Ok(exact_resultant_with_statistics(left, right, variable)?.0)
}

pub fn exact_resultant_with_statistics(
    left: &Poly,
    right: &Poly,
    variable: usize,
) -> Result<(Poly, Option<crate::resultant_interpolation::Statistics>)> {
    ensure!(
        left.variables() == right.variables(),
        "resultant variable maps differ"
    );
    ensure!(variable < left.nvars(), "resultant variable out of range");
    check_resultant_degrees(left, right, variable)?;
    // Existing library calls have no deadline context. Callers needing
    // cooperative cancellation may use try_resultant's callback directly;
    // supervised CLI workers retain hard process deadlines.
    if let Some((result, stats)) = crate::resultant_interpolation::try_resultant(
        left,
        right,
        variable,
        &crate::resultant_interpolation::Options::default(),
        || Ok(()),
    )? {
        return Ok((result, Some(stats)));
    }
    Ok((exact_resultant_ducos(left, right, variable)?, None))
}

/// Deterministic direct backend, also used for independent interpolation
/// regressions and exact numeric images. No automatic/Monte Carlo dispatch.
pub fn exact_resultant_ducos(left: &Poly, right: &Poly, variable: usize) -> Result<Poly> {
    ensure!(
        left.variables() == right.variables(),
        "resultant variable maps differ"
    );
    ensure!(variable < left.nvars(), "resultant variable out of range");
    check_resultant_degrees(left, right, variable)?;
    // Use the signed homogeneous evaluation identity explicitly for a linear
    // operand. The current Symbolica small-case implementation omits the
    // (-1)^degree factor for Res(f, b0+b1*x) when degree(f) is odd.
    if !left.is_zero() && !right.is_zero() {
        if right.degree(variable) == 1 && left.degree(variable) > 0 {
            return Ok(resultant_with_linear_second(left, right, variable));
        }
        if left.degree(variable) == 1 && right.degree(variable) > 0 {
            let result = resultant_with_linear_second(right, left, variable);
            return Ok(if right.degree(variable) % 2 == 1 {
                -result
            } else {
                result
            });
        }
    }
    if left.degree(variable) >= 2
        && right.degree(variable) >= 2
        && resultant_vanishes_identically(left, right, variable)
    {
        return Ok(left.zero());
    }
    Ok(left
        .to_univariate(variable)
        .resultant_ducos_integer(&right.to_univariate(variable)))
}

/// Exact test for Res_x(left, right) = 0, i.e. a common factor of positive
/// degree in x. Symbolica 3.0.1's Lazard-Ducos panics instead of returning
/// zero in that case (`patches/symbolica-ducos-zero-subresultant.patch`), so
/// Ducos is only called once this returns false.
///
/// At a point where both leading coefficients in x stay nonzero, coprime
/// specializations prove Res(point) != 0 (resultant and specialization
/// commute there), hence Res != 0. That integer coprimality test is
/// `resultant_segmentation::integer_polynomials_share_factor`. If three
/// such points all share a factor, Symbolica's exact multivariate gcd decides.
fn resultant_vanishes_identically(left: &Poly, right: &Poly, variable: usize) -> bool {
    use symbolica::prelude::*;
    let parameters: Vec<usize> = (0..left.nvars())
        .filter(|&v| v != variable && (left.degree(v) > 0 || right.degree(v) > 0))
        .collect();
    let leading = [
        left.univariate_lcoeff(variable),
        right.univariate_lcoeff(variable),
    ];
    let integer_image = |f: &Poly| -> UnivariatePolynomial<Z> {
        let image = f.to_univariate_from_univariate(variable);
        let mut denominator = Integer::one();
        for c in image.coefficients() {
            denominator = denominator.lcm(c.denominator_ref());
        }
        let scale = Rational::from(denominator);
        UnivariatePolynomial::from_coefficients(
            &Z,
            image
                .coefficients()
                .iter()
                .map(|c| (c * &scale).numerator())
                .collect(),
            std::sync::Arc::new(f.variables()[variable].clone()),
        )
    };
    let mut full_degree_points = 0;
    for t in 0..16i64 {
        let specialize = |f: &Poly| {
            parameters
                .iter()
                .enumerate()
                .fold(f.clone(), |acc, (j, &v)| {
                    let j = j as i64;
                    acc.replace(v, &Rational::from(2 + t * (j + 1) + j))
                })
        };
        if leading.iter().any(|c| specialize(c).is_zero()) {
            continue;
        }
        let (a, b) = (specialize(left), specialize(right));
        if !crate::resultant_segmentation::integer_polynomials_share_factor(
            &integer_image(&a),
            &integer_image(&b),
        ) {
            return false;
        }
        full_degree_points += 1;
        if full_degree_points == 3 || parameters.is_empty() {
            break;
        }
    }
    left.gcd(right).degree(variable) > 0
}

fn resultant_with_linear_second(polynomial: &Poly, linear: &Poly, variable: usize) -> Poly {
    let coefficients = polynomial.to_univariate(variable);
    let linear = linear.to_univariate(variable);
    let neg_constant = -linear.coefficients()[0].clone();
    let leading = &linear.coefficients()[1];
    let mut power = leading.clone();
    let mut result = coefficients.coefficients().last().unwrap().clone();
    for coefficient in coefficients.coefficients()[..coefficients.degree()]
        .iter()
        .rev()
    {
        result = &(&result * &neg_constant) + &(coefficient * &power);
        power = &power * leading;
    }
    if coefficients.degree() % 2 == 1 {
        -result
    } else {
        result
    }
}

/// Exact discriminant, including its conventional sign. Degree-zero and
/// degree-one inputs contribute no nonconstant discriminant to projection.
pub fn discriminant(polynomial: &Poly, variable: usize) -> Result<Poly> {
    ensure!(
        variable < polynomial.nvars(),
        "projection variable out of range"
    );
    let degree = polynomial.degree(variable);
    if degree == 0 {
        return Ok(polynomial.zero());
    }
    if degree == 1 {
        return Ok(polynomial.one());
    }
    let derivative = polynomial.derivative(variable);
    check_resultant_degrees(polynomial, &derivative, variable)?;
    if degree <= 4 {
        let coefficients = polynomial.to_univariate(variable);
        return Ok(low_degree_discriminant(coefficients.coefficients()));
    }
    let resultant = exact_resultant(polynomial, &derivative, variable)?;
    discriminant_from_resultant(polynomial, variable, resultant)
}

/// Opt-in exact discriminant backend. Low-degree identities are unchanged;
/// ordinary bounded interpolation has priority. Only after its static decline
/// may a one- or two-parameter resultant use binary segmentation with the existing
/// resource caps. No discovery cache or supplied polynomial is consulted.
///
/// Like the ordinary library backend, individual native calls require an
/// external worker deadline. Runtime errors never trigger a second backend.
pub fn discriminant_with_binary_statistics(
    polynomial: &Poly,
    variable: usize,
) -> Result<(Poly, Option<crate::resultant_segmentation::Statistics>)> {
    ensure!(
        variable < polynomial.nvars(),
        "projection variable out of range"
    );
    if polynomial.degree(variable) <= 4 {
        return Ok((discriminant(polynomial, variable)?, None));
    }
    let derivative = polynomial.derivative(variable);
    check_resultant_degrees(polynomial, &derivative, variable)?;
    if let Some((resultant, _)) = crate::resultant_interpolation::try_resultant(
        polynomial,
        &derivative,
        variable,
        &crate::resultant_interpolation::Options::default(),
        || Ok(()),
    )? {
        return Ok((
            discriminant_from_resultant(polynomial, variable, resultant)?,
            None,
        ));
    }
    // Discriminants also admit one parameter; general pair-resultant dispatch
    // keeps its existing two-parameter envelope. All size caps are shared.
    let options = crate::resultant_segmentation::Options {
        min_parameters: 1,
        max_parameters: 2,
        ..crate::resultant_segmentation::Options::default()
    };
    let (resultant, stats) = if let Some((resultant, stats)) =
        crate::resultant_segmentation::try_resultant(
            polynomial,
            &derivative,
            variable,
            &options,
            || Ok(()),
        )? {
        (resultant, Some(stats))
    } else {
        (
            exact_resultant_ducos(polynomial, &derivative, variable)?,
            None,
        )
    };
    Ok((
        discriminant_from_resultant(polynomial, variable, resultant)?,
        stats,
    ))
}

fn discriminant_from_resultant(
    polynomial: &Poly,
    variable: usize,
    resultant: Poly,
) -> Result<Poly> {
    // Native Ducos may return a mapless ring constant (notably zero for
    // repeated factors). Reattach that exact value before checking support.
    let resultant = if resultant.is_constant() {
        polynomial.constant(resultant.get_constant())
    } else {
        resultant
    };
    ensure!(
        resultant.variables() == polynomial.variables(),
        "discriminant resultant variable map changed"
    );
    ensure!(
        resultant.degree(variable) == 0,
        "discriminant resultant contains eliminated variable"
    );
    let (mut quotient, remainder) =
        resultant.quot_rem(&polynomial.univariate_lcoeff(variable), false);
    ensure!(
        remainder.is_zero(),
        "discriminant leading-coefficient division was not exact"
    );
    if polynomial.degree(variable) % 4 == 2 || polynomial.degree(variable) % 4 == 3 {
        quotient = -quotient;
    }
    Ok(quotient)
}

/// Exact coefficient identities over Q. They introduce no polynomial
/// denominators and remain valid when the symbolic leading coefficient
/// vanishes after specialization. The generic-coefficient tests independently
/// reconstruct each identity by the deterministic resultant definition.
fn low_degree_discriminant(coefficients: &[Poly]) -> Poly {
    let scale = |polynomial: Poly, scalar: i64| polynomial.mul_coeff(Rational::from(scalar));
    match coefficients {
        [c, b, a] => &(b * b) - &scale(a * c, 4),
        [d, c, b, a] => {
            let bc = b * c;
            let ad = a * d;
            let ac3 = a * &c.pow(3);
            let b3d = &b.pow(3) * d;
            let first = &bc.pow(2) - &scale(&ac3 + &b3d, 4);
            &(&first - &scale(ad.pow(2), 27)) + &scale(&ad * &bc, 18)
        }
        [e, d, c, b, a] => {
            // Binary-quartic invariants: disc=(4 I^3-J^2)/27,
            // I=12ae-3bd+c^2, J=72ace+9bcd-27ad^2-27b^2e-2c^3.
            // Grouping through I and J avoids repeated polynomial quotients
            // and shares products while retaining the conventional scaling.
            let c2 = c * c;
            let bd = b * d;
            let invariant_i = &(&scale(a * e, 12) - &scale(bd.clone(), 3)) + &c2;
            let positive_j = &scale(&(a * c) * e, 72) + &scale(&bd * c, 9);
            let negative_j = &scale(a * &d.pow(2), 27) + &scale(&b.pow(2) * e, 27);
            let invariant_j = &(&positive_j - &negative_j) - &scale(&c2 * c, 2);
            (&scale(invariant_i.pow(3), 4) - &invariant_j.pow(2)).div_coeff(&Rational::from(27))
        }
        _ => unreachable!("low-degree discriminant requires degree two, three or four"),
    }
}

/// Raw PR: leading coefficients, discriminants, and pairwise resultants.
/// Zero/constant output polynomials have no roots and are discarded.
pub fn projection_polynomials(polynomials: &[Poly], variable: usize) -> Result<Vec<Poly>> {
    let mut output = HashSet::new();
    let mut add = |polynomial: Poly| {
        if !polynomial.is_constant() {
            output.insert(canonical(&polynomial));
        }
    };
    for (i, polynomial) in polynomials.iter().enumerate() {
        ensure!(
            variable < polynomial.nvars(),
            "projection variable out of range"
        );
        if polynomial.is_constant() {
            continue;
        }
        add(polynomial.univariate_lcoeff(variable));
        if polynomial.degree(variable) >= 2 {
            add(discriminant(polynomial, variable)?);
        }
        for other in &polynomials[i + 1..] {
            ensure!(
                polynomial.variables() == other.variables(),
                "projection polynomials have different variable maps"
            );
            if other.is_constant() {
                continue;
            }
            add(exact_resultant(polynomial, other, variable)?);
        }
    }
    let mut output: Vec<Poly> = output.into_iter().collect();
    output.sort_by(|a, b| a.internal_cmp(b));
    Ok(output)
}

pub fn project_one(polynomials: &[Poly], variable: usize) -> Result<Vec<Poly>> {
    factors(&projection_polynomials(polynomials, variable)?)
}

pub fn project(algebra: &Algebra, polynomials: &[Poly]) -> Result<Projection> {
    project_with_factorizer(algebra, polynomials, |p| algebra.factors(p))
}

pub fn project_checked(algebra: &Algebra, polynomials: &[Poly]) -> Result<Projection> {
    project_with_factorizer(algebra, polynomials, |p| algebra.factors_checked(p))
}

fn project_with_factorizer(
    algebra: &Algebra,
    polynomials: &[Poly],
    factorize: impl Fn(&[Poly]) -> Result<Vec<Poly>>,
) -> Result<Projection> {
    ensure!(
        polynomials
            .iter()
            .all(|p| p.variables() == &algebra.variables),
        "projection input has a different variable map"
    );
    let n = algebra.names.len();
    if n == 0 {
        return Ok(Projection {
            levels: vec![],
            steps: vec![],
        });
    }
    let mut levels = vec![Vec::new(); n];
    let mut steps = Vec::with_capacity(n.saturating_sub(1));
    levels[n - 1] = factorize(polynomials)?;
    for variable in (1..n).rev() {
        let started = Instant::now();
        let projected = factorize(&projection_polynomials(&levels[variable], variable)?)?;
        steps.push(ProjectionStep {
            variable,
            input_count: levels[variable].len(),
            output_count: projected.len(),
            input_terms: levels[variable].iter().map(Poly::nterms).sum(),
            output_terms: projected.iter().map(Poly::nterms).sum(),
            elapsed_seconds: started.elapsed().as_secs_f64(),
        });
        levels[variable - 1] = projected;
    }
    for (variable, level) in levels.iter_mut().enumerate() {
        level.retain(|p| p.degree(variable) > 0);
    }
    Ok(Projection { levels, steps })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn algebra() -> Algebra {
        Algebra::new(&["x".into(), "y".into()]).unwrap()
    }

    #[test]
    fn deterministic_resultant_preserves_rational_scale_and_sign() {
        let a = algebra();
        // Reduce the second polynomial modulo y^2-x: x^3+x*y+1.
        // Its product at the two conjugate roots is x^6+x^3+1.
        let left = a.parse("(2/3)*(y^2-x)").unwrap();
        let right = a.parse("(-3/5)*(y^6+x*y+1)").unwrap();
        assert_eq!(
            exact_resultant(&left, &right, 1).unwrap(),
            a.parse("(64/2025)*(x^6+x^3+1)").unwrap()
        );
        assert_eq!(
            exact_resultant(&a.parse("y^3+x").unwrap(), &a.parse("y+2").unwrap(), 1).unwrap(),
            a.parse("8-x").unwrap()
        );
        assert_eq!(
            exact_resultant(&a.parse("y+2").unwrap(), &a.parse("y^3+x").unwrap(), 1).unwrap(),
            a.parse("x-8").unwrap()
        );
        assert_eq!(
            exact_resultant(&a.parse("2*y+x").unwrap(), &a.parse("3*y+1").unwrap(), 1).unwrap(),
            a.parse("2-3*x").unwrap()
        );
        assert_eq!(
            exact_resultant(&a.parse("y^3-x").unwrap(), &a.parse("y^3-1").unwrap(), 1).unwrap(),
            a.parse("(x-1)^3").unwrap()
        );
        assert!(exact_resultant(&left, &right, 2).is_err());
        let other = Algebra::new(&["x".into(), "z".into()]).unwrap();
        assert!(exact_resultant(&left, &other.parse("z+1").unwrap(), 1).is_err());
    }

    #[test]
    fn discriminant_includes_leading_coefficient_division() {
        let a = algebra();
        assert_eq!(
            discriminant(&a.parse("x*y^2+y+1").unwrap(), 1).unwrap(),
            a.parse("1-4*x").unwrap()
        );
        assert_eq!(
            discriminant(&a.parse("y^3+x").unwrap(), 1).unwrap(),
            a.parse("-27*x^2").unwrap()
        );
    }

    fn independent_discriminant(polynomial: &Poly, variable: usize) -> Poly {
        let degree = polynomial.degree(variable);
        let resultant =
            exact_resultant(polynomial, &polynomial.derivative(variable), variable).unwrap();
        let (mut value, remainder) =
            resultant.quot_rem(&polynomial.univariate_lcoeff(variable), false);
        assert!(remainder.is_zero());
        if degree % 4 == 2 || degree % 4 == 3 {
            value = -value;
        }
        value
    }

    #[test]
    fn low_degree_identities_equal_generic_resultant_definitions() {
        let algebra = Algebra::new(&["a", "b", "c", "d", "e", "y"].map(String::from)).unwrap();
        for expression in [
            "a*y^2+b*y+c",
            "a*y^3+b*y^2+c*y+d",
            "a*y^4+b*y^3+c*y^2+d*y+e",
            "((a-1)/3)*(y^4+(2/5)*y+7)",
            "(y^2+a)^2",
            "(y-a)^3*(y+b)",
        ] {
            let polynomial = algebra.parse(expression).unwrap();
            assert_eq!(
                discriminant(&polynomial, 5).unwrap(),
                independent_discriminant(&polynomial, 5),
                "{expression}"
            );
        }
        let polynomial = algebra.parse("((a-1)/3)*(y^4+(2/5)*y+7)").unwrap();
        assert_eq!(
            discriminant(&polynomial, 5).unwrap(),
            algebra.parse("((a-1)/3)^6*(256*7^3-27*(2/5)^4)").unwrap()
        );
    }

    #[test]
    fn low_degree_formal_discriminants_handle_vanishing_leading_coefficients() {
        let algebra = Algebra::new(&["a", "b", "c", "d", "e", "y"].map(String::from)).unwrap();
        let zero = Rational::from(0);
        for expression in [
            "a*y^2+b*y+c",
            "a*y^3+b*y^2+c*y+d",
            "a*y^4+b*y^3+c*y^2+d*y+e",
        ] {
            let polynomial = algebra.parse(expression).unwrap();
            let formal_discriminant = discriminant(&polynomial, 5).unwrap();
            let lower_degree = polynomial.replace(0, &zero);
            // Formal degree-n discriminants specialize to b^2 times the
            // degree-(n-1) discriminant, rather than commuting with degree drop.
            assert_eq!(
                formal_discriminant.replace(0, &zero),
                &algebra.parse("b^2").unwrap() * &discriminant(&lower_degree, 5).unwrap()
            );
            assert!(
                formal_discriminant
                    .replace(0, &zero)
                    .replace(1, &zero)
                    .is_zero()
            );
        }
        assert_eq!(
            discriminant(&algebra.parse("b*y+c").unwrap(), 5).unwrap(),
            algebra.parse("1").unwrap()
        );
        assert!(
            discriminant(&algebra.parse("c").unwrap(), 5)
                .unwrap()
                .is_zero()
        );
    }

    #[test]
    fn projection_retains_coefficients_and_pairwise_intersections() {
        let a = algebra();
        let input = [
            a.parse("x*y+1").unwrap(),
            a.parse("y-x").unwrap(),
            a.parse("x-2").unwrap(),
        ];
        let result = project(&a, &input).unwrap();
        assert_eq!(result.levels[1].len(), 2);
        let expected = factors(&[
            a.parse("x").unwrap(),
            a.parse("x^2+1").unwrap(),
            a.parse("x-2").unwrap(),
        ])
        .unwrap();
        assert_eq!(result.levels[0], expected);
    }

    #[test]
    fn repeated_factors_are_removed_before_discriminants() {
        let a = algebra();
        let result = project(&a, &[a.parse("(y^2-x)^2").unwrap()]).unwrap();
        assert_eq!(
            result.levels[1],
            vec![canonical(&a.parse("y^2-x").unwrap())]
        );
        assert_eq!(result.levels[0], vec![a.parse("x").unwrap()]);
    }

    #[test]
    fn zero_dimensional_projection_has_no_levels() {
        let a = Algebra::new(&[]).unwrap();
        let result = project(&a, &[a.parse("2").unwrap(), a.parse("-1/3").unwrap()]).unwrap();
        assert!(result.levels.is_empty());
        assert!(result.steps.is_empty());
    }
}
