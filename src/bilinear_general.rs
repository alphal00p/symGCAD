//! Exact positive fibers of F=a*x*y+b*x+c*y+d over a union of base pieces.
//!
//! The last two lifted axes are y (second to last) and x (last). On a base
//! region where the strict signs of a, b, c, d are fixed and, when both
//! breakpoints -b/a and -d/c are positive, the sign of Delta=b*c-a*d is fixed,
//! the positive (y,x)-fiber of F>0 is an explicit union of at most three
//! cylindrical rows with simple linear boundaries in the original coordinates.
//!
//! The base is split into ordinary symGCAD problems ("pieces") whose strict
//! atom conditions and split atoms fix exactly these signs. Pieces never
//! depend on each other's cells, so they can be solved by separate processes.
//!
//! Every pruning step is backed by an exact identity that the verifier replays:
//! * `RatioOrder{lower,upper}`: upper*u_lower-lower*u_upper = m*s^2, m>0, for
//!   the corresponding coefficients u_* of a supplied guide polynomial U, each
//!   certified strictly positive on the whole coordinate box. Pointwise this
//!   gives lower/u_lower <= upper/u_upper, hence lower>0 => upper>0 and
//!   upper<0 => lower<0.
//! * `GlobalSign`: a whole-box strict sign certificate for one atom.
//! * The separator G: Delta = m*G^2 with m>0 (so Delta>0 wherever G!=0),
//!   Delta = -m*G^2 with m>0 (Delta<0 wherever G!=0), or G = Delta itself.
//!
//! An identically zero b, c or d has no zero set inside the open quadrant to
//! respect, and its term never changes sign there: for y>0, a*y+0 has the sign
//! of a, c*y+0 that of c, and 0*y+d that of d. The fiber table therefore uses
//! the strict sign of that partner atom for the zero coefficient. Zero atoms
//! carry no premise, condition or split. An identically zero a or Delta is
//! declined.
//!
//! The verifier never trusts the planner. It checks, by a truth table over all
//! strict sign vectors of the five atoms that are consistent with the replayed
//! premises, that exactly one piece or empty claim matches each vector, that
//! every piece leaf determines the fiber table, and that every empty claim is
//! contradictory or has an all-negative coefficient pattern (empty fiber).
//! Atom zero sets are retained as exceptional seams.
//!
//! The planner objective only selects among valid plans: the verifier accepts
//! every plan that passes the checks above, whichever objective produced it.
use crate::{
    Problem,
    algebra::{Poly, rational},
    domain::{
        CertificateOptions, DomainCertificate, certify_nonzero, derive_domain, verify_certificate,
    },
    identity::{check_positive_square_identity, positive_square_witness},
    output::{Axis, Cell, RootBound, RootIndexDomain, SolveResult, Statistics, Status},
    problem::{NormalizedProblem, normalize},
    solver::{evaluate, merge, sign},
    verify::Verification,
};
use anyhow::{Result, anyhow, bail, ensure};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use symbolica::prelude::*;

/// The five base atoms in canonical order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Atom {
    /// Coefficient of x*y.
    A,
    /// Coefficient of x.
    B,
    /// Coefficient of y.
    C,
    /// Constant coefficient.
    D,
    /// Root-order separator (see [`Separator`]).
    G,
}

pub const ATOMS: [Atom; 5] = [Atom::A, Atom::B, Atom::C, Atom::D, Atom::G];

