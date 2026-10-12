//! Exhaustive positive fibers of F=a*x*y+b*x+c*y+d on a positive orthant.
//!
//! Strict guide comparisons prove the seven rows of the two-fiber table.
//! All exported roots are ordinary, simple, full-real linear roots. No
//! finite/infinite transition or empty fiber is hidden inside a parent cell.
use crate::{
    Problem,
    algebra::{Poly, rational},
    domain::{
        CertificateOptions, DomainCertificate, certify_nonzero, derive_domain, verify_certificate,
    },
    output::{Axis, Cell, RootBound, RootIndexDomain, SolveResult, Status},
    problem::{NormalizedProblem, normalize},
    solver::{evaluate, merge, sign},
    verify::Verification,
};
use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, time::Instant};
use symbolica::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BilinearPositiveEvidence {
    pub target_constraint: usize,
    pub guide: String,
    /// Strict positive certificates for d,C0,D0,W0,C4,D4,W4, in that order.
    pub positivity: [DomainCertificate; 7],
    pub base: Box<SolveResult>,
}

fn positive_axis(p: &Poly) -> Option<usize> {
    if p.nterms() != 1 || p.coefficients[0] <= Rational::zero() {
        return None;
    }
    let e = p.exponents_iter().next()?;
    let mut active = e.iter().enumerate().filter(|(_, v)| **v != 0);
    let (i, degree) = active.next()?;
    (*degree == 1 && active.next().is_none()).then_some(i)
}

pub fn selected_order(problem: &Problem) -> Result<Vec<String>> {
    problem.validate()?;
    ensure!(
        problem.variables.len() >= 2,
        "bilinear positive reduction requires two fiber axes"
    );
    ensure!(
        !problem.solver.order.is_empty() || problem.solver.ordering == "given",
        "bilinear positive reduction requires an explicit or given order"
    );
    let order = if problem.solver.order.is_empty() && !problem.variable_groups.is_empty() {
        problem.variable_groups.iter().flatten().cloned().collect()
    } else {
        problem.effective_order()
    };
    let mut checked = problem.clone();
    checked.solver.order = order.clone();
    checked.validate()?;
    Ok(order)
}

struct Input {
    order: Vec<String>,
    normalized: NormalizedProblem,
    target: usize,
    guide: Poly,
    a: Poly,
    b: Poly,
    c: Poly,
    d: Poly,
    a0: Poly,
    b0: Poly,
    positive: [Poly; 7],
}

