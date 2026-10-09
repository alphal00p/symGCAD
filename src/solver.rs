use crate::{
    Problem,
    algebra::{Algebra, Poly},
    ordering::{OrderingMethod, choose_order},
    output::{Axis, Cell, LiftTrace, RootBound, SectorTrace, SolveResult, Statistics, Status},
    problem::{NormalizedProblem, normalize},
    projection::{Projection, project},
    roots::{FiberRoot, isolate_union, sector_sample},
};
use anyhow::{Result, ensure};
use std::{collections::HashMap, time::Instant};
use symbolica::prelude::*;

pub fn selected_order(problem: &Problem) -> Result<Vec<String>> {
    if !problem.solver.order.is_empty() || problem.solver.ordering == "given" {
        let order = if problem.solver.order.is_empty() && !problem.variable_groups.is_empty() {
            problem.variable_groups.iter().flatten().cloned().collect()
        } else {
            problem.effective_order()
        };
        return Ok(order);
    }
    let normalized = normalize(problem, &problem.variables)?;
    let groups = if problem.variable_groups.is_empty() {
        vec![(0..problem.variables.len()).collect()]
    } else {
        problem
            .variable_groups
            .iter()
            .map(|g| {
                g.iter()
                    .map(|v| problem.variables.iter().position(|n| n == v).unwrap())
                    .collect()
            })
            .collect()
    };
    let mut all = normalized.positives;
    all.extend(normalized.split);
    let order = choose_order(
        &normalized.algebra,
        &all,
        &groups,
        problem.solver.ordering.parse::<OrderingMethod>()?,
    )?
    .order;
    Ok(order
        .into_iter()
        .map(|i| problem.variables[i].clone())
        .collect())
}

pub fn bound(root: &FiberRoot) -> RootBound {
    RootBound {
        polynomial: root.polynomial,
        index: root.index,
        lower: root.interval.0.to_string(),
        upper: root.interval.1.to_string(),
    }
}

pub fn projection_table(
    algebra: &Algebra,
    projection: &Projection,
) -> (Vec<String>, Vec<Vec<usize>>) {
    let mut table = Vec::new();
    let mut ids = HashMap::new();
    let levels = projection
        .levels
        .iter()
        .map(|ps| {
            ps.iter()
                .map(|p| {
                    *ids.entry(p.clone()).or_insert_with(|| {
                        let i = table.len();
                        table.push(algebra.format(p));
                        i
                    })
                })
                .collect()
        })
        .collect();
    (table, levels)
}

struct Lifter<'a> {
    normalized: &'a NormalizedProblem,
    projection: &'a Projection,
    levels: &'a [Vec<usize>],
    problem: &'a Problem,
    started: Instant,
    incomplete: bool,
    cells: Vec<Cell>,
    statistics: Statistics,
}

impl Lifter<'_> {
    fn visit(
        &mut self,
        prefix: &mut Vec<Rational>,
        axes: &mut Vec<Axis>,
        positives: Vec<Poly>,
    ) -> Result<LiftTrace> {
        if self.incomplete {
            return Ok(LiftTrace::Unfinished {
                reason: "earlier branch exhausted a resource limit".into(),
            });
        }
        if self.started.elapsed().as_secs_f64() >= self.problem.limits.wall_time_secs {
            self.incomplete = true;
            return Ok(LiftTrace::Unfinished {
                reason: "wall time limit".into(),
            });
        }
        for (i, p) in positives.iter().enumerate() {
            if p.is_constant() && p.get_constant() <= Rational::zero() {
                self.statistics.rejected_branches += 1;
                return Ok(LiftTrace::Rejected { constraint: i });
            }
        }
        let level = prefix.len();
        let algebra = &self.normalized.algebra;
        if level == algebra.names.len() {
            if self
                .problem
                .limits
                .max_cells
                .is_some_and(|n| self.cells.len() >= n)
            {
                self.incomplete = true;
                return Ok(LiftTrace::Unfinished {
                    reason: "cell limit".into(),
                });
            }
            ensure!(
                positives.iter().all(|p| p.is_constant()),
                "not all coordinates substituted at leaf"
            );
            let signs = self
                .normalized
                .split
                .iter()
                .map(|p| sign(&evaluate(p, prefix)))
                .collect();
            let cell = self.cells.len();
            self.cells.push(Cell {
                axes: axes.clone(),
                signs,
            });
            return Ok(LiftTrace::Accepted { cell });
        }
        let polynomials = self.projection.levels[level]
            .iter()
            .zip(&self.levels[level])
            .map(|(p, id)| Ok((*id, algebra.specialize_univariate(p, level, prefix)?)))
            .collect::<Result<Vec<_>>>()?;
        let roots = isolate_union(polynomials)?;
        self.statistics.visited_stacks += 1;
        let mut sectors = Vec::with_capacity(roots.len() + 1);
        for i in 0..=roots.len() {
            let lo = if i > 0 { Some(&roots[i - 1]) } else { None };
            let hi = roots.get(i);
            let sample = sector_sample(lo, hi);
            let sample_text = sample.to_string();
            axes.push(Axis {
                variable: algebra.names[level].clone(),
                sample: sample_text.clone(),
                lower: lo.map(bound),
                upper: hi.map(bound),
            });
            prefix.push(sample.clone());
            let next = positives
                .iter()
                .map(|p| p.replace(level, &sample))
                .collect();
            let child = self.visit(prefix, axes, next)?;
            sectors.push(SectorTrace {
                sample: sample_text,
                child,
            });
            prefix.pop();
            axes.pop();
        }
        Ok(LiftTrace::Stack {
            roots: roots.iter().map(bound).collect(),
            sectors,
        })
    }
}

