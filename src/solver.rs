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
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};
use symbolica::prelude::*;

mod merging;
pub use merging::merge;

pub fn selected_order(problem: &Problem) -> Result<Vec<String>> {
    if problem.solver.engine == "bilinear_positive" {
        return crate::bilinear_positive::selected_order(problem);
    }
    if problem.solver.engine == "linear_negative" {
        return crate::linear_negative::selected_order(problem);
    }
    if problem.solver.engine == "monotone" && problem.split.is_empty() {
        if problem.solver.monotone_negative_target {
            // The constrained original problem supplies the positive-scale
            // certificate; target transfer happens only after that reduction.
            if problem.solver.homogeneous_anchor.is_some() {
                return crate::homogeneity::selected_order(problem);
            }
            return crate::monotone::selected_negative_order(problem);
        }
        return crate::sign_filter::selected_order(problem);
    }
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
        binary_resultants: problem.solver.binary_resultants,
        progress_path: std::env::var_os("SYMGCAD_PROGRESS_PATH").map(std::path::PathBuf::from),
        capture_dir: std::env::var_os("SYMGCAD_CAPTURE_DIR").map(std::path::PathBuf::from),
        domain_fingerprint: serde_json::to_string(&(
            &problem.variables,
            &problem.constraints,
            &problem.parameters,
            &problem.split,
            problem.solver.positivity,
            problem.solver.input_positivity,
            problem.solver.prune_operations,
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
        prune_operations: problem.solver.prune_operations,
        ..Default::default()
    }
}

pub fn bound(root: &FiberRoot) -> RootBound {
    RootBound {
        polynomial: root.polynomial,
        index: root.index,
        index_domain: crate::output::RootIndexDomain::Real,
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
    progress: Option<LiftingProgress>,
}

/// Opt-in diagnostics are deliberately separate from projection progress and
/// from the mathematical result. They cannot be used as restart certificates.
struct LiftingProgress {
    path: PathBuf,
    started: Instant,
    last: Option<Instant>,
}

impl LiftingProgress {
    fn write(
        &mut self,
        phase: &str,
        variables: &[String],
        prefix: &[Rational],
        stacks: usize,
        cells: usize,
        fiber: Option<(usize, &UnivariatePolynomial<Q>, usize)>,
        force: bool,
    ) -> Result<()> {
        if !force
            && self
                .last
                .is_some_and(|last| last.elapsed() < Duration::from_secs(1))
        {
            return Ok(());
        }
        // Bound optional operand text. An omitted large fiber is explicitly
        // identified rather than leaving a previous operand looking current.
        let fiber = fiber.map(|(id, p, roots)| {
            let bits = p.coefficients().iter().fold(0_u64, |total, c| {
                total
                    .saturating_add(c.numerator_ref().significant_bits() as u64)
                    .saturating_add(c.denominator_ref().significant_bits() as u64)
            });
            serde_json::json!({"polynomial":id,"degree":p.degree(),"roots_accumulated":roots,
                "coefficient_bits_sum":bits,
                "coefficients_low_to_high":if bits<=131072 && p.degree()<=4096 {
                    Some(p.coefficients().iter().map(ToString::to_string).collect::<Vec<_>>())
                } else {None}})
        });
        let report = serde_json::json!({"kind":"lifting_diagnostic_only","phase":phase,
            "elapsed_seconds":self.started.elapsed().as_secs_f64(),"variables":variables,
            "prefix":prefix.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "level":prefix.len(),"visited_stacks":stacks,"accepted_cells":cells,"fiber":fiber});
        if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = self
            .path
            .with_extension(format!("{}.tmp", std::process::id()));
        std::fs::write(&temporary, serde_json::to_vec(&report)?)?;
        std::fs::rename(temporary, &self.path)?;
        self.last = Some(Instant::now());
        Ok(())
    }
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
        if let Some(progress) = &mut self.progress {
            progress.write(
                "specialize",
                &algebra.names,
                prefix,
                self.statistics.visited_stacks,
                self.cells.len(),
                None,
                false,
            )?;
        }
        let fibers =
            algebra.specialize_univariate_batch(&self.projection.levels[level], level, prefix)?;
        let polynomials = self.levels[level].iter().copied().zip(fibers).collect();
        let roots = if let Some(progress) = &mut self.progress {
            crate::roots::isolate_union_observed(polynomials, |phase, id, fiber, count| {
                progress.write(
                    phase,
                    &algebra.names,
                    prefix,
                    self.statistics.visited_stacks,
                    self.cells.len(),
                    Some((id, fiber, count)),
                    phase == "isolate_polynomial" && fiber.degree() >= 8,
                )
            })?
        } else {
            isolate_union(polynomials)?
        };
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
    classify_solve_result(problem, solve_impl(problem))
}

fn classify_solve_result(problem: &Problem, result: Result<SolveResult>) -> Result<SolveResult> {
    match result {
        Err(error)
            if error
                .downcast_ref::<crate::resultant_interpolation::Stopped>()
                .is_some() =>
        {
            Ok(SolveResult::failure(
                problem.clone(),
                Status::Incomplete,
                error.to_string(),
            ))
        }
        other => other,
    }
}

fn solve_impl(problem: &Problem) -> Result<SolveResult> {
    problem.validate()?;
    if problem.solver.engine == "bilinear_positive" {
        return crate::bilinear_positive::solve(problem);
    }
    if problem.solver.engine == "linear_negative" {
        return crate::linear_negative::solve(problem);
    }
    if problem.solver.engine == "monotone" && problem.split.is_empty() {
        if problem.solver.monotone_negative_target {
            if problem.solver.homogeneous_anchor.is_some() {
                return crate::homogeneity::solve_homogeneous(problem);
            }
            return crate::monotone::solve_negative_target(problem);
        }
        return crate::sign_filter::solve(problem);
    }
    if problem.solver.homogeneous_anchor.is_some() {
        return crate::homogeneity::solve_homogeneous(problem);
    }
    if problem.solver.engine == "monotone" {
        return crate::monotone::solve_monotone(problem);
    }
    if problem.solver.engine == "local" {
        let defaults = crate::local_projection::LocalOptions::default();
        let local = crate::local_projection::solve_local(
            problem,
            &crate::local_projection::LocalOptions {
                batch_sibling_discovery: problem.solver.local_batch_siblings,
                reuse_completed_subtrees: problem.solver.local_reuse_subtrees,
                reuse_learned_cuts: problem.solver.local_reuse_learned_cuts,
                stable_sector_samples: problem.solver.local_stable_samples,
                positive_pair_cancellations: problem.solver.local_pair_cancellations,
                descartes_one: problem.solver.local_descartes_one,
                descartes_witnesses: problem.solver.local_descartes_witnesses,
                descartes_intervals: problem.solver.local_descartes_intervals,
                descartes_interval_template_limit: problem
                    .solver
                    .local_descartes_interval_template_limit,
                positive_pair_witnesses: problem.solver.local_positive_pair_witnesses,
                strict_bezout: !problem.solver.local_strict_bezout_hints.is_empty(),
                base_point_projection: problem.solver.local_base_point_projection,
                adjacent_pairs: problem.solver.local_adjacent_pairs,
                subtree_cache_mib: problem.solver.local_subtree_cache_mib,
                learned_cut_cache_mib: problem.solver.local_learned_cut_cache_mib,
                max_refinements: problem
                    .solver
                    .local_max_refinements
                    .unwrap_or(defaults.max_refinements),
                max_nodes: problem.solver.local_max_nodes.unwrap_or(defaults.max_nodes),
                ..defaults
            },
        )?;
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
        progress: std::env::var_os("SYMGCAD_LIFTING_PROGRESS_PATH").map(|path| LiftingProgress {
            path: PathBuf::from(path),
            started,
            last: None,
        }),
    };
    let lifting_start = Instant::now();
    let trace = lifter.visit(
        &mut vec![],
        &mut vec![],
        normalized.positives.iter().cloned().enumerate().collect(),
    )?;
    if let Some(progress) = &mut lifter.progress {
        progress.write(
            "finished",
            &normalized.algebra.names,
            &[],
            lifter.statistics.visited_stacks,
            lifter.cells.len(),
            None,
            true,
        )?;
    }
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
        sign_filter: None,
        linear_negative: None,
        bilinear_positive: None,
    })
}

#[cfg(test)]
mod algebra_stop_tests {
    use super::*;

    #[test]
    fn only_typed_algebra_resource_stops_become_incomplete_results() {
        let problem = Problem::default();
        let stopped =
            anyhow::Error::new(crate::resultant_interpolation::Stopped("image cap".into()))
                .context("during global projection");
        let result = classify_solve_result(&problem, Err(stopped)).unwrap();
        assert_eq!(result.status, Status::Incomplete);
        assert!(result.cells.is_empty());
        assert!(crate::verify::verify(&result).is_err());
        let ordinary = anyhow::anyhow!("resultant interpolation stopped: image cap");
        assert!(classify_solve_result(&problem, Err(ordinary)).is_err());
    }
}