fn input(problem: &Problem) -> Result<Input> {
    let order = selected_order(problem)?;
    ensure!(
        problem.solver.engine == "bilinear_positive",
        "wrong bilinear positive engine"
    );
    ensure!(
        problem.split.is_empty(),
        "bilinear positive reduction requires an unsplit strict target"
    );
    ensure!(
        problem.solver.homogeneous_anchor.is_none(),
        "bilinear positive guide transport through homogeneity is not implemented"
    );
    let normalized = normalize(problem, &order)?;
    let targets: Vec<_> = normalized
        .positives
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.is_constant() && positive_axis(p).is_none())
        .map(|(i, _)| i)
        .collect();
    ensure!(
        targets.len() == 1,
        "bilinear positive reduction requires one non-coordinate target on a positive orthant"
    );
    let target = targets[0];
    let mut covered = BTreeSet::new();
    for (i, p) in normalized.positives.iter().enumerate() {
        if i == target {
            continue;
        }
        if let Some(axis) = positive_axis(p) {
            covered.insert(axis);
        } else {
            ensure!(
                p.is_constant() && p.get_constant() > Rational::zero(),
                "unsupported nonpositive constant or extra domain constraint"
            );
        }
    }
    ensure!(
        covered.len() == order.len(),
        "every original coordinate must be strictly positive"
    );
    let x = order.len() - 1;
    let y = x - 1;
    let f = &normalized.positives[target];
    ensure!(
        f.degree(x) == 1 && f.degree(y) == 1,
        "target must be affine in each final fiber axis"
    );
    let a0 = f.derivative(x);
    let b0 = f.replace(x, &Rational::zero());
    let a = a0.derivative(y);
    let b = a0.replace(y, &Rational::zero());
    let c = b0.derivative(y);
    let d = b0.replace(y, &Rational::zero());
    ensure!(
        [&a, &b, &c].iter().all(|p| !p.is_zero()),
        "identically zero a, b, or c is outside this bilinear rule"
    );
    let guide_text = problem
        .solver
        .bilinear_positive_guide
        .as_ref()
        .ok_or_else(|| anyhow!("bilinear positive guide missing"))?;
    let mut proposed = problem.clone();
    proposed.split = vec![guide_text.clone()];
    let guide = normalize(&proposed, &order)?.split.remove(0);
    ensure!(
        guide.degree(x) == 1,
        "guide must be affine in the final fiber axis"
    );
    let c0 = guide.derivative(x);
    let d0 = guide.replace(x, &Rational::zero());
    ensure!(
        c0.degree(y) == 1,
        "guide leading coefficient must be affine in the preceding fiber axis"
    );
    let c4 = c0.derivative(y);
    let d4 = c0.replace(y, &Rational::zero());
    ensure!(
        [f, &guide]
            .iter()
            .all(|p| p.nterms() <= 4096 && (0..p.nvars()).all(|v| p.degree(v) <= 64)),
        "bilinear guide polynomial cap exceeded"
    );
    ensure!(
        b0.nterms()
            .saturating_mul(c0.nterms())
            .saturating_add(a0.nterms().saturating_mul(d0.nterms()))
            <= 2_000_000,
        "bilinear guide product cap exceeded"
    );
    ensure!(
        b.nterms()
            .saturating_mul(c4.nterms())
            .saturating_add(a.nterms().saturating_mul(d4.nterms()))
            <= 2_000_000,
        "bilinear derivative guide product cap exceeded"
    );
    let w0 = &b0 * &c0 - &a0 * &d0;
    let w4 = &b * &c4 - &a * &d4;
    let positive = [d.clone(), c0, d0, w0, c4, d4, w4];
    Ok(Input {
        order,
        normalized,
        target,
        guide,
        a,
        b,
        c,
        d,
        a0,
        b0,
        positive,
    })
}

fn base_problem(problem: &Problem, input: &Input) -> Result<Problem> {
    let n = input.order.len() - 2;
    let algebra = &input.normalized.algebra;
    let mut base = problem.clone();
    base.variables = input.order[..n].to_vec();
    base.parameters.clear();
    base.constraints = input
        .normalized
        .positives
        .iter()
        .enumerate()
        .filter(|(i, p)| *i != input.target && positive_axis(p).is_none_or(|axis| axis < n))
        .map(|(_, p)| format!("({})>0", algebra.format(p)))
        .collect();
    base.split = [&input.a, &input.b, &input.c]
        .iter()
        .map(|p| algebra.format(p))
        .collect();
    base.variable_groups = problem
        .variable_groups
        .iter()
        .map(|g| {
            g.iter()
                .filter(|v| base.variables.contains(v))
                .cloned()
                .collect::<Vec<_>>()
        })
        .filter(|g| !g.is_empty())
        .collect();
    base.solver.engine = problem.solver.bilinear_positive_base_engine.clone();
    if base.solver.engine == "baseline" {
        base.solver.positivity = false;
        base.solver.input_positivity = false;
        base.solver.prune_operations = false;
    }
    base.solver.ordering = "given".into();
    base.solver.order = base.variables.clone();
    base.solver.bilinear_positive_guide = None;
    base.solver.negative_linear_guide = None;
    base.solver.homogeneous_anchor = None;
    base.solver.homogeneous_block.clear();
    base.solver.local_strict_bezout_hints.clear();
    base.solver.projection_strict_bezout_hints.clear();
    base.solver.merge = "none".into();
    base.validate()?;
    Ok(base)
}

fn check_time(problem: &Problem, started: Option<Instant>) -> Result<()> {
    if started.is_some_and(|s| s.elapsed().as_secs_f64() >= problem.limits.wall_time_secs) {
        return Err(crate::resultant_interpolation::Stopped(
            "bilinear positive wall budget exhausted".into(),
        )
        .into());
    }
    Ok(())
}

