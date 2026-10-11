//! Variable ordering with the same elimination direction and tie breaks as gcad.
//!
//! Public orders are in lifting order. Projection removes variables in reverse.
//! Heuristics only choose an order; they never delete solution regions.

use crate::algebra::{Algebra, Poly};
use crate::projection::projection_polynomials;
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::str::FromStr;
use symbolica::domains::rational::Rational;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderingMethod {
    Given,
    Mods,
    Sotd,
    T1,
    Brown,
    Sparsity,
}

impl FromStr for OrderingMethod {
    type Err = anyhow::Error;
    fn from_str(name: &str) -> Result<Self> {
        match name {
            "given" => Ok(Self::Given),
            "mods" | "gmods" | "greedy_mods" => Ok(Self::Mods),
            "sotd" | "greedy_sotd" => Ok(Self::Sotd),
            "t1" | "greedy_t1" => Ok(Self::T1),
            "brown" => Ok(Self::Brown),
            "sparsity" | "chordal" => Ok(Self::Sparsity),
            _ => bail!("unknown ordering method {name:?}"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProjectionChoice {
    pub variable: usize,
    pub projected: Vec<Poly>,
}

#[derive(Clone, Debug)]
pub struct OrderingResult {
    /// Variable indices, from the first lifted to the last lifted variable.
    pub order: Vec<usize>,
    /// Factorization and selected projection work can be reused by the solver.
    pub initial_basis: Vec<Poly>,
    pub projections: Vec<ProjectionChoice>,
}

pub fn validate_groups(groups: &[Vec<usize>], variables: usize) -> Result<()> {
    let listed: Vec<_> = groups.iter().flatten().copied().collect();
    ensure!(
        listed.len() == variables,
        "variable groups must contain every variable once"
    );
    ensure!(
        listed.iter().all(|&v| v < variables),
        "variable group index is out of range"
    );
    ensure!(
        listed.iter().copied().collect::<BTreeSet<_>>().len() == variables,
        "duplicate variable in variable groups"
    );
    Ok(())
}

/// Faithful greedy gcad heuristics, followed by Brown and minimum-fill alternatives.
///
/// SOTD chooses the *last* equal candidate; MODS and T1 choose the *first*.
/// Groups are interpreted in lifting order, so the last group is eliminated first.
pub fn choose_order(
    algebra: &Algebra,
    positives: &[Poly],
    groups: &[Vec<usize>],
    method: OrderingMethod,
) -> Result<OrderingResult> {
    choose_order_from_basis(
        algebra,
        algebra.factors(positives)?,
        groups,
        method,
        |basis, axis| algebra.factors(&projection_polynomials(basis, axis)?),
    )
}

/// Apply the same conventions with a caller-owned projection cache. The basis
/// must be canonical square-free factors in the algebra's stable variable map.
pub fn choose_order_from_basis(
    algebra: &Algebra,
    initial_basis: Vec<Poly>,
    groups: &[Vec<usize>],
    method: OrderingMethod,
    mut project_step: impl FnMut(&[Poly], usize) -> Result<Vec<Poly>>,
) -> Result<OrderingResult> {
    validate_groups(groups, algebra.names.len())?;
    ensure!(
        initial_basis
            .iter()
            .all(|p| p.variables() == &algebra.variables),
        "ordering input has a different variable map"
    );
    if method == OrderingMethod::Given {
        return Ok(OrderingResult {
            order: groups.iter().flatten().copied().collect(),
            initial_basis: vec![],
            projections: vec![],
        });
    }
    let mut basis = initial_basis.clone();
    let mut eliminated = Vec::with_capacity(algebra.names.len());
    let mut projections = Vec::new();
    for group in groups.iter().rev() {
        let mut remaining = group.clone();
        while !remaining.is_empty() {
            let total_left = algebra.names.len() - eliminated.len();
            let mut selected_projection = None;
            let chosen = if remaining.len() == 1 {
                remaining[0]
            } else if method == OrderingMethod::Sotd {
                let mut best = None;
                for &variable in &remaining {
                    let candidate = project_step(&basis, variable)?;
                    let score = sotd(&candidate);
                    if best
                        .as_ref()
                        .is_none_or(|&(best_score, _)| score <= best_score)
                    {
                        best = Some((score, variable));
                        selected_projection = Some(candidate);
                    }
                }
                best.expect("nonempty candidate group").1
            } else {
                rank_candidates(&basis, &remaining, method)[0]
            };
            // Match gcad's skipped final projection for MODS/T1. A singleton
            // group still projects when another group remains.
            let need_projection = total_left > 1
                && (remaining.len() == 1 || total_left > 2 || method == OrderingMethod::Sotd);
            if need_projection {
                basis = match selected_projection {
                    Some(projected) => projected,
                    None => project_step(&basis, chosen)?,
                };
                projections.push(ProjectionChoice {
                    variable: chosen,
                    projected: basis.clone(),
                });
            }
            eliminated.push(chosen);
            remaining.retain(|&v| v != chosen);
        }
    }
    eliminated.reverse();
    Ok(OrderingResult {
        order: eliminated,
        initial_basis,
        projections,
    })
}

pub fn sotd(polys: &[Poly]) -> u128 {
    polys
        .iter()
        .flat_map(Poly::exponents_iter)
        .map(|term| term.iter().map(|&d| u128::from(d)).sum::<u128>())
        .sum()
}

pub fn degree_sum(polys: &[Poly], variable: usize) -> u128 {
    polys.iter().map(|p| u128::from(p.degree(variable))).sum()
}

/// The average-of-averages is compared exactly, not through floating point.
/// Its common factor 1 / number_of_polynomials can be omitted for comparisons.
pub fn t1_score(polys: &[Poly], variable: usize) -> (u128, Rational, u128) {
    let mut average_sum = Rational::from(0);
    let mut all_degrees = 0u128;
    for p in polys {
        let degree_sum: u64 = p.exponents_iter().map(|m| u64::from(m[variable])).sum();
        all_degrees += u128::from(degree_sum);
        if p.nterms() != 0 {
            average_sum += Rational::from((degree_sum, p.nterms() as u64));
        }
    }
    (degree_sum(polys, variable), average_sum, all_degrees)
}

/// Brown: degree in the variable; largest total degree of a term containing
/// it; number of terms containing it. Zero-degree variables sort first.
pub fn brown_score(polys: &[Poly], variable: usize) -> (u32, u128, usize) {
    let mut max_total = 0;
    let mut occurrences = 0;
    for term in polys.iter().flat_map(Poly::exponents_iter) {
        if term[variable] > 0 {
            occurrences += 1;
            max_total = max_total.max(term.iter().map(|&d| u128::from(d)).sum());
        }
    }
    (
        polys.iter().map(|p| p.degree(variable)).max().unwrap_or(0),
        max_total,
        occurrences,
    )
}

/// Correlative sparsity graph: variables occurring anywhere in the same
/// polynomial are adjacent. Minimum fill is a heuristic, not a pruning rule.
pub fn sparsity_score(polys: &[Poly], variable: usize) -> (usize, usize, u32, u128, usize) {
    let n = polys.first().map_or(variable + 1, |p| p.nvars());
    let mut adjacency = vec![BTreeSet::new(); n];
    for p in polys {
        let support: Vec<_> = (0..n).filter(|&v| p.degree(v) > 0).collect();
        for &a in &support {
            for &b in &support {
                if a != b {
                    adjacency[a].insert(b);
                }
            }
        }
    }
    let neighbors: Vec<_> = adjacency[variable].iter().copied().collect();
    let mut fill = 0;
    for (i, &a) in neighbors.iter().enumerate() {
        for &b in &neighbors[i + 1..] {
            if !adjacency[a].contains(&b) {
                fill += 1;
            }
        }
    }
    let (degree, total, terms) = brown_score(polys, variable);
    (fill, neighbors.len(), degree, total, terms)
}

/// Stable ordering of legal next elimination variables using only the current
/// polynomial set. SOTD needs actual projections and is deliberately excluded.
pub fn rank_candidates(polys: &[Poly], candidates: &[usize], method: OrderingMethod) -> Vec<usize> {
    let mut ranked = candidates.to_vec();
    match method {
        OrderingMethod::Mods => ranked.sort_by_key(|&v| degree_sum(polys, v)),
        OrderingMethod::T1 => ranked.sort_by_cached_key(|&v| t1_score(polys, v)),
        OrderingMethod::Brown => ranked.sort_by_key(|&v| brown_score(polys, v)),
        OrderingMethod::Sparsity => ranked.sort_by_key(|&v| sparsity_score(polys, v)),
        OrderingMethod::Given | OrderingMethod::Sotd => {}
    }
    ranked
}

/// Inexpensive complete seeds for bounded search. Unlike choose_order these
/// use the input features throughout; no unbounded algebra is hidden here.
pub fn cheap_seed_orders(polys: &[Poly], groups: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut orders = vec![groups.iter().flatten().copied().collect()];
    for method in [
        OrderingMethod::Mods,
        OrderingMethod::T1,
        OrderingMethod::Brown,
        OrderingMethod::Sparsity,
    ] {
        let mut elimination = Vec::new();
        for group in groups.iter().rev() {
            elimination.extend(rank_candidates(polys, group, method));
        }
        elimination.reverse();
        if !orders.contains(&elimination) {
            orders.push(elimination);
        }
    }
    orders
}
