//! Bounded exact PSD quadratic components with a strictly positive residual.
use super::{
    CertificateMethod, CertificateOptions, Domain, ProofBudget, WeightedResidualTerm,
    shift_to_positive_oriented,
    weighted_squares::{
        bounded, height, positive_literal, positive_support, replay_residual, strict_residual,
    },
};
use crate::algebra::{Algebra, Poly, rational};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use symbolica::prelude::*;

const MAX_TERMS: usize = 256;
const MAX_HEIGHT: u64 = 512;
const MAX_SQUARES: usize = 16;
const MAX_ORIENTATIONS: usize = 16;
const MAX_MATRIX_UPDATES: usize = 100_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuadraticComponentSquare {
    pub weight: String,
    pub coefficients: Vec<String>,
}

fn signed_literal(text: &str) -> Result<Rational> {
    // Zero is the sole nonpositive unsigned spelling accepted here. Other
    // literals share the weighted rule's bounded grammar before arithmetic.
    if text == "0" {
        return Ok(Rational::zero());
    }
    match text.strip_prefix('-') {
        Some(magnitude) => Ok(-positive_literal(magnitude)?),
        None => positive_literal(text),
    }
}

fn replay_square(
    template: &Poly,
    square: &QuadraticComponentSquare,
    positive: &[bool],
) -> Result<Poly> {
    ensure!(
        square.coefficients.len() == template.nvars(),
        "quadratic square variable map"
    );
    let weight = positive_literal(&square.weight)?;
    let mut form = template.zero();
    for (i, text) in square.coefficients.iter().enumerate() {
        let coefficient = signed_literal(text)?;
        if coefficient.is_zero() {
            continue;
        }
        ensure!(positive[i], "unrestricted coordinate in quadratic square");
        let mut powers = vec![0; template.nvars()];
        powers[i] = 1;
        form.append_monomial(coefficient, &powers);
    }
    ensure!(!form.is_zero(), "zero quadratic linear form");
    let expanded = form.pow(2).mul_coeff(weight);
    ensure!(bounded(&expanded), "quadratic square size budget");
    Ok(expanded)
}

fn root(parent: &[usize], mut i: usize) -> usize {
    while parent[i] != i {
        i = parent[i];
    }
    i
}

