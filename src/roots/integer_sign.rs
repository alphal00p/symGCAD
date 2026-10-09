//! Exact signs of integer-coefficient polynomials at rational points.
//!
//! For x=n/d with d>0, homogeneous Horner evaluation computes d^m*p(n/d)
//! using only Symbolica integers. Its sign is p(x)'s sign, with no rational
//! normalization between terms. Only the current accumulator and denominator
//! power are retained; rational-coefficient inputs use ordinary evaluation.
use symbolica::prelude::*;

#[inline]
fn integer_sign(value: &Integer) -> i8 {
    if value.is_zero() {
        0
    } else if value.is_negative() {
        -1
    } else {
        1
    }
}

pub(super) fn sign(poly: &UnivariatePolynomial<Q>, x: &Rational) -> i8 {
    let coefficients = poly.coefficients();
    let Some(leading) = coefficients.last() else {
        return 0;
    };
    if coefficients
        .iter()
        .any(|coefficient| !coefficient.is_integer())
    {
        return integer_sign(poly.evaluate(x).numerator_ref());
    }
    if coefficients.len() == 1 {
        return integer_sign(leading.numerator_ref());
    }
    let numerator = x.numerator_ref();
    if numerator.is_zero() {
        return integer_sign(coefficients[0].numerator_ref());
    }
    let denominator = x.denominator_ref();
    // Symbolica Rational maintains a positive denominator. That positivity is
    // the reason clearing d^degree preserves the sign, also for negative x.
    debug_assert!(!denominator.is_negative() && !denominator.is_zero());
    let mut accumulated = leading.numerator_ref().clone();
    if denominator.is_one() {
        for coefficient in coefficients[..coefficients.len() - 1].iter().rev() {
            Z.mul_assign(&mut accumulated, numerator);
            Z.add_assign(&mut accumulated, coefficient.numerator_ref());
        }
    } else {
        let mut denominator_power = denominator.clone();
        for index in (0..coefficients.len() - 1).rev() {
            Z.mul_assign(&mut accumulated, numerator);
            Z.add_mul_assign(
                &mut accumulated,
                coefficients[index].numerator_ref(),
                &denominator_power,
            );
            if index != 0 {
                Z.mul_assign(&mut denominator_power, denominator);
            }
        }
    }
    integer_sign(&accumulated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::{Algebra, rational};

    fn prototype() -> UnivariatePolynomial<Q> {
        Algebra::new(&["x".into()])
            .unwrap()
            .parse("x")
            .unwrap()
            .to_univariate_from_univariate(0)
    }

    fn expected(poly: &UnivariatePolynomial<Q>, x: &Rational) -> i8 {
        let value = poly.evaluate(x);
        if value.is_zero() {
            0
        } else if value.is_negative() {
            -1
        } else {
            1
        }
    }

    fn compare(poly: &UnivariatePolynomial<Q>, points: &[Rational]) {
        for x in points {
            assert_eq!(sign(poly, x), expected(poly, x), "{poly} at {x}");
        }
    }

    #[test]
    fn zero_constants_and_trailing_zero_coefficients() {
        let prototype = prototype();
        let points: Vec<_> = ["0", "-7", "9/13", "-999999999999999999999/17"]
            .map(|s| rational(s).unwrap())
            .into();
        for values in [vec![], vec![0], vec![7], vec![-19], vec![2, -1, 0, 0]] {
            let p = UnivariatePolynomial::from_coefficients(
                &Q,
                values.into_iter().map(Rational::from).collect(),
                prototype.get_vars(),
            );
            compare(&p, &points);
        }
        compare(&prototype.constant(rational("-7/13").unwrap()), &points);
    }

    #[test]
    fn deterministic_integer_polynomials_and_rational_samples() {
        let prototype = prototype();
        let mut state = 0x5347_4341_4448_4f52u64;
        let mut next = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            state
        };
        for case in 0..96 {
            let degree = case % 33;
            let coefficients = (0..=degree)
                .map(|_| Rational::from((next() % 2_000_001) as i64 - 1_000_000))
                .collect();
            let p = UnivariatePolynomial::from_coefficients(&Q, coefficients, prototype.get_vars());
            let mut points = vec![Rational::zero(), Rational::one(), Rational::from(-1)];
            for _ in 0..5 {
                let numerator = (next() % 4097) as i64 - 2048;
                let denominator = (next() % 1024 + 1) as i64;
                points.push(Rational::new(numerator, denominator));
            }
            compare(&p, &points);
        }
    }

    #[test]
    fn rational_coefficients_retain_the_general_evaluator() {
        let prototype = prototype();
        for degree in 1..=16 {
            let coefficients = (0..=degree)
                .map(|i| Rational::new(3 * i as i64 - 17, i as i64 + 2))
                .collect();
            let p = UnivariatePolynomial::from_coefficients(&Q, coefficients, prototype.get_vars());
            compare(
                &p,
                &["0", "-3/11", "7/5", "-2", "10000000000000000000001/3"]
                    .map(|s| rational(s).unwrap()),
            );
        }
    }

    #[test]
    fn exact_zeros_near_roots_large_coefficients_and_negative_points() {
        let a = Algebra::new(&["x".into()]).unwrap();
        let scale = Integer::from(10).pow(80);
        let centre = Rational::new(Integer::one(), scale.clone());
        let delta = Rational::new(Integer::one(), &scale * &scale);
        let points = [
            &centre - &delta,
            centre.clone(),
            &centre + &delta,
            -centre.clone(),
            Rational::from(-2),
            Rational::from(2),
            Rational::new(-scale.clone(), Integer::from(3)),
        ];
        for expression in [
            format!("{scale}*x-1"),
            format!("({scale}*x-1)^2*(x+2)"),
            format!("-({scale}*x-1)^3*(x+2)^2"),
            format!("{scale}*x^32-{scale}*x^2+x-1"),
        ] {
            let p = a
                .parse(&expression)
                .unwrap()
                .to_univariate_from_univariate(0);
            compare(&p, &points);
        }
    }

    #[test]
    fn captured_a381_degree24_integer_fiber_matches_symbolica() {
        // Polynomial1120 from the first-axis root union. Original capture SHA:
        // a75a19940cf6a7c7e009cc4de9402a761eb8373fb666d8581df6645d39b35929.
        // Keep the exact coefficients in source so tests need no local artifacts.
        let coefficients = [
            "1106705900647799669565344790186711540392472352031356012488465606717000000000000",
            "-3645601751931941752796309049456337878958334010091167337235656891307040000000000",
            "-549351685044706917745664331758091695076381492155131559748978145405095313920000000",
            "3228903255377745972506212969358331129422332880015610531287353126518650524032000000",
            "-22789403470900029214500092810310032736969787862753242800895714598793822900908480000",
            "-242240647345579125807935506537432547678308759184864886641433075840599148358875008000",
            "2456333099130564623328236767402104133134078956036706999404633056836120534510029644800",
            "-5214812381431732912021058940932675782783948278565825057981348083539108078731765849600",
            "-1055582158598077482207481851188727224308534934462315415773741462309058874620977292080",
            "7794048957979318042575399705132570613342753507728638522448157330705999444224461002880",
            "1430971131038287399853164493665375684767884795042664690598744707128684624494655382208",
            "-4253817851477133405074220715291343533474667409377211518960162470338302128934659980160",
            "-1698875995986432136661650650108169360246782381845436146575739049462706111625411140640",
            "711393260315242495473314888354607936194762044500587238639360628656560853795067256000",
            "554963706494540476413056125843988263130683445978091280771243478817484247796567385600",
            "47134544820573887084424864544778867170467954917536706439863654681023318488367736000",
            "-53296369226231415163846275464775527931792533898859962949230637293871591826315329000",
            "-19050327497246181655998389472308600915485808921542503327276961553868431143782980000",
            "-1280101154765242759746078302564148201344357361610031756232603128059557091487090000",
            "725315517966327952049762188309202323708473378631253562139979948724110098279300000",
            "238395372936170440216789036395649139654292145827529278219130535115746990788425000",
            "35567830077828531658755933789648315186690096239329061951473304858588332001250000",
            "3002140365138065908211215905979108082029502921755756694535366742473355116250000",
            "138535054609677229352549404196815126203851593431496594198929879302017156250000",
            "2721712147910433472227912211572052121464887418879073661860909458139533203125",
        ];
        let p = UnivariatePolynomial::from_coefficients(
            &Q,
            coefficients.iter().map(|c| rational(c).unwrap()).collect(),
            prototype().get_vars(),
        );
        assert_eq!(p.degree(), 24);
        let mut points: Vec<_> = (-64..=64).map(|n| Rational::new(n, 17)).collect();
        points.extend(
            [
                "1/1000000000000000000000000000000",
                "-1/1000000000000000000000000000000",
                "1000000000000000000000000000001/1000000000000000000000000000000",
            ]
            .map(|x| rational(x).unwrap()),
        );
        compare(&p, &points);
    }
}