impl Atom {
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub atom: Atom,
    /// Strict sign, +1 or -1.
    pub sign: i8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CornerCertificate {
    /// The guide coefficient u_corner (corner of the (x,y) quadrant).
    pub corner: Atom,
    pub certificate: DomainCertificate,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SignPremise {
    /// upper*u_lower - lower*u_upper == multiplier*square^2, multiplier > 0.
    RatioOrder {
        lower: Atom,
        upper: Atom,
        multiplier: String,
        square: String,
        /// Optional strict nonvanishing certificate of the square witness:
        /// then lower/u_lower < upper/u_upper everywhere, so lower and upper
        /// never vanish together. Recorded and replayed; strict sign vectors
        /// do not need it.
        strict: Option<DomainCertificate>,
    },
    /// Strict sign of one atom on the whole certified coordinate box.
    GlobalSign {
        atom: Atom,
        certificate: DomainCertificate,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Separator {
    /// b*c - a*d == multiplier*square^2 with multiplier > 0, G = square.
    Square { multiplier: String, square: String },
    /// b*c - a*d == -multiplier*square^2 with multiplier > 0, G = square.
    NegativeSquare { multiplier: String, square: String },
    /// G = b*c - a*d itself.
    Delta { polynomial: String },
}

impl Separator {
    /// The sign of Delta wherever G != 0, or 0 when G is Delta itself.
    fn delta_sign(&self) -> i8 {
        match self {
            Separator::Square { .. } => 1,
            Separator::NegativeSquare { .. } => -1,
            Separator::Delta { .. } => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PieceSpec {
    /// Strict path conditions in strictly increasing atom order. They become
    /// base constraints after the retained original base constraints.
    pub conditions: Vec<Condition>,
    /// Split atoms in strictly increasing atom order; every split pattern is
    /// a leaf of this piece.
    pub split: Vec<Atom>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmptyClaim {
    pub conditions: Vec<Condition>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BilinearGeneralPlan {
    pub schema_version: u32,
    pub target_constraint: usize,
    pub order: Vec<String>,
    /// a, b, c, d recorded for readability and replayed exactly.
    pub coefficients: Vec<String>,
    pub guide: Option<String>,
    pub guide_positivity: Vec<CornerCertificate>,
    pub premises: Vec<SignPremise>,
    pub separator: Separator,
    pub pieces: Vec<PieceSpec>,
    pub empty: Vec<EmptyClaim>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BilinearGeneralEvidence {
    pub plan: BilinearGeneralPlan,
    /// One complete base result per piece, in plan order.
    pub bases: Vec<SolveResult>,
}

/// How the pieces of a result were executed (see [`PieceTimings`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Execution {
    /// Planning, every piece solve and assembly ran in this process, one
    /// after another: `Statistics::total_seconds` is their elapsed wall time.
    #[default]
    Sequential,
    /// The pieces were solved by other processes, possibly concurrently.
    /// `Statistics::total_seconds` and the phase timings then cover only the
    /// assembling process; no end-to-end elapsed time is known.
    SeparateProcesses,
}

/// Wall-time report of a bilinear_general result. Like every timing it is a
/// report, not evidence, and it is not verified. Planning, piece solves and
/// assembly stay distinct (HANDOFF failure mode 10).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PieceTimings {
    pub execution: Execution,
    /// Planning wall time as recorded by the planning process.
    pub planning_seconds: f64,
    /// Each returned piece result's own `total_seconds`, in plan order.
    pub piece_seconds: Vec<f64>,
    /// Sum of `piece_seconds`: the sequential-equivalent piece time. This is
    /// not elapsed time when the pieces ran concurrently.
    pub piece_seconds_sum: f64,
    /// Largest entry of `piece_seconds`.
    pub piece_seconds_max: f64,
    /// Plan replay, piece-problem re-derivation, restoration and merging in
    /// the assembling process.
    pub assembly_seconds: f64,
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
        "bilinear general reduction requires two fiber axes"
    );
    ensure!(
        !problem.solver.order.is_empty() || problem.solver.ordering == "given",
        "bilinear general reduction requires an explicit or given order"
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
    /// Original normalized constraints retained in every piece.
    base_constraints: Vec<usize>,
    /// a, b, c, d.
    coefficients: [Poly; 4],
    delta: Poly,
    a0: Poly,
    b0: Poly,
    guide: Option<Poly>,
    /// u_a, u_b, u_c, u_d of the guide.
    corners: Option<[Poly; 4]>,
}

fn polynomial_cap(p: &Poly) -> Result<()> {
    ensure!(
        p.nterms() <= 1 << 16 && (0..p.nvars()).all(|v| p.degree(v) <= 256),
        "bilinear general polynomial cap exceeded"
    );
    Ok(())
}

/// Term-product budget of one exact product step.
const PRODUCT_CAP: usize = 4_000_000;

fn within_product_cap(pairs: &[(&Poly, &Poly)], cap: usize) -> bool {
    let total = pairs.iter().fold(0usize, |acc, (p, q)| {
        acc.saturating_add(p.nterms().saturating_mul(q.nterms()))
    });
    total <= cap
}

fn product_cap(pairs: &[(&Poly, &Poly)]) -> Result<()> {
    ensure!(
        within_product_cap(pairs, PRODUCT_CAP),
        "bilinear general product cap exceeded"
    );
    Ok(())
}

fn input(problem: &Problem) -> Result<Input> {
    let order = selected_order(problem)?;
    ensure!(
        problem.solver.engine == "bilinear_general",
        "wrong bilinear general engine"
    );
    ensure!(
        problem.split.is_empty(),
        "bilinear general reduction requires an unsplit strict target"
    );
    ensure!(
        problem.solver.homogeneous_anchor.is_none(),
        "bilinear general reduction uses bilinear_general_base_anchor inside pieces, not an outer anchor"
    );
    let normalized = normalize(problem, &order)?;
    let x = order.len() - 1;
    let y = x - 1;
    let mut covered = [false; 2];
    let mut targets = Vec::new();
    let mut base_constraints = Vec::new();
    for (i, p) in normalized.positives.iter().enumerate() {
        match positive_axis(p) {
            Some(axis) if axis == y => covered[0] = true,
            Some(axis) if axis == x => covered[1] = true,
            _ if p.degree(x) == 0 && p.degree(y) == 0 => base_constraints.push(i),
            _ => targets.push(i),
        }
    }
    ensure!(
        covered == [true, true],
        "both fiber axes must be strictly positive coordinates"
    );
    ensure!(
        targets.len() == 1,
        "bilinear general reduction requires exactly one non-coordinate constraint in the fiber axes"
    );
    let target = targets[0];
    let f = &normalized.positives[target];
    polynomial_cap(f)?;
    ensure!(
        f.degree(x) <= 1 && f.degree(y) <= 1,
        "target must be affine in each final fiber axis"
    );
    let a0 = f.derivative(x);
    let b0 = f.replace(x, &Rational::zero());
    let a = a0.derivative(y);
    let b = a0.replace(y, &Rational::zero());
    let c = b0.derivative(y);
    let d = b0.replace(y, &Rational::zero());
    ensure!(
        !a.is_zero(),
        "identically zero x*y coefficient a is outside the bilinear general rule"
    );
    product_cap(&[(&b, &c), (&a, &d)])?;
    let delta = &(&b * &c) - &(&a * &d);
    // Also excludes c and d vanishing together (F=x*(a*y+b)) and b, d
    // vanishing together (F=y*(a*x+c)), so every zero alias is nonzero.
    ensure!(
        !delta.is_zero(),
        "identically zero Delta=b*c-a*d is outside the bilinear general rule"
    );
    let guide = problem
        .solver
        .bilinear_general_guide
        .as_ref()
        .map(|text| -> Result<Poly> {
            let mut proposed = problem.clone();
            proposed.split = vec![text.clone()];
            let guide = normalize(&proposed, &order)?.split.remove(0);
            polynomial_cap(&guide)?;
            ensure!(
                guide.degree(x) <= 1 && guide.degree(y) <= 1,
                "bilinear general guide must be affine in each final fiber axis"
            );
            Ok(guide)
        })
        .transpose()?;
    let corners = guide.as_ref().map(|g| {
        let g0 = g.derivative(x);
        let g1 = g.replace(x, &Rational::zero());
        [
            g0.derivative(y),
            g0.replace(y, &Rational::zero()),
            g1.derivative(y),
            g1.replace(y, &Rational::zero()),
        ]
    });
    Ok(Input {
        order,
        normalized,
        target,
        base_constraints,
        coefficients: [a, b, c, d],
        delta,
        a0,
        b0,
        guide,
        corners,
    })
}

fn check_time(problem: &Problem, started: Option<Instant>) -> Result<()> {
    if started.is_some_and(|s| s.elapsed().as_secs_f64() >= problem.limits.wall_time_secs) {
        return Err(crate::resultant_interpolation::Stopped(
            "bilinear general wall budget exhausted".into(),
        )
        .into());
    }
    Ok(())
}

type Facts = [i8; 5];

/// The partner whose strict sign an identically zero coefficient takes on the
/// open quadrant y>0: a*y+0 has the sign of a, c*y+0 that of c and 0*y+d that
/// of d. `input` declines a=0 and Delta=0, so no partner is itself zero.
fn zero_aliases(coefficients: &[Poly; 4]) -> [Option<usize>; 5] {
    let mut alias = [None; 5];
    for (i, partner) in [(1, 0), (2, 3), (3, 2)] {
        if coefficients[i].is_zero() {
            alias[i] = Some(partner);
        }
    }
    alias
}

/// Strict-sign propositional logic over the five atoms. This is not algebra:
/// every relation is backed by a replayed exact identity or certificate.
/// Facts of an identically zero coefficient are its fiber-table sign (see
/// [`zero_aliases`]); they are only ever copied from the partner atom.
#[derive(Clone, Debug)]
struct Logic {
    global: Facts,
    /// (lower, upper): lower>0 => upper>0 and upper<0 => lower<0.
    relations: Vec<(usize, usize)>,
    alias: [Option<usize>; 5],
    /// Sign of Delta wherever G != 0 (square separators), or 0 (G = Delta).
    separator_sign: i8,
}

fn both_roots(f: &Facts) -> bool {
    f[0] * f[1] < 0 && f[2] * f[3] < 0
}

/// Every coefficient sign is fixed and, when both breakpoints are positive,
/// so is the sign of G (also for square separators: the breakpoints meet on
/// G=0, so G must be a boundary there).
fn decided(f: &Facts) -> bool {
    f[..4].iter().all(|s| *s != 0) && (!both_roots(f) || f[4] != 0)
}

impl Logic {
    fn close(&self, mut facts: Facts) -> Option<Facts> {
        for i in 0..5 {
            if self.global[i] != 0 {
                if facts[i] != 0 && facts[i] != self.global[i] {
                    return None;
                }
                facts[i] = self.global[i];
            }
        }
        loop {
            let mut changed = false;
            for &(lower, upper) in &self.relations {
                if facts[lower] == 1 {
                    match facts[upper] {
                        -1 => return None,
                        0 => {
                            facts[upper] = 1;
                            changed = true;
                        }
                        _ => {}
                    }
                }
                if facts[upper] == -1 {
                    match facts[lower] {
                        1 => return None,
                        0 => {
                            facts[lower] = -1;
                            changed = true;
                        }
                        _ => {}
                    }
                }
            }
            for (i, partner) in self.alias.iter().enumerate() {
                let Some(j) = *partner else { continue };
                match (facts[i], facts[j]) {
                    (_, 0) => {}
                    (0, s) => {
                        facts[i] = s;
                        changed = true;
                    }
                    (s, t) if s != t => return None,
                    _ => {}
                }
            }
            if !changed {
                return Some(facts);
            }
        }
    }

    fn consistent(&self, v: &Facts) -> bool {
        (0..5).all(|i| self.global[i] == 0 || self.global[i] == v[i])
            && self
                .relations
                .iter()
                .all(|&(lower, upper)| !(v[lower] == 1 && v[upper] == -1))
            && self
                .alias
                .iter()
                .enumerate()
                .all(|(i, j)| j.is_none_or(|j| v[i] == v[j]))
    }

    fn delta_sign(&self, f: &Facts) -> i8 {
        if self.separator_sign != 0 {
            self.separator_sign
        } else {
            f[4]
        }
    }
}

fn all_negative(f: &Facts) -> bool {
    f[..4].iter().all(|s| *s == -1)
}

/// Fiber-row endpoints along y.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YEnd {
    Zero,
    /// Root of the slope a*y+b.
    RootA,
    /// Root of the intercept c*y+d.
    RootB,
    Infinity,
}

/// The x-fiber over one y-interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XFiber {
    /// (0, infinity): slope and intercept positive.
    Full,
    /// (0, root of F): negative slope, positive intercept.
    Below,
    /// (root of F, infinity): positive slope, negative intercept.
    Above,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FiberRow {
    pub lower: YEnd,
    pub upper: YEnd,
    pub x: XFiber,
}

/// General fiber table for all 16 strict sign patterns of (a,b,c,d). When
/// both breakpoints are positive their order is sign(Delta)*sign(a*c), since
/// (-d/c)-(-b/a) = Delta/(a*c). `delta` is only consulted in that case.
pub fn fiber_rows(signs: [i8; 4], delta: i8) -> Result<Vec<FiberRow>> {
    let [a, b, c, d] = signs;
    ensure!(
        signs.iter().all(|s| *s == 1 || *s == -1),
        "fiber table requires strict coefficient signs"
    );
    let root_a = a * b < 0;
    let root_b = c * d < 0;
    let mut ends = vec![YEnd::Zero];
    match (root_a, root_b) {
        (true, true) => {
            ensure!(
                delta == 1 || delta == -1,
                "both breakpoints are positive but their order is undecided"
            );
            if delta * a * c > 0 {
                ends.extend([YEnd::RootA, YEnd::RootB]);
            } else {
                ends.extend([YEnd::RootB, YEnd::RootA]);
            }
        }
        (true, false) => ends.push(YEnd::RootA),
        (false, true) => ends.push(YEnd::RootB),
        (false, false) => {}
    }
    ends.push(YEnd::Infinity);
    let mut rows = Vec::new();
    for (i, w) in ends.windows(2).enumerate() {
        let above = |end| ends[..=i].contains(&end);
        let slope = if root_a && above(YEnd::RootA) { a } else { b };
        let intercept = if root_b && above(YEnd::RootB) { c } else { d };
        let x = match (slope, intercept) {
            (1, 1) => XFiber::Full,
            (-1, 1) => XFiber::Below,
            (1, -1) => XFiber::Above,
            _ => continue,
        };
        rows.push(FiberRow {
            lower: w[0],
            upper: w[1],
            x,
        });
    }
    Ok(rows)
}

// ------------------------------------------------------------------ planning

#[derive(Debug)]
enum Node {
    Infeasible,
    Empty,
    Leaf,
    Branch(usize, Box<Node>, Box<Node>),
}

fn build(logic: &Logic, facts: Facts, order: &[usize]) -> Node {
    let Some(f) = logic.close(facts) else {
        return Node::Infeasible;
    };
    if decided(&f) {
        return if all_negative(&f) {
            Node::Empty
        } else {
            Node::Leaf
        };
    }
    let Some(&atom) = order.iter().find(|&&a| f[a] == 0) else {
        // Every free atom is fixed: a decided node was returned above.
        return Node::Infeasible;
    };
    let mut plus = f;
    plus[atom] = 1;
    let mut minus = f;
    minus[atom] = -1;
    Node::Branch(
        atom,
        Box::new(build(logic, plus, order)),
        Box::new(build(logic, minus, order)),
    )
}

fn canonical_conditions(path: &[Condition]) -> Vec<Condition> {
    let mut out = path.to_vec();
    out.sort_by_key(|c| c.atom);
    out
}

fn collect(
    node: &Node,
    path: &mut Vec<Condition>,
    pieces: &mut Vec<PieceSpec>,
    empty: &mut Vec<EmptyClaim>,
) {
    match node {
        Node::Infeasible => {}
        Node::Empty => empty.push(EmptyClaim {
            conditions: canonical_conditions(path),
        }),
        Node::Leaf => pieces.push(PieceSpec {
            conditions: canonical_conditions(path),
            split: vec![],
        }),
        Node::Branch(atom, plus, minus) => {
            if matches!((&**plus, &**minus), (Node::Leaf, Node::Leaf)) {
                pieces.push(PieceSpec {
                    conditions: canonical_conditions(path),
                    split: vec![ATOMS[*atom]],
                });
                return;
            }
            for (sign, child) in [(1, plus), (-1, minus)] {
                path.push(Condition {
                    atom: ATOMS[*atom],
                    sign,
                });
                collect(child, path, pieces, empty);
                path.pop();
            }
        }
    }
}

fn permutations(items: &[usize]) -> Vec<Vec<usize>> {
    if items.is_empty() {
        return vec![vec![]];
    }
    let mut out = Vec::new();
    for (i, &first) in items.iter().enumerate() {
        let mut rest = items.to_vec();
        rest.remove(i);
        for mut tail in permutations(&rest) {
            tail.insert(0, first);
            out.push(tail);
        }
    }
    out
}

fn consistent_vectors(logic: &Logic) -> Vec<Facts> {
    (0..32u32)
        .map(|mask| std::array::from_fn(|i| if mask >> i & 1 == 1 { 1 } else { -1 }))
        .filter(|v| logic.consistent(v))
        .collect()
}

/// Planner objective, `solver.bilinear_general_plan_objective`. It selects
/// among valid plans only; the verifier accepts any valid plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Objective {
    /// `region_weighted` (default): atom polynomials per piece, each weighted
    /// by the premise-consistent strict sign vectors (sign regions) the piece
    /// covers. A piece carries all of its atoms over its whole region, so this
    /// prefers not to carry an atom such as d over a region where another sign
    /// already implies d>0, at the price of more pieces. On Gluza33 it gives
    /// the four region pieces {b,c sign quadrants} of the structure analysis.
    RegionWeighted,
    /// `atoms`: the plain total of atom polynomials over all pieces, with the
    /// region-weighted count as the first tie break. On Gluza33 it merges the
    /// {b<0,c>0} and {b<0,c<0,d>0} pieces into {b<0,d>0} split c (3 pieces).
    Atoms,
}

impl Objective {
    pub fn parse(text: &str) -> Result<Self> {
        match text {
            "region_weighted" => Ok(Objective::RegionWeighted),
            "atoms" => Ok(Objective::Atoms),
            _ => bail!("bilinear_general_plan_objective must be region_weighted or atoms"),
        }
    }
}

/// Region-weighted and plain atom occurrences in pieces (see [`Objective`]).
fn plan_cost(logic: &Logic, pieces: &[PieceSpec]) -> (usize, usize) {
    let vectors = consistent_vectors(logic);
    let mut weighted = 0;
    let mut total = 0;
    for piece in pieces {
        let atoms = piece.conditions.len() + piece.split.len();
        let regions = vectors
            .iter()
            .filter(|v| matches(&piece.conditions, v))
            .count();
        weighted += atoms * regions;
        total += atoms;
    }
    (weighted, total)
}

/// Enumerate branching orders over the free atoms and keep the plan with the
/// smallest objective. `region_weighted` ties prefer fewer pieces, fewer empty
/// claims, fewer plain atom occurrences; `atoms` ties prefer fewer weighted
/// occurrences, fewer pieces, fewer empty claims. The last tie break is the
/// lexicographically first branching order.
fn plan_pieces(logic: &Logic, objective: Objective) -> (Vec<PieceSpec>, Vec<EmptyClaim>) {
    let Some(root) = logic.close([0; 5]) else {
        return (vec![], vec![]);
    };
    // Identically zero coefficients are never branched on: their fiber sign
    // follows their partner atom.
    let free: Vec<usize> = (0..5)
        .filter(|&i| root[i] == 0 && logic.alias[i].is_none())
        .collect();
    type Key = (usize, usize, usize, usize, Vec<usize>);
    let mut best: Option<(Key, Vec<PieceSpec>, Vec<EmptyClaim>)> = None;
    for order in permutations(&free) {
        let tree = build(logic, [0; 5], &order);
        let mut pieces = Vec::new();
        let mut empty = Vec::new();
        collect(&tree, &mut vec![], &mut pieces, &mut empty);
        let (weighted, total) = plan_cost(logic, &pieces);
        let key = match objective {
            Objective::RegionWeighted => (weighted, pieces.len(), empty.len(), total, order),
            Objective::Atoms => (total, weighted, pieces.len(), empty.len(), order),
        };
        if best.as_ref().is_none_or(|(k, _, _)| key < *k) {
            best = Some((key, pieces, empty));
        }
    }
    let (_, pieces, empty) = best.expect("at least one branching order");
    (pieces, empty)
}

// ------------------------------------------------------------- replay

struct Replayed {
    input: Input,
    /// a, b, c, d, G in the original lifting order algebra.
    atoms: [Poly; 5],
    logic: Logic,
}

fn atom_text(input: &Input, atoms: &[Poly; 5], atom: Atom) -> String {
    input.normalized.algebra.format(&atoms[atom.index()])
}

/// Replay every exact obligation of a plan except the piece solves.
fn replay_plan(problem: &Problem, plan: &BilinearGeneralPlan) -> Result<Replayed> {
    ensure!(
        plan.schema_version == 1,
        "unsupported bilinear general plan"
    );
    let input = input(problem)?;
    let algebra = &input.normalized.algebra;
    ensure!(
        plan.order == input.order && plan.target_constraint == input.target,
        "bilinear general order or target changed"
    );
    ensure!(
        plan.coefficients.len() == 4
            && plan
                .coefficients
                .iter()
                .zip(&input.coefficients)
                .map(|(text, p)| Ok(algebra.parse(text)? == *p))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .all(|same| same),
        "bilinear general coefficient identity changed"
    );
    match (&plan.guide, &input.guide) {
        (None, None) => {}
        (Some(text), Some(guide)) => ensure!(
            algebra.parse(text)? == *guide,
            "bilinear general guide changed"
        ),
        _ => bail!("bilinear general guide presence changed"),
    }
    let n = input.order.len() - 2;
    let fiber_free = |p: &Poly| (n..n + 2).all(|v| p.degree(v) == 0);
    let mut certified_corners = [false; 4];
    for entry in &plan.guide_positivity {
        let corners = input
            .corners
            .as_ref()
            .ok_or_else(|| anyhow!("guide positivity without guide"))?;
        ensure!(entry.corner != Atom::G, "G is not a guide corner");
        let k = entry.corner.index();
        ensure!(!certified_corners[k], "duplicate guide corner certificate");
        verify_certificate(
            algebra,
            &corners[k],
            &input.normalized.positives,
            &[input.target],
            &entry.certificate,
        )?;
        ensure!(
            entry.certificate.sign == 1,
            "guide corners require strictly positive certificates"
        );
        certified_corners[k] = true;
    }
    // Separator.
    let separator = match &plan.separator {
        Separator::Square { multiplier, square } => {
            let multiplier = rational(multiplier)?;
            let square = algebra.parse(square)?;
            ensure!(fiber_free(&square), "separator depends on a fiber axis");
            check_positive_square_identity(&input.delta, &multiplier, &square)?;
            square
        }
        Separator::NegativeSquare { multiplier, square } => {
            let multiplier = rational(multiplier)?;
            let square = algebra.parse(square)?;
            ensure!(fiber_free(&square), "separator depends on a fiber axis");
            check_positive_square_identity(&-input.delta.clone(), &multiplier, &square)?;
            square
        }
        Separator::Delta { polynomial } => {
            let p = algebra.parse(polynomial)?;
            ensure!(p == input.delta, "separator is not Delta");
            p
        }
    };
    let [a, b, c, d] = input.coefficients.clone();
    let atoms = [a, b, c, d, separator];
    let alias = zero_aliases(&input.coefficients);
    let mut global = [0i8; 5];
    for (i, p) in atoms.iter().enumerate() {
        if p.is_constant() && alias[i].is_none() {
            global[i] = sign(&p.get_constant());
            ensure!(global[i] != 0, "zero atom");
        }
    }
    let mut relations = Vec::new();
    for premise in &plan.premises {
        match premise {
            SignPremise::RatioOrder {
                lower,
                upper,
                multiplier,
                square,
                strict,
            } => {
                ensure!(
                    lower != upper && *lower != Atom::G && *upper != Atom::G,
                    "ratio orders compare distinct coefficients"
                );
                // A ratio order speaks about true signs; a zero coefficient's
                // fact is only its fiber-table alias.
                ensure!(
                    alias[lower.index()].is_none() && alias[upper.index()].is_none(),
                    "ratio orders cannot involve an identically zero coefficient"
                );
                let corners = input
                    .corners
                    .as_ref()
                    .ok_or_else(|| anyhow!("ratio order requires a guide"))?;
                let (l, u) = (lower.index(), upper.index());
                ensure!(
                    certified_corners[l] && certified_corners[u],
                    "ratio order uses an uncertified guide corner"
                );
                let lhs = &(&atoms[u] * &corners[l]) - &(&atoms[l] * &corners[u]);
                let multiplier = rational(multiplier)?;
                let square = algebra.parse(square)?;
                ensure!(fiber_free(&square), "ratio witness depends on a fiber axis");
                check_positive_square_identity(&lhs, &multiplier, &square)?;
                if let Some(certificate) = strict {
                    verify_certificate(
                        algebra,
                        &square,
                        &input.normalized.positives,
                        &[input.target],
                        certificate,
                    )?;
                }
                relations.push((l, u));
            }
            SignPremise::GlobalSign { atom, certificate } => {
                let i = atom.index();
                ensure!(
                    alias[i].is_none(),
                    "an identically zero coefficient has no strict global sign"
                );
                verify_certificate(
                    algebra,
                    &atoms[i],
                    &input.normalized.positives,
                    &[input.target],
                    certificate,
                )?;
                ensure!(
                    global[i] == 0 || global[i] == certificate.sign,
                    "conflicting global atom signs"
                );
                global[i] = certificate.sign;
            }
        }
    }
    let logic = Logic {
        global,
        relations,
        alias,
        separator_sign: plan.separator.delta_sign(),
    };
    check_partition(&logic, &atoms, plan)?;
    Ok(Replayed {
        input,
        atoms,
        logic,
    })
}

fn check_shape(conditions: &[Condition], split: &[Atom], atoms: &[Poly; 5]) -> Result<()> {
    ensure!(
        conditions.windows(2).all(|w| w[0].atom < w[1].atom)
            && split.windows(2).all(|w| w[0] < w[1]),
        "piece atoms must be distinct and in canonical order"
    );
    ensure!(
        conditions.iter().all(|c| c.sign == 1 || c.sign == -1),
        "piece conditions must be strict"
    );
    ensure!(
        conditions.iter().all(|c| !split.contains(&c.atom)),
        "an atom cannot be both a condition and a split"
    );
    ensure!(
        conditions
            .iter()
            .map(|c| c.atom)
            .chain(split.iter().copied())
            .all(|a| !atoms[a.index()].is_constant()),
        "constant atoms cannot be piece conditions or splits"
    );
    Ok(())
}

fn matches(conditions: &[Condition], v: &Facts) -> bool {
    conditions.iter().all(|c| v[c.atom.index()] == c.sign)
}

/// Independent partition and leaf check; the planner is not trusted.
fn check_partition(logic: &Logic, atoms: &[Poly; 5], plan: &BilinearGeneralPlan) -> Result<()> {
    for piece in &plan.pieces {
        check_shape(&piece.conditions, &piece.split, atoms)?;
    }
    for claim in &plan.empty {
        check_shape(&claim.conditions, &[], atoms)?;
        let mut facts = [0i8; 5];
        for c in &claim.conditions {
            facts[c.atom.index()] = c.sign;
        }
        ensure!(
            logic.close(facts).is_none_or(|f| all_negative(&f)),
            "empty claim does not close to a contradiction or an all-negative pattern"
        );
    }
    for mask in 0..32u32 {
        let v: Facts = std::array::from_fn(|i| if mask >> i & 1 == 1 { 1 } else { -1 });
        if !logic.consistent(&v) {
            continue;
        }
        let mut claims = 0;
        for piece in plan.pieces.iter().filter(|p| matches(&p.conditions, &v)) {
            claims += 1;
            let mut facts = [0i8; 5];
            for c in &piece.conditions {
                facts[c.atom.index()] = c.sign;
            }
            for a in &piece.split {
                facts[a.index()] = v[a.index()];
            }
            let closed = logic
                .close(facts)
                .ok_or_else(|| anyhow!("consistent sign vector closes to a contradiction"))?;
            ensure!(
                decided(&closed),
                "piece leaf {v:?} does not determine the fiber table"
            );
        }
        claims += plan
            .empty
            .iter()
            .filter(|c| matches(&c.conditions, &v))
            .count();
        ensure!(
            claims == 1,
            "base pieces do not partition sign vector {v:?} ({claims} claims)"
        );
    }
    Ok(())
}

// ------------------------------------------------------------- discovery

/// Propose the ratio order between coefficients i<j from their certified
/// guide corners: atoms[j]*u_i - atoms[i]*u_j = m*s^2 (i below j) or its
/// negative (j below i). A pair over the product cap or without a square
/// witness is only a missing premise, never an error.
fn ratio_premise(
    algebra: &crate::algebra::Algebra,
    domain: &crate::domain::Domain,
    options: &CertificateOptions,
    (i, j): (usize, usize),
    [(atom_i, corner_i), (atom_j, corner_j)]: [(&Poly, &Poly); 2],
    cap: usize,
    check_time: impl Fn() -> Result<()>,
) -> Result<Option<SignPremise>> {
    if !within_product_cap(&[(atom_j, corner_i), (atom_i, corner_j)], cap) {
        return Ok(None);
    }
    let residual = &(atom_j * corner_i) - &(atom_i * corner_j);
    let found = match positive_square_witness(&residual) {
        Some(w) => Some((i, j, w)),
        None => {
            check_time()?;
            positive_square_witness(&-residual.clone()).map(|w| (j, i, w))
        }
    };
    let Some((lower, upper, (multiplier, square))) = found else {
        return Ok(None);
    };
    check_time()?;
    let strict = if square.is_constant() {
        None
    } else {
        certify_nonzero(algebra, &square, domain, options)?
    };
    Ok(Some(SignPremise::RatioOrder {
        lower: ATOMS[lower],
        upper: ATOMS[upper],
        multiplier: multiplier.to_string(),
        square: algebra.format(&square),
        strict,
    }))
}

fn discover(
    problem: &Problem,
    started: Option<Instant>,
) -> Result<(BilinearGeneralPlan, Replayed)> {
    check_time(problem, started)?;
    let input = input(problem)?;
    let algebra = &input.normalized.algebra;
    let options = CertificateOptions::default();
    let domain = derive_domain(algebra, &input.normalized.positives, &[input.target])?;
    let mut guide_positivity = Vec::new();
    let mut certified = [false; 4];
    if let Some(corners) = &input.corners {
        for (k, corner) in corners.iter().enumerate() {
            check_time(problem, started)?;
            if corner.is_zero() {
                continue;
            }
            if let Some(certificate) =
                certify_nonzero(algebra, corner, &domain, &options)?.filter(|c| c.sign == 1)
            {
                certified[k] = true;
                guide_positivity.push(CornerCertificate {
                    corner: ATOMS[k],
                    certificate,
                });
            }
        }
    }
    check_time(problem, started)?;
    let mut separator = positive_square_witness(&input.delta).map(|(m, s)| Separator::Square {
        multiplier: m.to_string(),
        square: algebra.format(&s),
    });
    if separator.is_none() {
        check_time(problem, started)?;
        separator = positive_square_witness(&-input.delta.clone()).map(|(m, s)| {
            Separator::NegativeSquare {
                multiplier: m.to_string(),
                square: algebra.format(&s),
            }
        });
    }
    let separator = separator.unwrap_or_else(|| Separator::Delta {
        polynomial: algebra.format(&input.delta),
    });
    let g = match &separator {
        Separator::Square { square, .. } | Separator::NegativeSquare { square, .. } => {
            algebra.parse(square)?
        }
        Separator::Delta { .. } => input.delta.clone(),
    };
    let [a, b, c, d] = input.coefficients.clone();
    let atoms = [a, b, c, d, g];
    let alias = zero_aliases(&input.coefficients);
    let mut premises = Vec::new();
    if let Some(corners) = &input.corners {
        for i in 0..4 {
            for j in i + 1..4 {
                check_time(problem, started)?;
                if !(certified[i] && certified[j]) || alias[i].is_some() || alias[j].is_some() {
                    continue;
                }
                let check = || check_time(problem, started);
                let pair = [(&atoms[i], &corners[i]), (&atoms[j], &corners[j])];
                if let Some(premise) =
                    ratio_premise(algebra, &domain, &options, (i, j), pair, PRODUCT_CAP, check)?
                {
                    premises.push(premise);
                }
            }
        }
    }
    for (i, p) in atoms.iter().enumerate() {
        check_time(problem, started)?;
        if p.is_constant() {
            // Nonzero constants are global facts; zero coefficients alias.
            continue;
        }
        if let Some(certificate) = certify_nonzero(algebra, p, &domain, &options)? {
            premises.push(SignPremise::GlobalSign {
                atom: ATOMS[i],
                certificate,
            });
        }
    }
    let mut plan = BilinearGeneralPlan {
        schema_version: 1,
        target_constraint: input.target,
        order: input.order.clone(),
        coefficients: input
            .coefficients
            .iter()
            .map(|p| algebra.format(p))
            .collect(),
        guide: input.guide.as_ref().map(|g| algebra.format(g)),
        guide_positivity,
        premises,
        separator,
        pieces: vec![],
        empty: vec![],
    };
    // The planner runs on the same logic the verifier replays.
    let premise_logic = {
        let mut global = [0i8; 5];
        for (i, p) in atoms.iter().enumerate() {
            if p.is_constant() && alias[i].is_none() {
                global[i] = sign(&p.get_constant());
            }
        }
        let mut relations = Vec::new();
        for premise in &plan.premises {
            match premise {
                SignPremise::RatioOrder { lower, upper, .. } => {
                    relations.push((lower.index(), upper.index()))
                }
                SignPremise::GlobalSign { atom, certificate } => {
                    global[atom.index()] = certificate.sign
                }
            }
        }
        Logic {
            global,
            relations,
            alias,
            separator_sign: plan.separator.delta_sign(),
        }
    };
    let objective = Objective::parse(&problem.solver.bilinear_general_plan_objective)?;
    let (pieces, empty) = plan_pieces(&premise_logic, objective);
    plan.pieces = pieces;
    plan.empty = empty;
    check_time(problem, started)?;
    let replayed = replay_plan(problem, &plan)?;
    Ok((plan, replayed))
}

// ---------------------------------------------------------- piece problems

fn piece_problem(
    problem: &Problem,
    replayed: &Replayed,
    plan: &BilinearGeneralPlan,
    index: usize,
) -> Result<Problem> {
    let input = &replayed.input;
    let spec = plan
        .pieces
        .get(index)
        .ok_or_else(|| anyhow!("unknown bilinear general piece"))?;
    let n = input.order.len() - 2;
    let algebra = &input.normalized.algebra;
    let mut base = problem.clone();
    base.name = Some(format!(
        "{} [bilinear_general piece {index}]",
        problem.name.as_deref().unwrap_or("bilinear_general")
    ));
    base.variables = input.order[..n].to_vec();
    base.parameters.clear();
    base.constraints = input
        .base_constraints
        .iter()
        .map(|&i| format!("({})>0", algebra.format(&input.normalized.positives[i])))
        .collect();
    for condition in &spec.conditions {
        base.constraints.push(format!(
            "({}){}0",
            atom_text(input, &replayed.atoms, condition.atom),
            if condition.sign > 0 { ">" } else { "<" }
        ));
    }
    base.split = spec
        .split
        .iter()
        .map(|a| atom_text(input, &replayed.atoms, *a))
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
    base.solver.engine = problem.solver.bilinear_general_base_engine.clone();
    if base.solver.engine == "baseline" {
        base.solver.positivity = false;
        base.solver.input_positivity = false;
        base.solver.prune_operations = false;
        base.solver.weighted_square_positivity = false;
        base.solver.quadratic_component_positivity = false;
    }
    base.solver.ordering = "given".into();
    base.solver.order = base.variables.clone();
    base.solver.bilinear_general_guide = None;
    base.solver.bilinear_general_base_anchor = None;
    base.solver.bilinear_general_base_block.clear();
    base.solver.bilinear_general_base_engine = "optimized".into();
    base.solver.bilinear_general_plan_objective = "region_weighted".into();
    base.solver.bilinear_positive_guide = None;
    base.solver.negative_linear_guide = None;
    base.solver.local_strict_bezout_hints.clear();
    base.solver.merge = "none".into();
    match &problem.solver.bilinear_general_base_anchor {
        Some(anchor) => {
            // The default scale block is every base variable. A declared block
            // keeps kinematic parameters outside it.
            let declared = &problem.solver.bilinear_general_base_block;
            let position = |v: &String| base.variables.iter().position(|w| w == v);
            ensure!(
                declared.iter().all(|v| position(v).is_some()),
                "the inner homogeneous block may contain base variables only"
            );
            let mut block: Vec<String> = if declared.is_empty() {
                base.variables.clone()
            } else {
                declared.clone()
            };
            block.sort_by_key(|v| position(v));
            ensure!(
                block.contains(anchor),
                "the inner homogeneous anchor must be a base variable in its block"
            );
            ensure!(
                block.iter().all(|v| position(v) >= position(anchor)),
                "the inner homogeneous anchor must precede every other block variable in the base order"
            );
            base.solver.homogeneous_anchor = Some(anchor.clone());
            base.solver.homogeneous_block = block;
        }
        None => {
            ensure!(
                problem.solver.bilinear_general_base_block.is_empty(),
                "bilinear_general_base_block requires bilinear_general_base_anchor"
            );
            base.solver.homogeneous_anchor = None;
            base.solver.homogeneous_block.clear();
        }
    }
    base.validate()?;
    Ok(base)
}

// ---------------------------------------------------------- reconstruction

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

fn shift(bound: &Option<RootBound>, offset: usize) -> Option<RootBound> {
    bound.as_ref().map(|b| RootBound {
        polynomial: b.polynomial + offset,
        ..b.clone()
    })
}

/// Emit each expected cell exactly once; also the streaming verifier.
fn reconstruct(
    replayed: &Replayed,
    plan: &BilinearGeneralPlan,
    bases: &[SolveResult],
    mut emit: impl FnMut(Cell) -> Result<bool>,
) -> Result<Layout> {
    let input = &replayed.input;
    let logic = &replayed.logic;
    let n = input.order.len() - 2;
    ensure!(
        bases.len() == plan.pieces.len(),
        "bilinear general piece inventory changed"
    );
    let algebra = &input.normalized.algebra;
    let mut layout = Layout {
        table: vec![],
        levels: vec![vec![]; n + 2],
        exceptional: vec![],
        stopped: false,
    };
    let mut offsets = Vec::with_capacity(bases.len());
    for base in bases {
        ensure!(
            base.order == input.order[..n],
            "bilinear general piece order changed"
        );
        ensure!(
            base.projection_levels.len() == n,
            "bilinear general piece level count changed"
        );
        let offset = layout.table.len();
        offsets.push(offset);
        for p in &base.polynomials {
            layout.table.push(algebra.format(&algebra.parse(p)?));
        }
        let count = base.polynomials.len();
        for (level, ids) in base.projection_levels.iter().enumerate() {
            for &id in ids {
                ensure!(id < count, "invalid piece projection polynomial id");
                layout.levels[level].push(id + offset);
            }
        }
        for &id in &base.exceptional_polynomials {
            ensure!(id < count, "invalid piece exceptional polynomial id");
            layout.exceptional.push(id + offset);
        }
    }
    for p in &replayed.atoms {
        if !p.is_constant() {
            append(input, p, &mut layout);
        }
    }
    for variable in &input.order[..n] {
        append(input, &algebra.parse(variable)?, &mut layout);
    }
    let yid = append(input, &algebra.parse(&input.order[n])?, &mut layout);
    let xid = append(input, &algebra.parse(&input.order[n + 1])?, &mut layout);
    let aid = append(input, &input.a0, &mut layout);
    let bid = append(input, &input.b0, &mut layout);
    let target = &input.normalized.positives[input.target];
    let fid = append(input, target, &mut layout);
    layout.exceptional.sort_unstable();
    layout.exceptional.dedup();
    let zero = Rational::zero();
    for ((spec, base), offset) in plan.pieces.iter().zip(bases).zip(offsets) {
        for cell in &base.cells {
            ensure!(
                cell.axes.len() == n && cell.signs.len() == spec.split.len(),
                "invalid bilinear general piece cell shape"
            );
            let mut point = cell
                .axes
                .iter()
                .map(|axis| rational(&axis.sample))
                .collect::<Result<Vec<_>>>()?;
            point.resize(n + 2, Rational::zero());
            let values: [Rational; 5] =
                std::array::from_fn(|i| evaluate(&replayed.atoms[i], &point));
            let signs: Facts = std::array::from_fn(|i| sign(&values[i]));
            ensure!(
                spec.conditions
                    .iter()
                    .all(|c| signs[c.atom.index()] == c.sign),
                "piece cell violates its strict path conditions"
            );
            ensure!(
                cell.signs.iter().all(|s| *s != 0)
                    && spec
                        .split
                        .iter()
                        .zip(&cell.signs)
                        .all(|(a, s)| signs[a.index()] == *s),
                "piece cell split signs changed"
            );
            let mut facts = [0i8; 5];
            for c in &spec.conditions {
                facts[c.atom.index()] = c.sign;
            }
            for (a, s) in spec.split.iter().zip(&cell.signs) {
                facts[a.index()] = *s;
            }
            let facts = logic
                .close(facts)
                .ok_or_else(|| anyhow!("piece cell premises are contradictory"))?;
            ensure!(
                decided(&facts),
                "piece cell does not determine the fiber table"
            );
            // An identically zero coefficient keeps its partner's fiber sign.
            ensure!(
                (0..4).all(|i| match logic.alias[i] {
                    Some(_) => signs[i] == 0,
                    None => signs[i] == facts[i],
                }),
                "piece cell sample contradicts its derived coefficient signs"
            );
            let delta = logic.delta_sign(&facts);
            let [a, b, c, d, _] = values.clone();
            if both_roots(&facts) {
                ensure!(
                    signs[4] == facts[4] && sign(&(&(&b * &c) - &(&a * &d))) == delta,
                    "piece cell sample contradicts its root order"
                );
            }
            let rows = fiber_rows([facts[0], facts[1], facts[2], facts[3]], delta)?;
            let end = |e: YEnd| -> Option<(Rational, usize)> {
                match e {
                    YEnd::Zero => Some((zero.clone(), yid)),
                    YEnd::RootA => Some((-b.clone() / &a, aid)),
                    YEnd::RootB => Some((-d.clone() / &c, bid)),
                    YEnd::Infinity => None,
                }
            };
            for row in rows {
                let lower = end(row.lower).ok_or_else(|| anyhow!("row starts at infinity"))?;
                let upper = end(row.upper);
                ensure!(
                    lower.0 >= zero && upper.as_ref().is_none_or(|u| u.0 > lower.0),
                    "empty bilinear general y interval"
                );
                let ys = upper.as_ref().map_or_else(
                    || &lower.0 + &Rational::one(),
                    |u| (&lower.0 + &u.0) / Rational::from(2),
                );
                point[n] = ys.clone();
                let slope = &a * &ys + &b;
                let intercept = &c * &ys + &d;
                let (expected_slope, expected_intercept) = match row.x {
                    XFiber::Full => (1, 1),
                    XFiber::Below => (-1, 1),
                    XFiber::Above => (1, -1),
                };
                ensure!(
                    sign(&slope) == expected_slope && sign(&intercept) == expected_intercept,
                    "bilinear general row hypotheses failed"
                );
                let (x_lower, x_upper, xs) = match row.x {
                    XFiber::Full => (bound(xid, &zero), None, Rational::one()),
                    XFiber::Below => {
                        let root = -intercept / slope;
                        let xs = &root / &Rational::from(2);
                        (bound(xid, &zero), Some(bound(fid, &root)), xs)
                    }
                    XFiber::Above => {
                        let root = -intercept / slope;
                        let xs = &root + &Rational::one();
                        (bound(fid, &root), None, xs)
                    }
                };
                point[n + 1] = xs.clone();
                ensure!(
                    evaluate(target, &point) > Rational::zero(),
                    "restored sample is not in the original target"
                );
                let mut axes: Vec<Axis> = cell
                    .axes
                    .iter()
                    .map(|axis| Axis {
                        variable: axis.variable.clone(),
                        sample: axis.sample.clone(),
                        lower: shift(&axis.lower, offset),
                        upper: shift(&axis.upper, offset),
                    })
                    .collect();
                axes.push(Axis {
                    variable: input.order[n].clone(),
                    sample: ys.to_string(),
                    lower: Some(bound(lower.1, &lower.0)),
                    upper: upper.as_ref().map(|u| bound(u.1, &u.0)),
                });
                axes.push(Axis {
                    variable: input.order[n + 1].clone(),
                    sample: xs.to_string(),
                    lower: Some(x_lower),
                    upper: x_upper,
                });
                if !emit(Cell {
                    axes,
                    signs: vec![],
                })? {
                    layout.stopped = true;
                    return Ok(layout);
                }
            }
        }
    }
    Ok(layout)
}

// ------------------------------------------------------------- public API

/// A deterministic plan together with its ordinary piece problems.
pub struct PlannedPieces {
    pub plan: BilinearGeneralPlan,
    pub problems: Vec<Problem>,
    pub planning_seconds: f64,
}

/// Discover premises and pieces without solving them. Each returned problem
/// is an ordinary symGCAD input that can be solved by a separate process.
/// Discovery honours `limits.wall_time_secs` between its exact steps; the CLI
/// additionally runs it in a deadline-bounded worker process.
pub fn plan(problem: &Problem) -> Result<PlannedPieces> {
    let started = Instant::now();
    let (plan, replayed) = discover(problem, Some(started))?;
    let problems = (0..plan.pieces.len())
        .map(|i| piece_problem(problem, &replayed, &plan, i))
        .collect::<Result<Vec<_>>>()?;
    Ok(PlannedPieces {
        plan,
        problems,
        planning_seconds: started.elapsed().as_secs_f64(),
    })
}

/// Replay a recorded plan and derive its piece problems.
pub fn piece_problems(problem: &Problem, plan: &BilinearGeneralPlan) -> Result<Vec<Problem>> {
    let replayed = replay_plan(problem, plan)?;
    (0..plan.pieces.len())
        .map(|i| piece_problem(problem, &replayed, plan, i))
        .collect()
}

fn same_piece_problem(expected: &Problem, actual: &Problem) -> Result<bool> {
    let delegated = actual.limits.wall_time_secs;
    if !(delegated.is_finite() && delegated > 0. && delegated <= expected.limits.wall_time_secs) {
        return Ok(false);
    }
    let mut expected = expected.clone();
    expected.limits.wall_time_secs = delegated;
    Ok(serde_json::to_value(&expected)? == serde_json::to_value(actual)?)
}

/// Order separately solved piece results by their exact piece problems.
pub fn order_piece_results(
    problem: &Problem,
    plan: &BilinearGeneralPlan,
    results: Vec<SolveResult>,
) -> Result<Vec<SolveResult>> {
    let expected = piece_problems(problem, plan)?;
    ensure!(
        results.len() == expected.len(),
        "expected {} piece results, got {}",
        expected.len(),
        results.len()
    );
    let mut slots: Vec<Option<SolveResult>> = vec![None; expected.len()];
    for result in results {
        let mut target = None;
        for (i, wanted) in expected.iter().enumerate() {
            if slots[i].is_none() && same_piece_problem(wanted, &result.problem)? {
                target = Some(i);
                break;
            }
        }
        let i = target.ok_or_else(|| {
            anyhow!(
                "piece result {:?} matches no remaining piece problem",
                result.problem.name
            )
        })?;
        slots[i] = Some(result);
    }
    Ok(slots.into_iter().map(Option::unwrap).collect())
}

/// Phase timings of this process. Piece phases belong to it only when the
/// pieces ran inside it; work counters are always summed over pieces.
fn aggregate(bases: &[SolveResult], execution: Execution, planning_seconds: f64) -> Statistics {
    let mut statistics = Statistics::default();
    for base in bases {
        let s = &base.statistics;
        statistics.visited_stacks += s.visited_stacks;
        statistics.rejected_branches += s.rejected_branches;
        if execution == Execution::Sequential {
            statistics.ordering_seconds += s.ordering_seconds;
            statistics.search_seconds += s.search_seconds;
            statistics.projection_seconds += s.projection_seconds;
            statistics.lifting_seconds += s.lifting_seconds;
            statistics.merging_seconds += s.merging_seconds;
        }
    }
    if execution == Execution::Sequential {
        // As in the other composite engines, planning is pre-solve ordering.
        statistics.ordering_seconds += planning_seconds;
    }
    statistics
}

fn piece_timings(
    bases: &[SolveResult],
    execution: Execution,
    planning_seconds: f64,
    assembly_seconds: f64,
) -> PieceTimings {
    let piece_seconds: Vec<f64> = bases.iter().map(|b| b.statistics.total_seconds).collect();
    PieceTimings {
        execution,
        planning_seconds,
        piece_seconds_sum: piece_seconds.iter().sum(),
        piece_seconds_max: piece_seconds.iter().copied().fold(0.0, f64::max),
        piece_seconds,
        assembly_seconds,
    }
}

/// `started` is the start of this process's work: the whole sequential
/// solve, or the assembly of separately solved pieces.
fn finish(
    problem: &Problem,
    replayed: Replayed,
    plan: BilinearGeneralPlan,
    bases: Vec<SolveResult>,
    started: Instant,
    execution: Execution,
    planning_seconds: f64,
) -> Result<SolveResult> {
    let finishing = Instant::now();
    let input = &replayed.input;
    let mut statistics = aggregate(&bases, execution, planning_seconds);
    let failed = bases
        .iter()
        .enumerate()
        .find(|(_, b)| b.status != Status::CompleteGeneric);
    let mut result = match failed {
        Some((i, base)) => SolveResult::failure(
            problem.clone(),
            base.status.clone(),
            format!(
                "bilinear general piece {i}: {}",
                base.diagnostic.as_deref().unwrap_or("incomplete")
            ),
        ),
        None if bases.len() != plan.pieces.len() => SolveResult::failure(
            problem.clone(),
            Status::Incomplete,
            "bilinear general wall budget exhausted before every piece was solved",
        ),
        None => SolveResult::failure(problem.clone(), Status::CompleteGeneric, ""),
    };
    result.diagnostic = result.diagnostic.filter(|d| !d.is_empty());
    result.order = input.order.clone();
    result.normalized_constraints = input
        .normalized
        .positives
        .iter()
        .map(|p| input.normalized.algebra.format(p))
        .collect();
    if result.status == Status::CompleteGeneric {
        let restored = Instant::now();
        let mut cells = Vec::new();
        let mut reason = None;
        let layout = reconstruct(&replayed, &plan, &bases, |cell| {
            if check_time(problem, Some(started)).is_err() {
                reason = Some("bilinear general restoration reached wall budget".to_string());
                return Ok(false);
            }
            if problem
                .limits
                .max_cells
                .is_some_and(|cap| cells.len() >= cap)
            {
                reason = Some("bilinear general restoration reached cell limit".to_string());
                return Ok(false);
            }
            cells.push(cell);
            Ok(true)
        })?;
        statistics.lifting_seconds += restored.elapsed().as_secs_f64();
        result.polynomials = layout.table;
        result.projection_levels = layout.levels;
        result.exceptional_polynomials = layout.exceptional;
        result.cells = cells;
        if layout.stopped {
            result.status = Status::Incomplete;
            result.diagnostic = reason;
        }
        let merge_start = Instant::now();
        result.merged_cells = (problem.solver.merge == "aggressive").then(|| merge(&result.cells));
        statistics.merging_seconds += merge_start.elapsed().as_secs_f64();
    }
    statistics.raw_cells = result.cells.len();
    result.statistics = statistics;
    result.bilinear_general = Some(Box::new(BilinearGeneralEvidence { plan, bases }));
    if let Err(e) = check_time(problem, Some(started)) {
        result.status = Status::Incomplete;
        result.diagnostic = Some(e.to_string());
    }
    let elapsed = started.elapsed().as_secs_f64();
    let assembly_seconds = match execution {
        Execution::Sequential => finishing.elapsed().as_secs_f64(),
        Execution::SeparateProcesses => elapsed,
    };
    let evidence = result
        .bilinear_general
        .as_ref()
        .expect("evidence attached above");
    result.statistics.bilinear_general_pieces = Some(piece_timings(
        &evidence.bases,
        execution,
        planning_seconds,
        assembly_seconds,
    ));
    // Elapsed wall time of this process only, never a sum over processes.
    result.statistics.total_seconds = elapsed;
    Ok(result)
}

/// Assemble separately solved piece results (in plan order, see
/// [`order_piece_results`]) into one original-coordinate result. The pieces
/// may have run concurrently, so the result's `total_seconds` and phase
/// timings cover this assembly only; `statistics.bilinear_general_pieces`
/// reports the recorded planning time and every piece's own total.
pub fn assemble(
    problem: &Problem,
    plan: BilinearGeneralPlan,
    bases: Vec<SolveResult>,
    planning_seconds: f64,
) -> Result<SolveResult> {
    let started = Instant::now();
    ensure!(
        planning_seconds.is_finite() && planning_seconds >= 0.0,
        "recorded planning time must be a finite nonnegative number"
    );
    let replayed = replay_plan(problem, &plan)?;
    ensure!(
        bases.len() == plan.pieces.len(),
        "bilinear general assembly requires one result per piece"
    );
    for (i, base) in bases.iter().enumerate() {
        let expected = piece_problem(problem, &replayed, &plan, i)?;
        ensure!(
            same_piece_problem(&expected, &base.problem)?,
            "piece result {i} does not solve its exact piece problem"
        );
    }
    finish(
        problem,
        replayed,
        plan,
        bases,
        started,
        Execution::SeparateProcesses,
        planning_seconds,
    )
}

/// Plan, solve every piece sequentially, and assemble.
pub fn solve(problem: &Problem) -> Result<SolveResult> {
    let started = Instant::now();
    let (plan, replayed) = match discover(problem, Some(started)) {
        Ok(v) => v,
        Err(e)
            if e.downcast_ref::<crate::resultant_interpolation::Stopped>()
                .is_some() =>
        {
            let mut result =
                SolveResult::failure(problem.clone(), Status::Incomplete, e.to_string());
            result.statistics.ordering_seconds = started.elapsed().as_secs_f64();
            result.statistics.total_seconds = result.statistics.ordering_seconds;
            return Ok(result);
        }
        Err(e) => return Err(e),
    };
    let planning = started.elapsed().as_secs_f64();
    let mut bases = Vec::with_capacity(plan.pieces.len());
    for i in 0..plan.pieces.len() {
        let remaining = problem.limits.wall_time_secs - started.elapsed().as_secs_f64();
        if remaining <= 0. {
            break;
        }
        let mut piece = piece_problem(problem, &replayed, &plan, i)?;
        piece.limits.wall_time_secs = remaining;
        let base = crate::solver::solve(&piece)?;
        let complete = base.status == Status::CompleteGeneric;
        bases.push(base);
        if !complete {
            break;
        }
    }
    finish(
        problem,
        replayed,
        plan,
        bases,
        started,
        Execution::Sequential,
        planning,
    )
}

pub fn verify(result: &SolveResult, evidence: &BilinearGeneralEvidence) -> Result<Verification> {
    ensure!(
        result.schema_version == 1 && result.status == Status::CompleteGeneric,
        "bilinear general result is incomplete"
    );
    ensure!(
        result.trace.is_none()
            && result.homogeneous.is_none()
            && result.monotone.is_none()
            && result.local.is_none()
            && result.sign_filter.is_none()
            && result.linear_negative.is_none()
            && result.bilinear_positive.is_none()
            && result.quadratic_ladder.is_none(),
        "mixed bilinear general proof modes"
    );
    let plan = &evidence.plan;
    let replayed = replay_plan(&result.problem, plan)?;
    let input = &replayed.input;
    ensure!(
        result.order == input.order,
        "bilinear general order changed"
    );
    ensure!(
        evidence.bases.len() == plan.pieces.len(),
        "bilinear general piece inventory changed"
    );
    let mut stacks = 0;
    let mut rejected = 0;
    for (i, base) in evidence.bases.iter().enumerate() {
        let expected = piece_problem(&result.problem, &replayed, plan, i)?;
        ensure!(
            same_piece_problem(&expected, &base.problem)?,
            "bilinear general piece {i} problem or options changed"
        );
        ensure!(
            base.bilinear_general.is_none()
                && base.bilinear_positive.is_none()
                && base.linear_negative.is_none()
                && base.quadratic_ladder.is_none()
                && base.monotone.is_none()
                && base.sign_filter.is_none(),
            "unexpected nested fiber reduction in bilinear general piece {i}"
        );
        ensure!(
            base.status == Status::CompleteGeneric,
            "bilinear general piece {i} is incomplete"
        );
        let checked = crate::verify::verify(base)?;
        stacks += checked.stacks;
        rejected += checked.rejected_branches;
    }
    let mut index = 0;
    let layout = reconstruct(&replayed, plan, &evidence.bases, |cell| {
        ensure!(
            result.cells.get(index) == Some(&cell),
            "bilinear general fiber restoration changed at cell {index}"
        );
        index += 1;
        Ok(true)
    })?;
    ensure!(
        index == result.cells.len() && result.statistics.raw_cells == index,
        "bilinear general cell inventory changed"
    );
    ensure!(
        result
            .problem
            .limits
            .max_cells
            .is_none_or(|cap| index <= cap),
        "complete bilinear general result exceeds cell cap"
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
        "bilinear general original metadata changed"
    );
    ensure!(
        result.polynomials == layout.table
            && result.projection_levels == layout.levels
            && result.exceptional_polynomials == layout.exceptional,
        "bilinear general polynomial table or exceptional set changed"
    );
    ensure!(
        result.merged_cells
            == (result.problem.solver.merge == "aggressive").then(|| merge(&result.cells)),
        "bilinear general compatibility merge changed"
    );
    Ok(Verification {
        verified: true,
        cells: index,
        stacks,
        rejected_branches: rejected,
    })
}

#[cfg(test)]
mod table_tests {
    use super::*;

    fn member(rows: &[FiberRow], k: [Rational; 4], y: &Rational, x: &Rational) -> Option<bool> {
        let [a, b, c, d] = k;
        let value = |e: YEnd| match e {
            YEnd::Zero => Some(Rational::zero()),
            YEnd::RootA => Some(-b.clone() / &a),
            YEnd::RootB => Some(-d.clone() / &c),
            YEnd::Infinity => None,
        };
        for row in rows {
            let lo = value(row.lower).unwrap();
            let hi = value(row.upper);
            if y == &lo || hi.as_ref() == Some(y) {
                return None;
            }
            if *y > lo && hi.as_ref().is_none_or(|h| y < h) {
                let slope = &a * y + &b;
                let intercept = &c * y + &d;
                let root = -intercept / slope;
                if x == &root {
                    return None;
                }
                return Some(match row.x {
                    XFiber::Full => true,
                    XFiber::Below => x < &root,
                    XFiber::Above => x > &root,
                });
            }
        }
        Some(false)
    }

    #[test]
    fn all_sixteen_patterns_match_exact_membership() {
        let values = [1i64, 2, 3, 5, 7];
        let mut checked = 0;
        for mask in 0..16u32 {
            let s: [i8; 4] = std::array::from_fn(|i| if mask >> i & 1 == 1 { 1 } else { -1 });
            for (i, j, k, l) in [
                (0, 1, 2, 3),
                (4, 0, 3, 1),
                (2, 2, 1, 4),
                (1, 4, 4, 0),
                (3, 1, 0, 2),
            ] {
                let k: [Rational; 4] =
                    std::array::from_fn(|t| Rational::from(s[t] as i64 * values[[i, j, k, l][t]]));
                let delta = &k[1] * &k[2] - &k[0] * &k[3];
                if delta.is_zero() {
                    continue;
                }
                let rows = fiber_rows(s, sign(&delta)).unwrap();
                assert!(rows.len() <= 3);
                for yn in 1..40i64 {
                    for xn in 1..40i64 {
                        let y = Rational::from(yn) / Rational::from(7);
                        let x = Rational::from(xn) / Rational::from(5);
                        let f = &k[0] * &x * &y + &k[1] * &x + &k[2] * &y + &k[3];
                        if f.is_zero() {
                            continue;
                        }
                        if let Some(inside) = member(&rows, k.clone(), &y, &x) {
                            assert_eq!(inside, f > Rational::zero(), "{s:?} {k:?} {y} {x}");
                            checked += 1;
                        }
                    }
                }
            }
        }
        assert!(checked > 100_000);
        assert!(fiber_rows([-1, 1, -1, 1], 0).is_err());
        assert!(fiber_rows([0, 1, 1, 1], 1).is_err());
    }

    #[test]
    fn closure_and_planner_reproduce_the_monotone_corner_chain() {
        // a<=b<=d and a<=c<=d with Delta a positive square: the four pieces
        // governed by sign(b), sign(c).
        let logic = Logic {
            global: [0; 5],
            relations: vec![(0, 1), (0, 2), (1, 3), (2, 3)],
            alias: [None; 5],
            separator_sign: 1,
        };
        let (pieces, empty) = plan_pieces(&logic, Objective::RegionWeighted);
        let c = |atom, sign| Condition { atom, sign };
        assert_eq!(
            pieces,
            vec![
                PieceSpec {
                    conditions: vec![c(Atom::B, 1), c(Atom::C, 1)],
                    split: vec![Atom::A]
                },
                PieceSpec {
                    conditions: vec![c(Atom::B, 1), c(Atom::C, -1)],
                    split: vec![Atom::G]
                },
                PieceSpec {
                    conditions: vec![c(Atom::B, -1), c(Atom::C, 1)],
                    split: vec![]
                },
                PieceSpec {
                    conditions: vec![c(Atom::B, -1), c(Atom::C, -1), c(Atom::D, 1)],
                    split: vec![]
                },
            ]
        );
        assert_eq!(
            empty,
            vec![EmptyClaim {
                conditions: vec![c(Atom::B, -1), c(Atom::C, -1), c(Atom::D, -1)]
            }]
        );
        assert_eq!(logic.close([1, 0, 0, 0, 0]), Some([1, 1, 1, 1, 0]));
        assert_eq!(logic.close([0, 0, 0, -1, 0]), Some([-1, -1, -1, -1, 0]));
        assert_eq!(logic.close([1, 0, 0, -1, 0]), None);
        assert_eq!(
            plan_cost(&logic, &pieces),
            (28, 11),
            "region-weighted and plain costs of the four-piece plan"
        );

        // The plain atom count merges {b<0,c>0} and {b<0,c<0,d>0} into
        // {b<0,d>0} split c: three pieces of plain cost 9 (weighted 30).
        let (atoms, atoms_empty) = plan_pieces(&logic, Objective::Atoms);
        assert_eq!(
            atoms,
            vec![
                PieceSpec {
                    conditions: vec![c(Atom::B, 1), c(Atom::C, 1)],
                    split: vec![Atom::A]
                },
                PieceSpec {
                    conditions: vec![c(Atom::B, 1), c(Atom::C, -1)],
                    split: vec![Atom::G]
                },
                PieceSpec {
                    conditions: vec![c(Atom::B, -1), c(Atom::D, 1)],
                    split: vec![Atom::C]
                },
            ]
        );
        assert_eq!(plan_cost(&logic, &atoms), (30, 9));
        assert_eq!(
            atoms_empty,
            vec![EmptyClaim {
                conditions: vec![c(Atom::B, -1), c(Atom::D, -1)]
            }]
        );
        // Both plans pass the verifier's independent partition check.
        let atoms_polys: [Poly; 5] = {
            let algebra = crate::algebra::Algebra::new(&["t".into()]).unwrap();
            std::array::from_fn(|_| algebra.parse("t").unwrap())
        };
        for (pieces, empty) in [(pieces, empty), (atoms, atoms_empty)] {
            let plan = BilinearGeneralPlan {
                schema_version: 1,
                target_constraint: 0,
                order: vec![],
                coefficients: vec![],
                guide: None,
                guide_positivity: vec![],
                premises: vec![],
                separator: Separator::Delta {
                    polynomial: "t".into(),
                },
                pieces,
                empty,
            };
            check_partition(&logic, &atoms_polys, &plan).unwrap();
        }
    }

    #[test]
    fn over_cap_ratio_pairs_are_skipped_not_fatal() {
        let algebra = crate::algebra::Algebra::new(&["t".into()]).unwrap();
        let positives = vec![algebra.parse("t").unwrap()];
        let domain = derive_domain(&algebra, &positives, &[]).unwrap();
        let options = CertificateOptions::default();
        let a = algebra.parse("t-3").unwrap();
        let b = algebra.parse("t-1").unwrap();
        let one = algebra.parse("1").unwrap();
        let pair = [(&a, &one), (&b, &one)];
        let ok = || -> Result<()> { Ok(()) };
        assert!(
            ratio_premise(&algebra, &domain, &options, (0, 1), pair, 1, ok)
                .unwrap()
                .is_none()
        );
        // b*1 - a*1 = 2 = 2*1^2: a lies below b.
        let premise = ratio_premise(&algebra, &domain, &options, (0, 1), pair, PRODUCT_CAP, ok)
            .unwrap()
            .unwrap();
        assert!(matches!(
            premise,
            SignPremise::RatioOrder {
                lower: Atom::A,
                upper: Atom::B,
                strict: None,
                ..
            }
        ));
        // A time check failure still stops discovery.
        let late = || -> Result<()> { bail!("stopped") };
        let c = algebra.parse("t^2+1").unwrap();
        assert!(
            ratio_premise(
                &algebra,
                &domain,
                &options,
                (0, 1),
                [(&a, &one), (&c, &one)],
                PRODUCT_CAP,
                late
            )
            .is_err()
        );
    }

    #[test]
    fn zero_coefficient_aliases_match_exact_membership() {
        // b, c or d identically zero (alone, or b and c together): the fiber
        // table with the aliased partner sign is exact on the open quadrant.
        let mut checked = 0;
        for zero_mask in [0b0010u32, 0b0100, 0b1000, 0b0110] {
            for mask in 0..16u32 {
                let s: [i64; 4] = std::array::from_fn(|i| if mask >> i & 1 == 1 { 1 } else { -1 });
                let magnitudes = [2i64, 3, 5, 7];
                let k: [Rational; 4] = std::array::from_fn(|i| {
                    if zero_mask >> i & 1 == 1 {
                        Rational::zero()
                    } else {
                        Rational::from(s[i] * magnitudes[i])
                    }
                });
                let polys: [Poly; 4] = {
                    let algebra = crate::algebra::Algebra::new(&["t".into()]).unwrap();
                    std::array::from_fn(|i| algebra.parse(&k[i].to_string()).unwrap())
                };
                let alias = zero_aliases(&polys);
                let mut facts: Facts = [0; 5];
                for i in 0..4 {
                    if alias[i].is_none() {
                        facts[i] = sign(&k[i]);
                    }
                }
                let logic = Logic {
                    global: [0; 5],
                    relations: vec![],
                    alias,
                    separator_sign: 0,
                };
                let f = logic.close(facts).unwrap();
                assert!(decided(&f), "{zero_mask:b} {k:?}");
                assert!(!both_roots(&f));
                let rows = fiber_rows([f[0], f[1], f[2], f[3]], 0).unwrap();
                assert_eq!(rows.is_empty(), all_negative(&f));
                for yn in 1..25i64 {
                    for xn in 1..25i64 {
                        let y = Rational::from(yn) / Rational::from(7);
                        let x = Rational::from(xn) / Rational::from(5);
                        let value = &k[0] * &x * &y + &k[1] * &x + &k[2] * &y + &k[3];
                        if value.is_zero() {
                            continue;
                        }
                        if let Some(inside) = member(&rows, k.clone(), &y, &x) {
                            assert_eq!(inside, value > Rational::zero(), "{k:?} {y} {x}");
                            checked += 1;
                        }
                    }
                }
            }
        }
        assert!(checked > 30_000);
        // A zero coefficient never receives a fact that contradicts its partner.
        let logic = Logic {
            global: [0; 5],
            relations: vec![],
            alias: [None, Some(0), None, None, None],
            separator_sign: 0,
        };
        assert_eq!(logic.close([1, -1, 0, 0, 0]), None);
        assert!(!logic.consistent(&[1, -1, 1, 1, 1]));
        assert!(logic.consistent(&[-1, -1, 1, 1, 1]));
    }
}