/// Proposals only: every returned square is independently reconstructed.
fn extract(
    p: &Poly,
    budget: &mut ProofBudget,
    updates: &mut usize,
) -> Option<Vec<QuadraticComponentSquare>> {
    if !budget.spend(p.nterms().saturating_mul(p.nvars())) {
        return None;
    }
    let mut parent: Vec<_> = (0..p.nvars()).collect();
    for (coefficient, powers) in p.coefficients.iter().zip(p.exponents_iter()) {
        if *coefficient >= Rational::zero() || powers.iter().sum::<u32>() != 2 {
            continue;
        }
        let active: Vec<_> = powers
            .iter()
            .enumerate()
            .filter_map(|(i, &power)| (power == 1).then_some(i))
            .collect();
        if let [i, j] = active.as_slice() {
            let first = root(&parent, *i);
            let second = root(&parent, *j);
            parent[second] = first;
        }
    }
    let mut groups = BTreeMap::<usize, Vec<usize>>::new();
    for i in 0..p.nvars() {
        groups.entry(root(&parent, i)).or_default().push(i);
    }
    let mut squares = Vec::new();
    for axes in groups.values().filter(|axes| axes.len() > 1) {
        if !budget.spend(p.nterms().saturating_mul(p.nvars())) {
            return None;
        }
        let n = axes.len();
        let mut matrix = vec![vec![Rational::zero(); n]; n];
        for (coefficient, powers) in p.coefficients.iter().zip(p.exponents_iter()) {
            if powers.iter().sum::<u32>() != 2
                || powers
                    .iter()
                    .enumerate()
                    .any(|(i, power)| *power != 0 && !axes.contains(&i))
            {
                continue;
            }
            let factors: Vec<_> = axes
                .iter()
                .enumerate()
                .flat_map(|(i, &v)| std::iter::repeat_n(i, powers[v] as usize))
                .collect();
            let [i, j] = factors.as_slice() else {
                return None;
            };
            let value = if i == j {
                coefficient.clone()
            } else {
                coefficient.clone() / Rational::from(2)
            };
            if height(&value) > MAX_HEIGHT {
                return None;
            }
            matrix[*i][*j] += value.clone();
            if i != j {
                matrix[*j][*i] += value;
            }
            if height(&matrix[*i][*j]) > MAX_HEIGHT {
                return None;
            }
        }
        for k in 0..n {
            if !budget.spend(n.saturating_add(1)) {
                return None;
            }
            let pivot = matrix[k][k].clone();
            if pivot < Rational::zero() {
                return None;
            }
            if pivot.is_zero() {
                // For a PSD form an exact zero diagonal implies a zero row.
                if (k..n).any(|i| !matrix[i][k].is_zero()) {
                    return None;
                }
                continue;
            }
            if squares.len() >= MAX_SQUARES {
                return None;
            }
            let mut coefficients = vec![String::from("0"); p.nvars()];
            coefficients[axes[k]] = "1".into();
            for i in k + 1..n {
                let entry = &matrix[i][k] / &pivot;
                if height(&entry) > MAX_HEIGHT {
                    return None;
                }
                coefficients[axes[i]] = entry.to_string();
            }
            squares.push(QuadraticComponentSquare {
                weight: pivot.to_string(),
                coefficients,
            });
            for i in k + 1..n {
                for j in i..n {
                    if *updates >= MAX_MATRIX_UPDATES || !budget.spend(8) {
                        return None;
                    }
                    *updates += 1;
                    let correction = (&matrix[i][k] * &matrix[j][k]) / &pivot;
                    if height(&correction) > MAX_HEIGHT {
                        return None;
                    }
                    matrix[i][j] -= correction;
                    if height(&matrix[i][j]) > MAX_HEIGHT {
                        return None;
                    }
                    matrix[j][i] = matrix[i][j].clone();
                }
            }
        }
    }
    (!squares.is_empty()).then_some(squares)
}

fn reconstruct(
    signed: &Poly,
    positive: &[bool],
    squares: &[QuadraticComponentSquare],
    residual: &Poly,
    budget: &mut ProofBudget,
) -> Result<bool> {
    if squares.is_empty()
        || squares.len() > MAX_SQUARES
        || !bounded(residual)
        || !strict_residual(residual, positive)
        || !budget.spend(residual.nterms().saturating_mul(residual.nvars()))
    {
        return Ok(false);
    }
    let mut reconstructed = residual.clone();
    for square in squares {
        if !budget.spend(
            signed
                .nvars()
                .saturating_mul(signed.nvars())
                .saturating_add(reconstructed.nterms()),
        ) {
            return Ok(false);
        }
        reconstructed = reconstructed + replay_square(signed, square, positive)?;
        if !bounded(&reconstructed) {
            return Ok(false);
        }
    }
    Ok(reconstructed == *signed)
}

