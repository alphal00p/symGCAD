//! Exact positive-endpoint linear fibers above a quadratic root opening.
//! Coefficient identities and global strict certificates prove the five rows;
//! no topology name, numerical sign inference, or kinematic specialization is used.
use crate::{
    Problem,
    algebra::{Poly, rational},
    domain::{DomainCertificate, certify_nonzero, derive_domain, verify_certificate},
    output::{Axis, Cell, RootBound, RootIndexDomain, SolveResult, Status},
    problem::{NormalizedProblem, normalize},
    roots::{FiberRoot, isolate_union, sector_sample},
    solver::{bound, evaluate, merge, sign},
    verify::Verification,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, time::Instant};
use symbolica::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuadraticLadderEvidence {
    pub schema_version: u32,
    /// B,a,c,b0,b1,d2, freshly extracted from the signed original split F.
    pub positivity: [DomainCertificate; 6],
    pub base: Box<SolveResult>,
}

pub fn selected_order(problem: &Problem) -> Result<Vec<String>> {
    problem.validate()?;
    ensure!(
        problem.variables.len() >= 4,
        "quadratic ladder requires a base axis and three fiber axes"
    );
    ensure!(
        !problem.solver.order.is_empty() || problem.solver.ordering == "given",
        "quadratic ladder requires fixed or given order"
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

fn coefficient(p: &Poly, axis: usize, degree: u32) -> Poly {
    let mut out = p.zero();
    for (c, m) in p.coefficients.iter().zip(p.exponents_iter()) {
        if m[axis] == degree {
            let mut e = m.to_vec();
            e[axis] = 0;
            out.append_monomial(c.clone(), &e);
        }
    }
    out
}
fn positive_axis(p: &Poly) -> Option<usize> {
    if p.nterms() != 1 || p.coefficients[0] <= Rational::zero() {
        return None;
    }
    let mut exponents = p
        .exponents_iter()
        .next()?
        .iter()
        .enumerate()
        .filter(|(_, e)| **e != 0);
    let (axis, degree) = exponents.next()?;
    (*degree == 1 && exponents.next().is_none()).then_some(axis)
}
fn polynomial_cap(p: &Poly) -> Result<()> {
    ensure!(
        p.nvars() <= 32 && p.nterms() <= 4096 && (0..p.nvars()).all(|i| p.degree(i) <= 32),
        "quadratic ladder polynomial envelope"
    );
    ensure!(
        p.coefficients
            .iter()
            .all(|c| c.numerator_ref().significant_bits() as u64
                + c.denominator_ref().significant_bits() as u64
                <= 4096),
        "quadratic ladder coefficient envelope"
    );
    Ok(())
}
struct Input {
    order: Vec<String>,
    normalized: NormalizedProblem,
    q: Poly,
    discriminant: Poly,
    positive: [Poly; 6],
    base_constraint_indices: Vec<usize>,
}
fn input(problem: &Problem) -> Result<Input> {
    let order = selected_order(problem)?;
    ensure!(
        problem.solver.engine == "quadratic_ladder",
        "wrong quadratic ladder engine"
    );
    ensure!(
        problem.split.len() == 1,
        "quadratic ladder requires exactly one split polynomial"
    );
    ensure!(
        problem.solver.homogeneous_anchor.is_none() && problem.solver.homogeneous_block.is_empty(),
        "quadratic ladder homogeneity transport is not implemented"
    );
    ensure!(
        problem.solver.positivity,
        "quadratic ladder requires positivity certificates"
    );
    let normalized = normalize(problem, &order)?;
    let n = order.len() - 3;
    let (y, x, z) = (n, n + 1, n + 2);
    let mut covered = BTreeSet::new();
    let mut base_constraint_indices = Vec::new();
    for (i, p) in normalized.positives.iter().enumerate() {
        if (n..order.len()).all(|v| p.degree(v) == 0) {
            base_constraint_indices.push(i);
            continue;
        }
        let axis = positive_axis(p).filter(|&v| v >= n).ok_or_else(|| {
            anyhow::anyhow!("quadratic ladder requires three free positive fiber axes")
        })?;
        covered.insert(axis);
    }
    ensure!(
        covered == BTreeSet::from([y, x, z]),
        "missing positive fiber constraint"
    );
    let f = &normalized.split[0];
    polynomial_cap(f)?;
    ensure!(
        f.degree(z) == 1,
        "quadratic ladder split must be linear in final axis"
    );
    let q = coefficient(f, z, 1);
    let endpoint = coefficient(f, z, 0);
    ensure!(
        q.degree(x) == 2,
        "quadratic ladder slope must be quadratic in preceding axis"
    );
    let a = coefficient(&q, x, 2);
    let b = coefficient(&q, x, 1);
    let c = coefficient(&q, x, 0);
    ensure!(
        b.degree(y) == 1,
        "quadratic ladder linear coefficient must cross affinely"
    );
    let b0 = coefficient(&b, y, 0);
    let b1 = -coefficient(&b, y, 1);
    let work = b
        .nterms()
        .saturating_mul(b.nterms())
        .saturating_add(a.nterms().saturating_mul(c.nterms()));
    ensure!(
        work <= 2_000_000,
        "quadratic ladder discriminant work envelope"
    );
    let discriminant = &b * &b - (&a * &c).mul_coeff(Rational::from(4));
    polynomial_cap(&discriminant)?;
    ensure!(
        discriminant.degree(y) == 2,
        "quadratic ladder discriminant must be quadratic in outer fiber"
    );
    let d2 = coefficient(&discriminant, y, 2);
    for p in [&b0, &b1, &d2] {
        ensure!(
            (n..order.len()).all(|v| p.degree(v) == 0),
            "quadratic ladder base coefficient has fiber support"
        );
    }
    let positive = [endpoint, a, c, b0, b1, d2];
    for p in &positive {
        polynomial_cap(p)?;
    }
    Ok(Input {
        order,
        normalized,
        q,
        discriminant,
        positive,
        base_constraint_indices,
    })
}
fn base_problem(problem: &Problem, input: &Input) -> Result<Problem> {
    let mut base = problem.clone();
    let n = input.order.len() - 3;
    base.variables = input.order[..n].to_vec();
    base.parameters.clear();
    base.split.clear();
    base.constraints = input
        .base_constraint_indices
        .iter()
        .map(|&i| {
            format!(
                "({})>0",
                input
                    .normalized
                    .algebra
                    .format(&input.normalized.positives[i])
            )
        })
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
    base.solver.engine = "optimized".into();
    base.solver.ordering = "given".into();
    base.solver.order = base.variables.clone();
    base.solver.merge = "none".into();
    base.solver.local_strict_bezout_hints.clear();
    base.solver.projection_strict_bezout_hints.clear();
    base.solver.negative_linear_guide = None;
    base.solver.bilinear_positive_guide = None;
    base.validate()?;
    Ok(base)
}
fn check_time(problem: &Problem, start: Option<Instant>) -> Result<()> {
    if start.is_some_and(|s| s.elapsed().as_secs_f64() >= problem.limits.wall_time_secs) {
        return Err(crate::resultant_interpolation::Stopped(
            "quadratic ladder wall budget exhausted".into(),
        )
        .into());
    }
    Ok(())
}
struct Layout {
    table: Vec<String>,
    levels: Vec<Vec<usize>>,
    exceptional: Vec<usize>,
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
fn zero_bound(id: usize) -> RootBound {
    RootBound {
        polynomial: id,
        index: 0,
        index_domain: RootIndexDomain::Real,
        lower: "0".into(),
        upper: "0".into(),
    }
}
fn axis(
    input: &Input,
    index: usize,
    sample: &Rational,
    lower: RootBound,
    upper: Option<RootBound>,
) -> Axis {
    Axis {
        variable: input.order[index].clone(),
        sample: sample.to_string(),
        lower: Some(lower),
        upper,
    }
}
fn isolated(
    input: &Input,
    p: &Poly,
    id: usize,
    coordinate: &Poly,
    coordinate_id: usize,
    level: usize,
    prefix: &[Rational],
) -> Result<(Vec<FiberRoot>, FiberRoot)> {
    let algebra = &input.normalized.algebra;
    let roots = isolate_union(vec![
        (id, algebra.specialize_univariate(p, level, prefix)?),
        (
            coordinate_id,
            algebra.specialize_univariate(coordinate, level, prefix)?,
        ),
    ])?;
    let zero = roots
        .iter()
        .find(|r| r.interval.0.is_zero() && r.interval.1.is_zero())
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("coordinate zero root missing"))?;
    Ok((roots, zero))
}
fn reconstruct(
    input: &Input,
    base: &SolveResult,
    mut emit: impl FnMut(Cell) -> Result<()>,
) -> Result<Layout> {
    let n = input.order.len() - 3;
    let (y, x, z) = (n, n + 1, n + 2);
    let algebra = &input.normalized.algebra;
    ensure!(
        base.order == input.order[..n] && base.projection_levels.len() == n,
        "quadratic ladder base order changed"
    );
    let mut layout = Layout {
        table: base
            .polynomials
            .iter()
            .map(|p| Ok(algebra.format(&algebra.parse(p)?)))
            .collect::<Result<_>>()?,
        levels: base.projection_levels.clone(),
        exceptional: base.exceptional_polynomials.clone(),
    };
    layout.levels.extend([vec![], vec![], vec![]]);
    let coordinates = [y, x, z]
        .map(|v| algebra.parse(&input.order[v]))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let ids = coordinates
        .iter()
        .map(|p| append(input, p, &mut layout))
        .collect::<Vec<_>>();
    let did = append(input, &input.discriminant, &mut layout);
    let qid = append(input, &input.q, &mut layout);
    let fid = append(input, &input.normalized.split[0], &mut layout);
    layout.exceptional.sort_unstable();
    layout.exceptional.dedup();
    for base_cell in &base.cells {
        ensure!(
            base_cell.axes.len() == n && base_cell.signs.is_empty(),
            "quadratic ladder base cell shape"
        );
        let prefix = base_cell
            .axes
            .iter()
            .map(|a| rational(&a.sample))
            .collect::<Result<Vec<_>>>()?;
        let (roots, zero) = isolated(
            input,
            &input.discriminant,
            did,
            &coordinates[0],
            ids[0],
            y,
            &prefix,
        )?;
        let rho = roots
            .iter()
            .find(|r| r.polynomial == did && r.index == 1)
            .ok_or_else(|| anyhow::anyhow!("larger discriminant root missing"))?;
        ensure!(
            rho.interval.0 >= Rational::zero(),
            "discriminant larger root is not positive"
        );
        let low_y = sector_sample(Some(&zero), Some(rho));
        let high_y = sector_sample(Some(rho), None);
        // Row 1: q>0 on the complete positive x ray before the root opening.
        let mut row = base_cell.clone();
        row.axes.extend([
            axis(input, y, &low_y, zero_bound(ids[0]), Some(bound(rho))),
            axis(input, x, &Rational::one(), zero_bound(ids[1]), None),
            axis(input, z, &Rational::one(), zero_bound(ids[2]), None),
        ]);
        row.signs = vec![1];
        emit_checked(input, row, &mut emit)?;
        let mut high_prefix = prefix;
        high_prefix.push(high_y.clone());
        let (xroots, xzero) = isolated(
            input,
            &input.q,
            qid,
            &coordinates[1],
            ids[1],
            x,
            &high_prefix,
        )?;
        let r0 = xroots
            .iter()
            .find(|r| r.polynomial == qid && r.index == 0)
            .ok_or_else(|| anyhow::anyhow!("first positive quadratic root missing"))?;
        let r1 = xroots
            .iter()
            .find(|r| r.polynomial == qid && r.index == 1)
            .ok_or_else(|| anyhow::anyhow!("second positive quadratic root missing"))?;
        ensure!(
            r0.interval.0 >= Rational::zero() && r0.interval.1 <= r1.interval.0,
            "positive quadratic root order failed"
        );
        let xs = [
            sector_sample(Some(&xzero), Some(r0)),
            sector_sample(Some(r0), Some(r1)),
            sector_sample(Some(r1), None),
        ];
        for branch in 0..3 {
            let mut row = base_cell.clone();
            row.axes.push(axis(input, y, &high_y, bound(rho), None));
            row.axes.push(axis(
                input,
                x,
                &xs[branch],
                if branch == 0 {
                    zero_bound(ids[1])
                } else {
                    bound(if branch == 1 { r0 } else { r1 })
                },
                if branch == 2 {
                    None
                } else {
                    Some(bound(if branch == 0 { r0 } else { r1 }))
                },
            ));
            if branch != 1 {
                row.axes
                    .push(axis(input, z, &Rational::one(), zero_bound(ids[2]), None));
                row.signs = vec![1];
                emit_checked(input, row, &mut emit)?;
                continue;
            }
            let mut final_prefix = high_prefix.clone();
            final_prefix.push(xs[1].clone());
            let (zroots, zzero) = isolated(
                input,
                &input.normalized.split[0],
                fid,
                &coordinates[2],
                ids[2],
                z,
                &final_prefix,
            )?;
            let cut = zroots
                .iter()
                .find(|r| r.polynomial == fid && r.index == 0)
                .ok_or_else(|| anyhow::anyhow!("positive linear F root missing"))?;
            ensure!(
                cut.interval.0 >= Rational::zero(),
                "linear F root not positive"
            );
            for above in [false, true] {
                let mut out = row.clone();
                let sample = if above {
                    sector_sample(Some(cut), None)
                } else {
                    sector_sample(Some(&zzero), Some(cut))
                };
                out.axes.push(axis(
                    input,
                    z,
                    &sample,
                    if above {
                        bound(cut)
                    } else {
                        zero_bound(ids[2])
                    },
                    if above { None } else { Some(bound(cut)) },
                ));
                out.signs = vec![if above { -1 } else { 1 }];
                emit_checked(input, out, &mut emit)?;
            }
        }
    }
    Ok(layout)
}
fn emit_checked(
    input: &Input,
    cell: Cell,
    emit: &mut impl FnMut(Cell) -> Result<()>,
) -> Result<()> {
    let point = cell
        .axes
        .iter()
        .map(|a| rational(&a.sample))
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        input
            .normalized
            .positives
            .iter()
            .all(|p| evaluate(p, &point) > Rational::zero()),
        "restored ladder sample violates original constraints"
    );
    ensure!(
        cell.signs == vec![sign(&evaluate(&input.normalized.split[0], &point))],
        "restored ladder sign mismatch"
    );
    emit(cell)
}
pub fn solve(problem: &Problem) -> Result<SolveResult> {
    let started = Instant::now();
    check_time(problem, Some(started))?;
    let input = input(problem)?;
    let domain = derive_domain(&input.normalized.algebra, &input.normalized.positives, &[])?;
    let mut certificates = Vec::new();
    for p in &input.positive {
        check_time(problem, Some(started))?;
        certificates.push(
            certify_nonzero(
                &input.normalized.algebra,
                p,
                &domain,
                &problem.solver.certificate_options(),
            )?
            .filter(|c| c.sign == 1)
            .ok_or_else(|| anyhow::anyhow!("quadratic ladder strict premise inconclusive"))?,
        );
    }
    let positivity = certificates
        .try_into()
        .map_err(|_| anyhow::anyhow!("quadratic ladder certificate count"))?;
    let mut reduced = base_problem(problem, &input)?;
    check_time(problem, Some(started))?;
    reduced.limits.wall_time_secs = problem.limits.wall_time_secs - started.elapsed().as_secs_f64();
    let planning = started.elapsed().as_secs_f64();
    let base = crate::solver::solve(&reduced)?;
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
    result.split_polynomials = input
        .normalized
        .split
        .iter()
        .map(|p| input.normalized.algebra.format(p))
        .collect();
    if base.status == Status::CompleteGeneric {
        let expected = base
            .cells
            .len()
            .checked_mul(5)
            .ok_or_else(|| anyhow::anyhow!("quadratic ladder cell count overflow"))?;
        if problem.limits.max_cells.is_some_and(|cap| expected > cap) {
            return Err(crate::resultant_interpolation::Stopped(
                "quadratic ladder cell cap".into(),
            )
            .into());
        }
        let restored = Instant::now();
        let mut cells = Vec::new();
        let layout = reconstruct(&input, &base, |c| {
            check_time(problem, Some(started))?;
            cells.push(c);
            Ok(())
        })?;
        ensure!(
            cells.len() == expected,
            "quadratic ladder exhaustive row count"
        );
        result.statistics.lifting_seconds += restored.elapsed().as_secs_f64();
        result.cells = cells;
        result.statistics.raw_cells = expected;
        result.polynomials = layout.table;
        result.projection_levels = layout.levels;
        result.exceptional_polynomials = layout.exceptional;
        result.status = Status::CompleteGeneric;
        result.diagnostic = None;
        let merging = Instant::now();
        result.merged_cells = (problem.solver.merge == "aggressive").then(|| merge(&result.cells));
        result.statistics.merging_seconds += merging.elapsed().as_secs_f64();
    }
    result.quadratic_ladder = Some(Box::new(QuadraticLadderEvidence {
        schema_version: 1,
        positivity,
        base: Box::new(base),
    }));
    check_time(problem, Some(started))?;
    result.statistics.total_seconds = started.elapsed().as_secs_f64();
    Ok(result)
}
pub fn verify(result: &SolveResult, evidence: &QuadraticLadderEvidence) -> Result<Verification> {
    ensure!(
        result.schema_version == 1
            && evidence.schema_version == 1
            && result.status == Status::CompleteGeneric,
        "incomplete or unsupported quadratic ladder evidence"
    );
    ensure!(
        result.trace.is_none()
            && result.homogeneous.is_none()
            && result.monotone.is_none()
            && result.local.is_none()
            && result.sign_filter.is_none()
            && result.linear_negative.is_none()
            && result.bilinear_positive.is_none()
            && result.bilinear_general.is_none(),
        "mixed quadratic ladder proof modes"
    );
    let input = input(&result.problem)?;
    ensure!(
        result.order == input.order,
        "quadratic ladder order changed"
    );
    for (p, cert) in input.positive.iter().zip(&evidence.positivity) {
        verify_certificate(
            &input.normalized.algebra,
            p,
            &input.normalized.positives,
            &[],
            cert,
        )?;
        ensure!(
            cert.sign == 1,
            "quadratic ladder requires strict positive premises"
        );
    }
    let mut expected = base_problem(&result.problem, &input)?;
    let delegated = evidence.base.problem.limits.wall_time_secs;
    ensure!(
        delegated.is_finite() && delegated > 0. && delegated <= expected.limits.wall_time_secs,
        "quadratic ladder base wall budget changed"
    );
    expected.limits.wall_time_secs = delegated;
    ensure!(
        serde_json::to_value(&expected)? == serde_json::to_value(&evidence.base.problem)?,
        "quadratic ladder base input changed"
    );
    ensure!(
        evidence.base.quadratic_ladder.is_none()
            && evidence.base.bilinear_positive.is_none()
            && evidence.base.bilinear_general.is_none()
            && evidence.base.linear_negative.is_none()
            && evidence.base.homogeneous.is_none()
            && evidence.base.monotone.is_none()
            && evidence.base.sign_filter.is_none()
            && evidence.base.local.is_none(),
        "unexpected quadratic ladder base proof"
    );
    let checked = crate::verify::verify(&evidence.base)?;
    let mut count = 0;
    let layout = reconstruct(&input, &evidence.base, |cell| {
        ensure!(
            result.cells.get(count) == Some(&cell),
            "quadratic ladder exhaustive cell {count} changed"
        );
        count += 1;
        Ok(())
    })?;
    ensure!(
        count == result.cells.len()
            && result.statistics.raw_cells == count
            && evidence.base.cells.len().checked_mul(5) == Some(count),
        "quadratic ladder cell inventory changed"
    );
    ensure!(
        result
            .problem
            .limits
            .max_cells
            .is_none_or(|cap| count <= cap),
        "complete ladder exceeds cell cap"
    );
    ensure!(
        result.normalized_constraints
            == input
                .normalized
                .positives
                .iter()
                .map(|p| input.normalized.algebra.format(p))
                .collect::<Vec<_>>()
            && result.split_polynomials
                == input
                    .normalized
                    .split
                    .iter()
                    .map(|p| input.normalized.algebra.format(p))
                    .collect::<Vec<_>>(),
        "quadratic ladder original metadata changed"
    );
    ensure!(
        result.polynomials == layout.table
            && result.projection_levels == layout.levels
            && result.exceptional_polynomials == layout.exceptional,
        "quadratic ladder table or exceptions changed"
    );
    ensure!(
        result.merged_cells
            == (result.problem.solver.merge == "aggressive").then(|| merge(&result.cells)),
        "quadratic ladder merge changed"
    );
    Ok(Verification {
        verified: true,
        cells: count,
        stacks: checked.stacks,
        rejected_branches: checked.rejected_branches,
    })
}
