//! Certified root unions and exact rational open-sector samples.
use anyhow::{Result, ensure};
use symbolica::prelude::*;

#[derive(Clone, Debug)]
pub struct FiberRoot {
    pub polynomial: usize,
    pub index: usize,
    pub interval: (Rational, Rational),
    pub fiber: UnivariatePolynomial<Q>,
}

/// Sign-variation counts use polynomial arithmetic from Symbolica. Endpoints
/// that are roots are handled by one-sided limits, so adjoining roots are not
/// accidentally counted in an open interval.
fn sign_at(poly: &UnivariatePolynomial<Q>, x: &Rational, right: bool) -> i8 {
    let mut p = poly.clone();
    let mut parity = false;
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
    let mut sequence = vec![poly.clone(), poly.derivative()];
    loop {
        let n = sequence.len();
        let remainder = sequence[n - 2].rem(&sequence[n - 1]);
        if remainder.is_zero() {
            break;
        }
        sequence.push(-remainder);
    }
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
    let mut roots = Vec::new();
    for (id, polynomial) in polynomials {
        ensure!(
            !polynomial.is_zero(),
            "projection polynomial {id} nullified at a supposedly generic sample"
        );
        if polynomial.degree() == 0 {
            continue;
        }
        // Make square-free for sign-based bisection, keeping the same real roots.
        let gcd = polynomial.gcd(&polynomial.derivative());
        let (fiber, remainder) = polynomial.quot_rem(&gcd);
        ensure!(remainder.is_zero(), "square-free division failed");
        let mut intervals = fiber.isolate_real_root_intervals();
        intervals.sort_by(|a, b| a.0.cmp(&b.0));
        for (index, (lo, hi, _)) in intervals.into_iter().enumerate() {
            roots.push(FiberRoot {
                polynomial: id,
                index,
                interval: (lo, hi),
                fiber: fiber.clone(),
            });
        }
    }
    // Every collision is either certified as a common root, or separated by
    // exact bisection. Close but unequal roots are never numerically collapsed.
    loop {
        roots.sort_by(|a, b| {
            a.interval
                .0
                .cmp(&b.interval.0)
                .then(a.polynomial.cmp(&b.polynomial))
        });
        let collision = roots
            .windows(2)
            .position(|w| w[0].interval.1 >= w[1].interval.0);
        let Some(i) = collision else {
            break;
        };
        if same_root(&roots[i], &roots[i + 1]) {
            if roots[i + 1].polynomial < roots[i].polynomial {
                roots.swap(i, i + 1);
            }
            roots.remove(i + 1);
        } else {
            refine_once(&mut roots[i]);
            refine_once(&mut roots[i + 1]);
        }
    }
    Ok(roots)
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
