//! Exact polynomial operations. All arithmetic is delegated to Symbolica.

use std::{collections::HashSet, sync::Arc};

use anyhow::{Result, anyhow, bail, ensure};
use symbolica::{domains::InternalOrdering, poly::factor::Factorize, prelude::*};

pub type Poly = MultivariatePolynomial<Q, u32>;

/// Symbolica's dense univariate conversion indexes exponents through i32.
pub const MAX_DEGREE: u64 = i32::MAX as u64 - 1;

#[derive(Clone, Debug)]
pub struct Algebra {
    pub names: Vec<String>,
    pub variables: Arc<Vec<PolyVariable>>,
}

impl Algebra {
    pub fn new(names: &[String]) -> Result<Self> {
        let mut seen = HashSet::new();
        let mut variables = Vec::with_capacity(names.len());
        for name in names {
            ensure!(
                !name.is_empty()
                    && name
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                    && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "invalid variable name {name:?}; use a plain identifier"
            );
            ensure!(seen.insert(name), "duplicate variable {name}");
            variables.push(
                Symbol::parse(name, "symgcad")
                    .map_err(|e| anyhow!(e))?
                    .into(),
            );
        }
        Ok(Self {
            names: names.to_vec(),
            variables: Arc::new(variables),
        })
    }

    /// Parse polynomial syntax with exact rational coefficients. Unknown symbols,
    /// functions and negative/noninteger powers are rejected, rather than turned
    /// into extra polynomial variables by Symbolica's general conversion API.
    pub fn parse(&self, expression: &str) -> Result<Poly> {
        let atom = Atom::parse(expression.replace("**", "^"), "symgcad", Default::default())
            .map_err(|e| anyhow!("invalid polynomial: {e}"))?;
        self.degree_bounds(atom.as_view())?;
        let polynomial: Poly = atom
            .try_to_polynomial(&Q, self.variables.clone())
            .map_err(|e| anyhow!("invalid rational polynomial: {e}"))?;
        ensure!(
            polynomial.variables() == &self.variables,
            "expression contains undeclared variables"
        );
        Ok(polynomial)
    }

    fn degree_bounds(&self, atom: AtomView<'_>) -> Result<Vec<u64>> {
        let mut bounds = vec![0; self.names.len()];
        match atom {
            AtomView::Num(_) => {
                Rational::try_from(atom)
                    .map_err(|_| anyhow!("coefficients must be exact rational numbers"))?;
            }
            AtomView::Var(v) => {
                let variable = PolyVariable::from(v.get_symbol());
                let i = self
                    .variables
                    .iter()
                    .position(|x| x == &variable)
                    .ok_or_else(|| anyhow!("undeclared variable {atom}"))?;
                bounds[i] = 1;
            }
            AtomView::Add(sum) => {
                for term in sum {
                    for (bound, degree) in bounds.iter_mut().zip(self.degree_bounds(term)?) {
                        *bound = (*bound).max(degree);
                    }
                }
            }
            AtomView::Mul(product) => {
                for factor in product {
                    for (bound, degree) in bounds.iter_mut().zip(self.degree_bounds(factor)?) {
                        *bound = bound
                            .checked_add(degree)
                            .ok_or_else(|| anyhow!("polynomial degree overflow"))?;
                    }
                }
            }
            AtomView::Pow(power) => {
                let (base, exponent) = power.get_base_exp();
                let exponent = Rational::try_from(exponent)
                    .map_err(|_| anyhow!("polynomial exponent must be an integer"))?;
                ensure!(
                    exponent.is_integer() && !exponent.is_negative(),
                    "polynomial exponent must be a nonnegative integer"
                );
                let exponent: u64 = exponent
                    .numerator()
                    .to_string()
                    .parse()
                    .map_err(|_| anyhow!("polynomial exponent is too large"))?;
                bounds = self.degree_bounds(base)?;
                for bound in &mut bounds {
                    *bound = bound
                        .checked_mul(exponent)
                        .ok_or_else(|| anyhow!("polynomial degree overflow"))?;
                }
            }
            AtomView::Fun(_) => {
                bail!("functions are not allowed in rational polynomial input: {atom}")
            }
        }
        ensure!(
            bounds.iter().all(|x| *x <= MAX_DEGREE),
            "polynomial degree exceeds Symbolica's supported dense conversion range"
        );
        Ok(bounds)
    }

    /// Stable, parseable expression independent of Symbolica's namespace printer.
    pub fn format(&self, polynomial: &Poly) -> String {
        if polynomial.is_zero() {
            return "0".into();
        }
        if polynomial.nvars() == 0 {
            return polynomial.get_constant().to_string();
        }
        polynomial
            .coefficients
            .iter()
            .zip(polynomial.exponents_iter())
            .map(|(coefficient, exponents)| {
                let mut term = format!("({coefficient})");
                for (name, exponent) in self.names.iter().zip(exponents) {
                    if *exponent > 0 {
                        term.push('*');
                        term.push_str(name);
                        if *exponent > 1 {
                            term.push('^');
                            term.push_str(&exponent.to_string());
                        }
                    }
                }
                term
            })
            .collect::<Vec<_>>()
            .join("+")
    }