struct Layout {
    table: Vec<String>,
    levels: Vec<Vec<usize>>,
    exceptional: Vec<usize>,
    stopped: bool,
}
fn append(input: &Input, p: &Poly, layout: &mut Layout) -> usize {
    let id = layout.table.len();
    layout.table.push(input.normalized.algebra.format(p));
    if let Some(level) = (0..input.order.len()).rev().find(|&i| p.degree(i) > 0) {
        layout.levels[level].push(id);
        layout.exceptional.push(id);
    }
    id
}
fn bound(id: usize, value: &Rational) -> RootBound {
    RootBound {
        polynomial: id,
        index: 0,
        index_domain: RootIndexDomain::Real,
        lower: value.to_string(),
        upper: value.to_string(),
    }
}

/// Emit each expected cell exactly once, also used as a streaming verifier.
fn reconstruct(
    input: &Input,
    base: &SolveResult,
    mut emit: impl FnMut(Cell) -> Result<bool>,
) -> Result<Layout> {
    let n = input.order.len() - 2;
    ensure!(
        base.order == input.order[..n],
        "bilinear base order changed"
    );
    ensure!(
        base.projection_levels.len() == n,
        "bilinear base level count changed"
    );
    let algebra = &input.normalized.algebra;
    let mut layout = Layout {
        table: base
            .polynomials
            .iter()
            .map(|p| Ok(algebra.format(&algebra.parse(p)?)))
            .collect::<Result<_>>()?,
        levels: base.projection_levels.clone(),
        exceptional: base.exceptional_polynomials.clone(),
        stopped: false,
    };
    layout.levels.extend([vec![], vec![]]);
    for p in [&input.a, &input.b, &input.c] {
        if !p.is_constant() {
            append(input, p, &mut layout);
        }
    }
    let yid = append(input, &algebra.parse(&input.order[n])?, &mut layout);
    let xid = append(input, &algebra.parse(&input.order[n + 1])?, &mut layout);
    let aid = append(input, &input.a0, &mut layout);
    let bid = append(input, &input.b0, &mut layout);
    let fid = append(
        input,
        &input.normalized.positives[input.target],
        &mut layout,
    );
    layout.exceptional.sort_unstable();
    layout.exceptional.dedup();
    for cell in &base.cells {
        ensure!(
            cell.axes.len() == n && cell.signs.len() == 3,
            "invalid bilinear base cell shape"
        );
        let mut point = cell
            .axes
            .iter()
            .map(|axis| rational(&axis.sample))
            .collect::<Result<Vec<_>>>()?;
        point.resize(n + 2, Rational::zero());
        let [a, b, c, d] = [&input.a, &input.b, &input.c, &input.d].map(|p| evaluate(p, &point));
        let signs = [sign(&a), sign(&b), sign(&c)];
        ensure!(
            cell.signs == signs && signs.iter().all(|s| *s != 0) && d > Rational::zero(),
            "bilinear base signs or strict endpoint changed"
        );
        // (lower y value/id, optional upper y value/id, finite upper x?).
        let zero = Rational::zero();
        let z = (zero.clone(), yid);
        let mut rows: Vec<((Rational, usize), Option<(Rational, usize)>, bool)> = Vec::new();
        match signs {
            [1, 1, 1] => rows.push((z, None, false)),
            [-1, -1, 1] => rows.push((z, None, true)),
            [-1, -1, -1] => rows.push((z, Some((-d.clone() / &c, bid)), true)),
            [-1, 1, 1] => {
                let r = -b.clone() / &a;
                rows.push((z, Some((r.clone(), aid)), false));
                rows.push(((r, aid), None, true));
            }
            [-1, 1, -1] => {
                let ra = -b.clone() / &a;
                let rb = -d.clone() / &c;
                ensure!(ra < rb, "strict guide root ordering failed");
                rows.push((z, Some((ra.clone(), aid)), false));
                rows.push(((ra, aid), Some((rb, bid)), true));
            }
            _ => anyhow::bail!("impossible sign pattern under strict guide certificates"),
        }
        for (lower, upper, finite) in rows {
            ensure!(
                lower.0 >= zero && upper.as_ref().is_none_or(|u| u.0 > lower.0),
                "empty bilinear fiber"
            );
            let ys = upper.as_ref().map_or_else(
                || &lower.0 + &Rational::one(),
                |u| (&lower.0 + &u.0) / Rational::from(2),
            );
            point[n] = ys.clone();
            let slope = &a * &ys + &b;
            let endpoint = &c * &ys + &d;
            let xu = if finite {
                ensure!(
                    slope < Rational::zero() && endpoint > Rational::zero(),
                    "finite positive fiber hypotheses failed"
                );
                Some(-endpoint / slope)
            } else {
                ensure!(
                    slope > Rational::zero() && endpoint > Rational::zero(),
                    "unbounded positive fiber hypotheses failed"
                );
                None
            };
            let xs = xu
                .as_ref()
                .map_or_else(Rational::one, |u| u / &Rational::from(2));
            point[n + 1] = xs.clone();
            ensure!(
                evaluate(&input.normalized.positives[input.target], &point) > Rational::zero(),
                "restored sample is not in the original target"
            );
            let mut out = cell.clone();
            out.signs.clear();
            out.axes.push(Axis {
                variable: input.order[n].clone(),
                sample: ys.to_string(),
                lower: Some(bound(lower.1, &lower.0)),
                upper: upper.as_ref().map(|u| bound(u.1, &u.0)),
            });
            out.axes.push(Axis {
                variable: input.order[n + 1].clone(),
                sample: xs.to_string(),
                lower: Some(bound(xid, &zero)),
                upper: xu.as_ref().map(|u| bound(fid, u)),
            });
            if !emit(out)? {
                layout.stopped = true;
                return Ok(layout);
            }
        }
    }
    Ok(layout)
}

