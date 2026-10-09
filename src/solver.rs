use crate::{
    Problem,
    algebra::{Algebra, Poly},
    optimized_projection::{
        DomainPruningOptions, OptimizedProjectionOptions, OptimizedProjectionReport,
        ProjectionCache, project_ordered_certified, project_ordered_optimized,
    },
    ordering::{OrderingMethod, choose_order, choose_order_from_basis},
    output::{Axis, Cell, LiftTrace, RootBound, SectorTrace, SolveResult, Statistics, Status},
    problem::{NormalizedProblem, normalize},
    projection::{Projection, project},
    roots::{FiberRoot, isolate_union, sector_sample},
};
use anyhow::{Result, ensure};
use std::{collections::HashMap, time::Instant};
use symbolica::prelude::*;

pub fn selected_order(problem: &Problem) -> Result<Vec<String>> {
    if problem.solver.homogeneous_anchor.is_some() {
        return crate::homogeneity::selected_order(problem);
    }
    if problem.solver.engine == "monotone" {
        return crate::monotone::selected_order(problem);
    }
    if problem.solver.engine == "local" {
        return crate::local_projection::selected_order(problem);
    }
    if !problem.solver.order.is_empty() || problem.solver.ordering == "given" {
        let order = if problem.solver.order.is_empty() && !problem.variable_groups.is_empty() {
            problem.variable_groups.iter().flatten().cloned().collect()
        } else {
            problem.effective_order()
        };
        return Ok(order);
    }
    let normalized = normalize(problem, &problem.variables)?;
    if problem.solver.engine == "optimized" {
        let options = projection_options(problem)?;
        let mut cache = ProjectionCache::new(&normalized.algebra, &options);
        return Ok(select_cached(problem, &normalized, &mut cache)?
            .into_iter()
            .map(|i| problem.variables[i].clone())
            .collect());
    }
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

fn projection_options(problem: &Problem) -> Result<OptimizedProjectionOptions> {
    Ok(OptimizedProjectionOptions {
        checkpoint_dir: problem.solver.checkpoint_dir.clone(),
        memory_cache_bytes: problem.solver.cache_memory_mib * 1024 * 1024,
        progress_path: std::env::var_os("SYMGCAD_PROGRESS_PATH").map(std::path::PathBuf::from),
        capture_dir: std::env::var_os("SYMGCAD_CAPTURE_DIR").map(std::path::PathBuf::from),
        domain_fingerprint: serde_json::to_string(&(
            &problem.variables,
            &problem.constraints,
            &problem.parameters,
            &problem.split,
            problem.solver.positivity,
            problem.solver.input_positivity,
            &problem.solver.homogeneous_block,
            &problem.solver.homogeneous_anchor,
        ))?,
        ..Default::default()
    })
}

fn select_cached(
    problem: &Problem,
    normalized: &NormalizedProblem,
    cache: &mut ProjectionCache<'_>,
) -> Result<Vec<usize>> {
    if !problem.solver.order.is_empty() || problem.solver.ordering == "given" {
        let order = if problem.solver.order.is_empty() && !problem.variable_groups.is_empty() {
            problem.variable_groups.iter().flatten().cloned().collect()
        } else {
            problem.effective_order()
        };
        return Ok(order
            .iter()
            .map(|name| {
                problem
                    .variables
                    .iter()
                    .position(|n| n == name)
                    .expect("validated order")
            })
            .collect());
    }
    let groups = if problem.variable_groups.is_empty() {
        vec![(0..problem.variables.len()).collect()]
    } else {
        problem
            .variable_groups
            .iter()
            .map(|g| {
                g.iter()
                    .map(|v| {
                        problem
                            .variables
                            .iter()
                            .position(|n| n == v)
                            .expect("validated groups")
                    })
                    .collect()
            })
            .collect()
    };
    let mut all = normalized.positives.clone();
    all.extend(normalized.split.clone());
    let basis = if problem.solver.positivity {
        cache.initial_basis_certified(
            &normalized.positives,
            &normalized.split,
            &pruning_options(problem),
        )?
    } else {
        cache.initial_basis(&all)?
    };
    let protected = basis
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    let domain = crate::domain::derive_domain(&normalized.algebra, &normalized.positives, &[])?;
    let options = crate::domain::CertificateOptions::default();
    Ok(choose_order_from_basis(
        &normalized.algebra,
        basis,
        &groups,
        problem.solver.ordering.parse::<OrderingMethod>()?,
        |polynomials, variable| {
            let generated = cache.project_one(polynomials, variable)?;
            if !problem.solver.positivity {
                return Ok(generated);
            }
            let mut retained = Vec::new();
            for p in generated {
                if protected.contains(&p)
                    || crate::domain::certify_nonzero(&normalized.algebra, &p, &domain, &options)?
                        .is_none()
                {
                    retained.push(p);
                }
            }
            Ok(retained)
        },
    )?
    .order)
}

fn pruning_options(problem: &Problem) -> DomainPruningOptions {
    DomainPruningOptions {
        prune_input: problem.solver.input_positivity,
        ..Default::default()
    }
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
        positives: Vec<(usize, Poly)>,
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
        for (i, p) in &positives {
            if p.is_constant() && p.get_constant() <= Rational::zero() {
                self.statistics.rejected_branches += 1;
                return Ok(LiftTrace::Rejected { constraint: *i });
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
                positives.iter().all(|(_, p)| p.is_constant()),
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
            // Keep original IDs for exact rejection witnesses, but stop carrying
            // constraints already proved strictly positive on this branch.
            // Once a constraint fails, later substitutions cannot rescue it.
            let mut next = Vec::new();
            for (constraint, p) in &positives {
                let specialized = p.replace(level, &sample);
                if specialized.is_constant() {
                    if specialized.get_constant() <= Rational::zero() {
                        next.clear();
                        next.push((*constraint, specialized));
                        break;
                    }
                } else {
                    next.push((*constraint, specialized));
                }
            }
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
    if polynomial.is_constant() {
        polynomial.get_constant()
    } else {
        polynomial.replace_all(point)
    }
}

pub fn prepare_projection(
    problem: &Problem,
    order: &[String],
) -> Result<(
    NormalizedProblem,
    Projection,
    Option<OptimizedProjectionReport>,
)> {
    let normalized = normalize(problem, order)?;
    let (projection, report) = compute_projection(problem, order, &normalized)?;
    Ok((normalized, projection, report))
}

fn compute_projection(
    problem: &Problem,
    order: &[String],
    normalized: &NormalizedProblem,
) -> Result<(Projection, Option<OptimizedProjectionReport>)> {
    let mut all = normalized.positives.clone();
    all.extend(normalized.split.clone());
    if problem.solver.engine == "baseline" {
        return Ok((project(&normalized.algebra, &all)?, None));
    }
    let original = normalize(problem, &problem.variables)?;
    let mut original_polynomials = original.positives.clone();
    original_polynomials.extend(original.split.clone());
    let indices = order
        .iter()
        .map(|name| {
            problem
                .variables
                .iter()
                .position(|n| n == name)
                .expect("validated order")
        })
        .collect::<Vec<_>>();
    let options = projection_options(problem)?;
    let (projection, report) = if problem.solver.positivity {
        project_ordered_certified(
            &original.algebra,
            &original.positives,
            &original.split,
            &indices,
            &options,
            &pruning_options(problem),
        )?
    } else {
        project_ordered_optimized(&original.algebra, &original_polynomials, &indices, &options)?
    };
    Ok((projection, Some(report)))
}

fn prepare_optimized(
    problem: &Problem,
) -> Result<(
    Vec<String>,
    NormalizedProblem,
    Projection,
    OptimizedProjectionReport,
    f64,
    f64,
)> {
    let started = Instant::now();
    let original = normalize(problem, &problem.variables)?;
    let options = projection_options(problem)?;
    let mut cache = ProjectionCache::new(&original.algebra, &options);
    let indices = select_cached(problem, &original, &mut cache)?;
    let order = indices
        .iter()
        .map(|&i| problem.variables[i].clone())
        .collect::<Vec<_>>();
    let ordering_seconds = started.elapsed().as_secs_f64();
    let normalized = normalize(problem, &order)?;
    let projection_started = Instant::now();
    let projection = if problem.solver.positivity {
        cache.project_ordered_certified(
            &original.positives,
            &original.split,
            &indices,
            &pruning_options(problem),
        )?
    } else {
        let mut all = original.positives.clone();
        all.extend(original.split.clone());
        cache.project_ordered(&all, &indices)?
    };
    let projection_seconds = projection_started.elapsed().as_secs_f64();
    Ok((
        order,
        normalized,
        projection,
        cache.report,
        ordering_seconds,
        projection_seconds,
    ))
}

pub fn solve(problem: &Problem) -> Result<SolveResult> {
    problem.validate()?;
    if problem.solver.homogeneous_anchor.is_some() {
        return crate::homogeneity::solve_homogeneous(problem);
    }
    if problem.solver.engine == "monotone" {
        return crate::monotone::solve_monotone(problem);
    }
    if problem.solver.engine == "local" {
        let local = crate::local_projection::solve_local(problem, &Default::default())?;
        let mut result = local.solution;
        result.local = Some(Box::new(local.evidence));
        return Ok(result);
    }
    let started = Instant::now();
    let (order, normalized, projection, optimized_report, ordering_seconds, projection_seconds) =
        if problem.solver.engine == "optimized" {
            let (order, normalized, projection, report, ordering_seconds, projection_seconds) =
                prepare_optimized(problem)?;
            (
                order,
                normalized,
                projection,
                Some(report),
                ordering_seconds,
                projection_seconds,
            )
        } else {
            let order = selected_order(problem)?;
            let ordering_seconds = started.elapsed().as_secs_f64();
            let normalized = normalize(problem, &order)?;
            let projection_start = Instant::now();
            let (projection, report) = compute_projection(problem, &order, &normalized)?;
            let projection_seconds = projection_start.elapsed().as_secs_f64();
            (
                order,
                normalized,
                projection,
                report,
                ordering_seconds,
                projection_seconds,
            )
        };
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
    let trace = lifter.visit(
        &mut vec![],
        &mut vec![],
        normalized.positives.iter().cloned().enumerate().collect(),
    )?;
    lifter.statistics.lifting_seconds = lifting_start.elapsed().as_secs_f64();
    lifter.statistics.ordering_seconds = ordering_seconds;
    lifter.statistics.projection_seconds = projection_seconds;
    lifter.statistics.projection_steps = projection.steps.clone();
    lifter.statistics.optimized_projection = optimized_report;
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
        homogeneous: None,
        monotone: None,
        local: None,
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