    /// Specialize preceding coordinates exactly and retain the current axis.
    pub fn specialize_univariate(
        &self,
        polynomial: &Poly,
        variable: usize,
        prefix: &[Rational],
    ) -> Result<UnivariatePolynomial<Q>> {
        ensure!(variable < self.names.len(), "variable index out of range");
        ensure!(
            prefix.len() == variable,
            "sample prefix must have one value for every earlier variable"
        );
        ensure!(
            polynomial.variables() == &self.variables,
            "polynomial has a different variable map"
        );
        ensure!(
            (variable + 1..self.names.len()).all(|i| polynomial.degree(i) == 0),
            "polynomial depends on an unassigned later variable"
        );
        let mut specialized = polynomial.clone();
        for (i, value) in prefix.iter().enumerate() {
            specialized = specialized.replace(i, value);
        }
        ensure!(
            specialized.degree(variable) <= u16::MAX as u32,
            "specialized degree exceeds Symbolica's certified real-root isolation exponent range"
        );
        Ok(specialized.to_univariate_from_univariate(variable))
    }
}

/// Primitive integer representative, with positive leading coefficient.
/// Scalars and multiplicities do not affect the zero set used for projection.
pub fn canonical(polynomial: &Poly) -> Poly {
    if polynomial.is_zero() {
        return polynomial.clone();
    }
    let primitive = polynomial.clone().make_primitive();
    if primitive.lcoeff().is_negative() {
        -primitive
    } else {
        primitive
    }
}

/// The original GCAD SFRP: unique primitive irreducible factors, with no
/// multiplicities or constants. Sorting makes ordering/cache behavior stable.
pub fn factors(polynomials: &[Poly]) -> Result<Vec<Poly>> {
    let mut unique = HashSet::new();
    for polynomial in polynomials {
        if polynomial.is_constant() {
            continue;
        }
        for (factor, _) in polynomial.factor() {
            if !factor.is_constant() {
                unique.insert(canonical(&factor));
            }
        }
    }
    let mut result: Vec<_> = unique.into_iter().collect();
    result.sort_by(|a, b| a.internal_cmp(b));
    Ok(result)
}

pub fn rational(text: &str) -> Result<Rational> {
    let atom = Atom::parse(text, "symgcad", Default::default()).map_err(|e| anyhow!(e))?;
    Rational::try_from(atom.as_view())
        .map_err(|_| anyhow!("expected an exact rational number, got {text:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn algebra() -> Algebra {
        Algebra::new(&["x".into(), "y".into()]).unwrap()
    }

    #[test]
    fn polynomial_roundtrip_and_exact_specialization() {
        let a = algebra();
        let p = a.parse("(x+1)^2/3-y/7").unwrap();
        assert_eq!(a.parse(&a.format(&p)).unwrap(), p);
        let u = a
            .specialize_univariate(&p, 1, &[rational("1/2").unwrap()])
            .unwrap();
        assert_eq!(
            u.evaluate(&rational("7").unwrap()),
            rational("-1/4").unwrap()
        );
    }

    #[test]
    fn rejects_nonpolynomial_and_undeclared_input() {
        let a = algebra();
        for expression in ["z+1", "1/x", "sqrt(x)", "x^(1/2)", "x^2147483647"] {
            assert!(a.parse(expression).is_err(), "{expression}");
        }
    }

    #[test]
    fn factor_basis_removes_scalars_and_multiplicities() {
        let a = algebra();
        let fs = factors(&[
            a.parse("-3*(x-y)^2*(x+y)/7").unwrap(),
            a.parse("2*(y-x)").unwrap(),
        ])
        .unwrap();
        assert_eq!(fs.len(), 2);
        assert!(fs.contains(&canonical(&a.parse("x-y").unwrap())));
        assert!(fs.contains(&canonical(&a.parse("x+y").unwrap())));
        assert_eq!(
            canonical(&a.parse("-3*x/7-6*y/7").unwrap()),
            a.parse("x+2*y").unwrap()
        );
    }

    #[test]
    fn exact_constants_allow_zero_variables() {
        let a = Algebra::new(&[]).unwrap();
        let p = a.parse("-7/3").unwrap();
        assert!(p.is_constant());
        assert_eq!(p.get_constant(), rational("-7/3").unwrap());
        assert_eq!(a.parse(&a.format(&p)).unwrap(), p);
        assert_eq!(a.parse("0").unwrap().replace_all(&[]), Rational::zero());
        assert!(a.parse("x").is_err());
    }

    #[test]
    fn rational_input_rejects_undefined_and_inexact_values() {
        for text in ["1/0", "0/0", "0.1", "NaN", "inf", "1.2.3"] {
            assert!(rational(text).is_err(), "{text}");
        }
        assert_eq!(rational("(7-2)/15").unwrap(), rational("1/3").unwrap());
    }
}
