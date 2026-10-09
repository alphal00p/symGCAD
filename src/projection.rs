//! The original GCAD generic projection operator (SFRP after each PR step).

use std::{collections::HashSet, time::Instant};

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use symbolica::{domains::InternalOrdering, prelude::*};

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
    let resultant = <Q as PolynomialResultant<u32>>::resultant(polynomial, &derivative, variable);
    let (mut quotient, remainder) =
        resultant.quot_rem(&polynomial.univariate_lcoeff(variable), false);
    ensure!(
        remainder.is_zero(),
        "discriminant leading-coefficient division was not exact"
    );
    if degree % 4 == 2 || degree % 4 == 3 {
        quotient = -quotient;
    }
    Ok(quotient)
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
            check_resultant_degrees(polynomial, other, variable)?;
            add(<Q as PolynomialResultant<u32>>::resultant(
                polynomial, other, variable,
            ));
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
    levels[n - 1] = factors(polynomials)?;
    for variable in (1..n).rev() {
        let started = Instant::now();
        let projected = project_one(&levels[variable], variable)?;
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
