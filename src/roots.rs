//! Certified root unions and exact rational open-sector samples.
use anyhow::{Result, ensure};
use std::{cmp::Ordering, collections::BinaryHeap};
use symbolica::prelude::*;

/// Symbolica's certified real isolator currently converts to u16 exponents.
pub const MAX_ROOT_DEGREE: usize = u16::MAX as usize;

#[derive(Clone, Debug)]
pub struct FiberRoot {
    pub polynomial: usize,
    pub index: usize,
    pub interval: (Rational, Rational),
    pub fiber: UnivariatePolynomial<Q>,
}

/// Reversed interval order makes BinaryHeap expose the lowest possible root.
/// Refinement only shrinks intervals, so a root wholly left of the next lower
/// endpoint is also wholly left of every other pending root.
struct PendingRoot(FiberRoot);
impl PartialEq for PendingRoot {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for PendingRoot {}
impl PartialOrd for PendingRoot {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for PendingRoot {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .0
            .interval
            .0
            .cmp(&self.0.interval.0)
            .then(other.0.polynomial.cmp(&self.0.polynomial))
            .then(other.0.index.cmp(&self.0.index))
    }
}

/// Sign-variation counts use polynomial arithmetic from Symbolica. Endpoints
/// that are roots are handled by one-sided limits, so adjoining roots are not
/// accidentally counted in an open interval.
fn sign_at(poly: &UnivariatePolynomial<Q>, x: &Rational, right: bool) -> i8 {
    let value = poly.evaluate(x);
    if !value.is_zero() {
        return if value.is_negative() { -1 } else { 1 };
    }
    if poly.is_zero() {
        return 0;
    }
    // Derivatives are needed only at a zero endpoint. Ordinary bisection and
    // Sturm evaluations can borrow the fiber without cloning its coefficients.
    let mut p = poly.derivative();
    let mut parity = true;
    loop {
        let value = p.evaluate(x);
        if !value.is_zero() {
            let sign = if value.is_negative() { -1 } else { 1 };
            return if !right && parity { -sign } else { sign };
        }
        if p.is_zero() {
            return 0;
        }
        p = p.derivative();
        parity = !parity;
    }
}

fn variations(sequence: &[UnivariatePolynomial<Q>], x: &Rational, right: bool) -> usize {
    let mut previous = 0;
    let mut count = 0;
    for p in sequence {
        let sign = sign_at(p, x, right);
        if sign != 0 {
            if previous != 0 && sign != previous {
                count += 1;
            }
            previous = sign;
        }
    }
    count
}

pub fn roots_in_open_interval(
    poly: &UnivariatePolynomial<Q>,
    lo: &Rational,
    hi: &Rational,
) -> usize {
    if lo >= hi || poly.is_zero() || poly.degree() == 0 {
        return 0;
    }
    // Remove rational content by a positive scale, then form an integer
    // primitive pseudo-remainder sequence using Symbolica throughout. Ordinary
    // rational division can generate much larger intermediate denominators.
    let content = poly.content().abs();
    let first = poly.clone().div_coeff(&content);
    let first = first.map_coeff(
        |c| {
            assert!(
                c.is_integer(),
                "rational content did not clear denominators"
            );
            c.numerator()
        },
        Z,
    );
    let primitive = |p: UnivariatePolynomial<Z>| {
        let content = p.content().abs();
        p.div_coeff(&content)
    };
    let second = primitive(first.derivative());
    let mut sequence = vec![first, second];
    loop {
        let n = sequence.len();
        // Symbolica multiplies by the divisor's leading coefficient once
        // per actual elimination, which may skip degrees. Making that LC
        // positive ensures a positive multiple of the field remainder without
        // assuming a fixed exponent or changing the stored Sturm signs.
        let divisor = &sequence[n - 1];
        let positive_divisor;
        let divisor = if divisor.lcoeff().is_negative() {
            positive_divisor = -divisor.clone();
            &positive_divisor
        } else {
            divisor
        };
        let remainder = sequence[n - 2].pseudo_remainder(divisor);
        if remainder.is_zero() {
            break;
        }
        sequence.push(primitive(-remainder));
    }
    let sequence: Vec<_> = sequence
        .iter()
        .map(|p| p.map_coeff(|c| Rational::from(c.clone()), Q))
        .collect();
    variations(&sequence, lo, true) - variations(&sequence, hi, false)
}

pub fn refine_once(root: &mut FiberRoot) {
    let (lo, hi) = &root.interval;
    if lo == hi {
        return;
    }
    let mid = (lo + hi) / Rational::from(2);
    let value = root.fiber.evaluate(&mid);
    if value.is_zero() {
        root.interval = (mid.clone(), mid);
    } else if sign_at(&root.fiber, lo, true) == if value.is_negative() { -1 } else { 1 } {
        root.interval.0 = mid;
    } else {
        root.interval.1 = mid;
    }
}

fn same_root(a: &FiberRoot, b: &FiberRoot) -> bool {
    let lo = a.interval.0.clone().max(b.interval.0.clone());
    let hi = a.interval.1.clone().min(b.interval.1.clone());
    if lo > hi {
        return false;
    }
    if lo == hi {
        return a.fiber.evaluate(&lo).is_zero()
            && b.fiber.evaluate(&lo).is_zero()
            && (a.interval.0 == a.interval.1 || (a.interval.0 < lo && lo < a.interval.1))
            && (b.interval.0 == b.interval.1 || (b.interval.0 < lo && lo < b.interval.1));
    }
    let gcd = a.fiber.gcd(&b.fiber);
    roots_in_open_interval(&gcd, &lo, &hi) != 0
}

pub fn isolate_union(polynomials: Vec<(usize, UnivariatePolynomial<Q>)>) -> Result<Vec<FiberRoot>> {
    isolate_union_observed(polynomials, |_, _, _, _| Ok(()))
}

/// Read-only progress for diagnosing an expensive exact isolation. The observer
/// cannot change the root computation. It runs before each individual isolation
/// and collision check; callers should throttle any filesystem output.
pub fn isolate_union_observed(
    polynomials: Vec<(usize, UnivariatePolynomial<Q>)>,
    mut observer: impl FnMut(&str, usize, &UnivariatePolynomial<Q>, usize) -> Result<()>,
) -> Result<Vec<FiberRoot>> {
    let mut roots = Vec::new();
    for (id, polynomial) in polynomials {
        ensure!(
            !polynomial.is_zero(),
            "projection polynomial {id} nullified at a supposedly generic sample"
        );
        if polynomial.degree() == 0 {
            continue;
        }
        observer("isolate_polynomial", id, &polynomial, roots.len())?;
        if polynomial.degree() == 1 {
            // Exact linear isolation needs neither a gcd nor VAS factorization.
            let root = -polynomial.coefficients()[0].clone() / polynomial.coefficients()[1].clone();
            roots.push(FiberRoot {
                polynomial: id,
                index: 0,
                interval: (root.clone(), root),
                fiber: polynomial,
            });
            continue;
        }
        ensure!(
            polynomial.degree() <= MAX_ROOT_DEGREE,
            "univariate degree exceeds the certified isolator's u16 exponent range"
        );
        // The certified isolator already computes square-free factors and
        // reports multiplicities. Avoid repeating its gcd on generic fibers.
        // Sign bisection only requires the *real* roots to be simple; repeated
        // nonreal factors never vanish on the real line and are harmless.
        let mut intervals = polynomial.isolate_real_root_intervals();
        let fiber = if intervals
            .iter()
            .any(|(_, _, multiplicity)| *multiplicity != 1)
        {
            let gcd = polynomial.gcd(&polynomial.derivative());
            let (fiber, remainder) = polynomial.quot_rem(&gcd);
            ensure!(remainder.is_zero(), "square-free division failed");
            fiber
        } else {
            polynomial
        };
        intervals.sort_by(|a, b| a.0.cmp(&b.0));
        for (index, (lo, hi, _)) in intervals.into_iter().enumerate() {
            let interval = open_or_exact_interval(&fiber, index, lo, hi)?;
            roots.push(FiberRoot {
                polynomial: id,
                index,
                interval,
                fiber: fiber.clone(),
            });
        }
    }
    // Every collision is either certified as a common root, or separated by
    // exact bisection. Each changed interval is reinserted in O(log n) order,
    // avoiding a full-list sort and rescan on every bisection. Close but unequal
    // roots are never numerically collapsed.
    let mut pending: BinaryHeap<_> = roots.drain(..).map(PendingRoot).collect();
    while let Some(PendingRoot(mut first)) = pending.pop() {
        let Some(next) = pending.peek() else {
            roots.push(first);
            break;
        };
        if first.interval.1 < next.0.interval.0 {
            roots.push(first);
            continue;
        }
        let PendingRoot(mut second) = pending.pop().expect("peeked root");
        observer(
            "separate_roots",
            first.polynomial,
            &first.fiber,
            roots.len() + pending.len() + 2,
        )?;
        if same_root(&first, &second) {
            if second.polynomial < first.polynomial {
                std::mem::swap(&mut first, &mut second);
            }
            pending.push(PendingRoot(first));
        } else {
            refine_once(&mut first);
            refine_once(&mut second);
            pending.push(PendingRoot(first));
            pending.push(PendingRoot(second));
        }
    }
    Ok(roots)
}

/// Symbolica can return a nondegenerate interval whose target rational root
/// is an endpoint (e.g. 4*x^2-8*x gives (1,2)). Our exported contract requires
/// either one root strictly inside or an exact point. An endpoint can also be
/// a *different* root, so collapse only after exact open root counting and an
/// ordinal check; blindly replacing any zero endpoint would merge roots.
fn open_or_exact_interval(
    fiber: &UnivariatePolynomial<Q>,
    index: usize,
    lo: Rational,
    hi: Rational,
) -> Result<(Rational, Rational)> {
    if lo == hi {
        return Ok((lo, hi));
    }
    let lower_root = fiber.evaluate(&lo).is_zero();
    let upper_root = fiber.evaluate(&hi).is_zero();
    if !lower_root && !upper_root {
        return Ok((lo, hi));
    }
    let inside = roots_in_open_interval(fiber, &lo, &hi);
    if inside == 1 {
        return Ok((lo, hi));
    }
    ensure!(
        inside == 0,
        "backend interval contains multiple distinct roots"
    );
    let lc = fiber.coefficients().last().unwrap().abs();
    let cauchy = fiber
        .coefficients()
        .iter()
        .take(fiber.degree())
        .map(|c| c.abs() / &lc)
        .max()
        .unwrap_or_else(Rational::zero)
        + Rational::from(2);
    let left = -cauchy;
    for (endpoint, is_root) in [(&lo, lower_root), (&hi, upper_root)] {
        if is_root && roots_in_open_interval(fiber, &left, endpoint) == index {
            return Ok((endpoint.clone(), endpoint.clone()));
        }
    }
    anyhow::bail!("backend endpoint root does not have the claimed real-root index {index}")
}

/// A small-denominator rational in a closed rational interval. Continued
/// fractions use Symbolica's exact integer/rational arithmetic.
pub fn simplest_between(lo: &Rational, hi: &Rational) -> Rational {
    assert!(lo <= hi);
    if hi.is_negative() {
        return -simplest_between(&(-hi.clone()), &(-lo.clone()));
    }
    if lo <= &Rational::zero() {
        return Rational::zero();
    }
    let mut a = lo.clone();
    let mut b = hi.clone();
    let mut quotients = Vec::new();
    let mut answer;
    loop {
        let ceiling = Rational::from(&a.ceil());
        if ceiling <= b {
            answer = ceiling;
            break;
        }
        let c = ceiling - Rational::one();
        let next_a = (&b - &c).inv();
        let next_b = (&a - &c).inv();
        quotients.push(c);
        a = next_a;
        b = next_b;
    }
    for c in quotients.into_iter().rev() {
        answer = c + answer.inv();
    }
    answer
}

pub fn sector_sample(lo: Option<&FiberRoot>, hi: Option<&FiberRoot>) -> Rational {
    match (lo, hi) {
        (None, None) => Rational::zero(),
        (None, Some(b)) => Rational::from(&b.interval.0.floor()) - Rational::one(),
        (Some(a), None) => Rational::from(&a.interval.1.ceil()) + Rational::one(),
        (Some(a), Some(b)) => {
            let gap = &b.interval.0 - &a.interval.1;
            let adjustment = &gap / &Rational::from(1024);
            let lower = if a.interval.0 == a.interval.1 {
                &a.interval.1 + &adjustment
            } else {
                a.interval.1.clone()
            };
            let upper = if b.interval.0 == b.interval.1 {
                &b.interval.0 - &adjustment
            } else {
                b.interval.0.clone()
            };
            simplest_between(&lower, &upper)
        }
    }
}