pub(super) fn discover(
    algebra: &Algebra,
    polynomial: &Poly,
    domain: &Domain,
    options: &CertificateOptions,
) -> Result<Option<(i8, CertificateMethod)>> {
    if options.quadratic_component_candidates == 0 || !bounded(polynomial) {
        return Ok(None);
    }
    let limit = options.max_terms.min(MAX_TERMS);
    if polynomial.nterms() > limit {
        return Ok(None);
    }
    let mut alternatives = Vec::new();
    for (v, bounds) in domain.bounds.iter().enumerate() {
        if polynomial.degree(v) == 0 {
            continue;
        }
        for bound in [&bounds.lower, &bounds.upper].into_iter().flatten() {
            if height(&rational(bound)?) > MAX_HEIGHT {
                return Ok(None);
            }
        }
        if bounds.lower.is_some() && bounds.upper.is_some() {
            alternatives.push(v);
        }
    }
    let mut orientations = vec![vec![]];
    for &v in alternatives.iter().take(MAX_ORIENTATIONS - 2) {
        orientations.push(vec![v]);
    }
    if alternatives.len() > 1 {
        orientations.push(alternatives);
    }
    let mut budget = ProofBudget::new(options);
    let mut attempts = 0usize;
    let mut updates = 0usize;
    for upper_variables in orientations {
        if !budget.spend(polynomial.nterms().saturating_mul(polynomial.nvars())) {
            return Ok(None);
        }
        let Some((shifted, positive)) =
            shift_to_positive_oriented(algebra, polynomial, domain, &upper_variables, limit)?
        else {
            continue;
        };
        if !bounded(&shifted) || !positive_support(&shifted, &positive) {
            continue;
        }
        for sign in [1i8, -1] {
            if attempts >= options.quadratic_component_candidates || budget.work == 0 {
                return Ok(None);
            }
            attempts += 1;
            let signed = shifted.clone().mul_coeff(Rational::from(sign as i64));
            let Some(squares) = extract(&signed, &mut budget, &mut updates) else {
                continue;
            };
            let mut residual = signed.clone();
            let mut valid = true;
            for square in &squares {
                if !budget.spend(
                    polynomial
                        .nvars()
                        .saturating_mul(polynomial.nvars())
                        .saturating_add(residual.nterms()),
                ) {
                    return Ok(None);
                }
                let Ok(expanded) = replay_square(polynomial, square, &positive) else {
                    valid = false;
                    break;
                };
                residual = residual - expanded;
                if !bounded(&residual) {
                    valid = false;
                    break;
                }
            }
            if valid
                && residual.nterms() <= limit
                && reconstruct(&signed, &positive, &squares, &residual, &mut budget)?
            {
                return Ok(Some((
                    sign,
                    CertificateMethod::QuadraticComponentsPlusPositiveResidual {
                        upper_variables: upper_variables.clone(),
                        squares,
                        residual: residual
                            .coefficients
                            .iter()
                            .zip(residual.exponents_iter())
                            .map(|(c, e)| WeightedResidualTerm {
                                coefficient: c.to_string(),
                                exponents: e.to_vec(),
                            })
                            .collect(),
                    },
                )));
            }
        }
    }
    Ok(None)
}

pub(super) fn verify(
    algebra: &Algebra,
    polynomial: &Poly,
    domain: &Domain,
    sign: i8,
    upper_variables: &[usize],
    squares: &[QuadraticComponentSquare],
    residual: &[WeightedResidualTerm],
    options: &CertificateOptions,
) -> Result<bool> {
    ensure!(
        options.quadratic_component_candidates > 0,
        "quadratic component certificate is disabled"
    );
    let limit = options.max_terms.min(MAX_TERMS);
    ensure!(
        bounded(polynomial) && polynomial.nterms() <= limit,
        "quadratic component polynomial budget"
    );
    ensure!(
        !squares.is_empty() && squares.len() <= MAX_SQUARES,
        "quadratic component square count"
    );
    for (v, bounds) in domain.bounds.iter().enumerate() {
        if polynomial.degree(v) == 0 {
            continue;
        }
        for bound in [&bounds.lower, &bounds.upper].into_iter().flatten() {
            ensure!(
                height(&rational(bound)?) <= MAX_HEIGHT,
                "quadratic component bound height"
            );
        }
    }
    let mut budget = ProofBudget::new(options);
    if !budget.spend(polynomial.nterms().saturating_mul(polynomial.nvars())) {
        return Ok(false);
    }
    let signed = polynomial.clone().mul_coeff(Rational::from(sign as i64));
    let Some((shifted, positive)) =
        shift_to_positive_oriented(algebra, &signed, domain, upper_variables, limit)?
    else {
        return Ok(false);
    };
    ensure!(
        bounded(&shifted) && positive_support(&shifted, &positive),
        "quadratic component shifted domain"
    );
    let residual = replay_residual(&shifted, residual, limit)?;
    reconstruct(&shifted, &positive, squares, &residual, &mut budget)
}