pub fn solve(problem: &Problem) -> Result<SolveResult> {
    let started = Instant::now();
    let planned = (|| -> Result<_> {
        check_time(problem, Some(started))?;
        let input = input(problem)?;
        let domain = derive_domain(
            &input.normalized.algebra,
            &input.normalized.positives,
            &[input.target],
        )?;
        let mut certificates = Vec::new();
        for p in &input.positive {
            check_time(problem, Some(started))?;
            let cert = certify_nonzero(
                &input.normalized.algebra,
                p,
                &domain,
                &CertificateOptions::default(),
            )?
            .filter(|c| c.sign == 1)
            .ok_or_else(|| {
                anyhow!(
                    "bilinear positive strict guide certificate inconclusive for {}",
                    input.normalized.algebra.format(p)
                )
            })?;
            certificates.push(cert);
        }
        let positivity: [DomainCertificate; 7] = certificates
            .try_into()
            .map_err(|_| anyhow!("internal certificate count"))?;
        let mut base = base_problem(problem, &input)?;
        check_time(problem, Some(started))?;
        base.limits.wall_time_secs =
            problem.limits.wall_time_secs - started.elapsed().as_secs_f64();
        Ok((input, positivity, base))
    })();
    let (input, positivity, base_problem) = match planned {
        Ok(v) => v,
        Err(e)
            if e.downcast_ref::<crate::resultant_interpolation::Stopped>()
                .is_some() =>
        {
            return Ok(SolveResult::failure(
                problem.clone(),
                Status::Incomplete,
                e.to_string(),
            ));
        }
        Err(e) => return Err(e),
    };
    let planning = started.elapsed().as_secs_f64();
    let base = crate::solver::solve(&base_problem)?;
    let mut result = SolveResult::failure(
        problem.clone(),
        base.status.clone(),
        base.diagnostic.clone().unwrap_or_default(),
    );
    result.order = input.order.clone();
    result.statistics = base.statistics.clone();
    result.statistics.ordering_seconds += planning;
    result.statistics.raw_cells = 0;
    result.normalized_constraints = input
        .normalized
        .positives
        .iter()
        .map(|p| input.normalized.algebra.format(p))
        .collect();
    if base.status == Status::CompleteGeneric {
        let restored = Instant::now();
        let mut cells = Vec::new();
        let mut reason = None;
        let layout = reconstruct(&input, &base, |cell| {
            if check_time(problem, Some(started)).is_err() {
                reason = Some("bilinear positive restoration reached wall budget".to_string());
                return Ok(false);
            }
            if problem
                .limits
                .max_cells
                .is_some_and(|cap| cells.len() >= cap)
            {
                reason = Some("bilinear positive restoration reached cell limit".to_string());
                return Ok(false);
            }
            cells.push(cell);
            Ok(true)
        })?;
        result.statistics.lifting_seconds += restored.elapsed().as_secs_f64();
        result.polynomials = layout.table;
        result.projection_levels = layout.levels;
        result.exceptional_polynomials = layout.exceptional;
        result.cells = cells;
        result.statistics.raw_cells = result.cells.len();
        result.status = if layout.stopped {
            Status::Incomplete
        } else {
            Status::CompleteGeneric
        };
        result.diagnostic = reason;
        let merge_start = Instant::now();
        result.merged_cells = (problem.solver.merge == "aggressive").then(|| merge(&result.cells));
        result.statistics.merging_seconds += merge_start.elapsed().as_secs_f64();
    }
    result.bilinear_positive = Some(Box::new(BilinearPositiveEvidence {
        target_constraint: input.target,
        guide: input.normalized.algebra.format(&input.guide),
        positivity,
        base: Box::new(base),
    }));
    if let Err(e) = check_time(problem, Some(started)) {
        result.status = Status::Incomplete;
        result.diagnostic = Some(e.to_string());
    }
    result.statistics.total_seconds = started.elapsed().as_secs_f64();
    Ok(result)
}