pub fn sign(value: &Rational) -> i8 {
    if value.is_zero() {
        0
    } else if value.is_negative() {
        -1
    } else {
        1
    }
}

pub fn evaluate(polynomial: &Poly, point: &[Rational]) -> Rational {
    if polynomial.is_constant() { polynomial.get_constant() } else { polynomial.replace_all(point) }
}

pub fn solve(problem: &Problem) -> Result<SolveResult> {
    problem.validate()?;
    ensure!(
        problem.solver.engine == "baseline",
        "optimized engine is not available yet"
    );
    let started = Instant::now();
    let order = selected_order(problem)?;
    let ordering_seconds = started.elapsed().as_secs_f64();
    let normalized = normalize(problem, &order)?;
    let projection_start = Instant::now();
    let mut all = normalized.positives.clone();
    all.extend(normalized.split.clone());
    let projection = project(&normalized.algebra, &all)?;
    let projection_seconds = projection_start.elapsed().as_secs_f64();
    let (polynomials, levels) = projection_table(&normalized.algebra, &projection);
    let mut lifter = Lifter {
        normalized: &normalized,
        projection: &projection,
        levels: &levels,
        problem,
        started,
        incomplete: false,
        cells: vec![],
        statistics: Statistics::default(),
    };
    let lifting_start = Instant::now();
    let trace = lifter.visit(&mut vec![], &mut vec![], normalized.positives.clone())?;
    lifter.statistics.lifting_seconds = lifting_start.elapsed().as_secs_f64();
    lifter.statistics.ordering_seconds = ordering_seconds;
    lifter.statistics.projection_seconds = projection_seconds;
    lifter.statistics.projection_steps = projection.steps.clone();
    lifter.statistics.raw_cells = lifter.cells.len();
    lifter.statistics.total_seconds = started.elapsed().as_secs_f64();
    let merging_started = Instant::now();
    let merged_cells = if problem.solver.merge == "aggressive" {
        Some(merge(&lifter.cells))
    } else {
        None
    };
    lifter.statistics.merging_seconds = merging_started.elapsed().as_secs_f64();
    lifter.statistics.total_seconds = started.elapsed().as_secs_f64();
    Ok(SolveResult {
        schema_version: 1,
        status: if lifter.incomplete {
            Status::Incomplete
        } else {
            Status::CompleteGeneric
        },
        problem: problem.clone(),
        order,
        normalized_constraints: normalized
            .positives
            .iter()
            .map(|p| normalized.algebra.format(p))
            .collect(),
        split_polynomials: normalized
            .split
            .iter()
            .map(|p| normalized.algebra.format(p))
            .collect(),
        exceptional_polynomials: (0..polynomials.len()).collect(),
        polynomials,
        projection_levels: levels.clone(),
        cells: lifter.cells,
        trace: Some(trace),
        statistics: lifter.statistics,
        diagnostic: None,
        merged_cells,
    })
}

fn same_boundary(a: &Option<RootBound>, b: &Option<RootBound>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.polynomial == b.polynomial && a.index == b.index,
        _ => false,
    }
}
/// Compatibility only: seams remain recoverable from raw cells and the trace.
pub fn merge(raw: &[Cell]) -> Vec<Cell> {
    let mut cells = raw.to_vec();
    loop {
        let mut found = None;
        'pairs: for i in 0..cells.len() {
            for j in i + 1..cells.len() {
                if cells[i].signs != cells[j].signs {
                    continue;
                }
                for k in 0..cells[i].axes.len() {
                    if (0..cells[i].axes.len()).any(|axis| {
                        axis != k
                            && (!same_boundary(
                                &cells[i].axes[axis].lower,
                                &cells[j].axes[axis].lower,
                            ) || !same_boundary(
                                &cells[i].axes[axis].upper,
                                &cells[j].axes[axis].upper,
                            ))
                    }) {
                        continue;
                    }
                    let a = &cells[i].axes[k];
                    let b = &cells[j].axes[k];
                    if a.upper.is_some() && same_boundary(&a.upper, &b.lower) {
                        found = Some((i, j, k, true));
                        break 'pairs;
                    }
                    if b.upper.is_some() && same_boundary(&b.upper, &a.lower) {
                        found = Some((i, j, k, false));
                        break 'pairs;
                    }
                }
            }
        }
        let Some((i, j, k, forward)) = found else {
            break;
        };
        if forward {
            cells[i].axes[k].upper = cells[j].axes[k].upper.clone();
        } else {
            cells[i].axes[k].lower = cells[j].axes[k].lower.clone();
        }
        cells.remove(j);
    }
    cells
}
