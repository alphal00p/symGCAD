//! Explicit resource regression, separate from the short default suite.
use symgcad::{algebra::Algebra, projection::discriminant_with_binary_statistics};

#[test]
#[ignore = "large exact encoding; run explicitly under a bounded worker"]
fn larger_encoding_preserves_full_signed_rational_discriminant() {
    let a = Algebra::new(&["unused".into(), "s".into(), "x".into()]).unwrap();
    let p = a.parse("-2/3*(x^10+2^3500*(s+1)^500)").unwrap();
    // Disc(c*(x^10+b)) = -10^10*c^18*b^9, including rational scale.
    let expected = a.parse("-10^10*(-2/3)^18*2^31500*(s+1)^4500").unwrap();
    let (actual, statistics) = discriminant_with_binary_statistics(&p, 2).unwrap();
    let statistics = statistics.expect("the larger bounded encoding must be used");
    assert!(statistics.plan.output_bit_bound > 96_000_000);
    assert!(statistics.plan.output_bit_bound <= 256_000_000);
    assert_eq!(actual, expected);
    assert_eq!(actual.variables(), p.variables());
    assert!(actual.replace(1, &(-1).into()).is_zero());
}