pub fn verify(result: &SolveResult, evidence: &BilinearPositiveEvidence) -> Result<Verification> {
    ensure!(
        result.schema_version == 1 && result.status == Status::CompleteGeneric,
        "bilinear positive result is incomplete"
    );
    ensure!(
        result.trace.is_none()
            && result.homogeneous.is_none()
            && result.monotone.is_none()
            && result.local.is_none()
            && result.sign_filter.is_none()
            && result.linear_negative.is_none()
            && result.bilinear_general.is_none(),
        "mixed bilinear positive proof modes"
    );
    let input = input(&result.problem)?;
    ensure!(
        result.order == input.order && evidence.target_constraint == input.target,
        "bilinear positive order or target changed"
    );
    ensure!(
        input.normalized.algebra.parse(&evidence.guide)? == input.guide,
        "bilinear positive guide changed"
    );
    for (p, cert) in input.positive.iter().zip(&evidence.positivity) {
        verify_certificate(
            &input.normalized.algebra,
            p,
            &input.normalized.positives,
            &[input.target],
            cert,
        )?;
        ensure!(
            cert.sign == 1,
            "bilinear guide requires strictly positive certificates"
        );
    }
    let mut expected = base_problem(&result.problem, &input)?;
    let delegated = evidence.base.problem.limits.wall_time_secs;
    ensure!(
        delegated.is_finite() && delegated > 0. && delegated <= expected.limits.wall_time_secs,
        "bilinear base wall budget changed"
    );
    expected.limits.wall_time_secs = delegated;
    ensure!(
        serde_json::to_value(&expected)? == serde_json::to_value(&evidence.base.problem)?,
        "bilinear base mathematical problem or options changed"
    );
    ensure!(
        evidence.base.bilinear_positive.is_none()
            && evidence.base.bilinear_general.is_none()
            && evidence.base.homogeneous.is_none()
            && evidence.base.monotone.is_none()
            && evidence.base.sign_filter.is_none()
            && evidence.base.linear_negative.is_none(),
        "unexpected nested bilinear base proof"
    );
    let checked = crate::verify::verify(&evidence.base)?;
    let mut index = 0;
    let layout = reconstruct(&input, &evidence.base, |cell| {
        ensure!(
            result.cells.get(index) == Some(&cell),
            "bilinear exhaustive fiber restoration changed at cell {index}"
        );
        index += 1;
        Ok(true)
    })?;
    ensure!(
        index == result.cells.len() && result.statistics.raw_cells == index,
        "bilinear result cell inventory changed"
    );
    ensure!(
        result
            .problem
            .limits
            .max_cells
            .is_none_or(|cap| index <= cap),
        "complete bilinear result exceeds cell cap"
    );
    ensure!(
        result.normalized_constraints
            == input
                .normalized
                .positives
                .iter()
                .map(|p| input.normalized.algebra.format(p))
                .collect::<Vec<_>>()
            && result.split_polynomials.is_empty(),
        "bilinear original metadata changed"
    );
    ensure!(
        result.polynomials == layout.table
            && result.projection_levels == layout.levels
            && result.exceptional_polynomials == layout.exceptional,
        "bilinear polynomial table or exceptional set changed"
    );
    ensure!(
        result.merged_cells
            == (result.problem.solver.merge == "aggressive").then(|| merge(&result.cells)),
        "bilinear compatibility merge changed"
    );
    Ok(Verification {
        verified: true,
        cells: index,
        stacks: checked.stacks,
        rejected_branches: checked.rejected_branches,
    })
}
