//! Small exact identity checks for the unrestricted-kinematics A446 audit.
use anyhow::{Result, ensure};
use symbolica::prelude::*;
use symgcad::{
    Problem,
    algebra::{Algebra, rational},
    problem::normalize,
};

fn main() -> Result<()> {
    let problem: Problem = toml::from_str(&std::fs::read_to_string(
        "artifacts/corpus/symbolic/0093_finiteA_7_446_11_0__0_5_1_1_1_1_0_1_1_symbolic.toml",
    )?)?;
    let original = normalize(&problem, &problem.variables)?;
    let mut names = problem.variables.clone();
    names.extend(["a".to_string(), "b".to_string()]);
    let algebra = Algebra::new(&names)?;
    // t now denotes the crossed invariant u=5/9-s-t_A, equal to t_Ax.
    let f = algebra
        .parse(&original.algebra.format(&original.split[0]))?
        .replace_with_poly(1, &algebra.parse("5/9-s-t")?)
        .div_coeff(&rational("18")?);
    let grouped = algebra.parse(
        "(x1+x2+x3+x4+x5+x7)*((x1+x2+x3)*(x4+x5+x7)+(x1+x2+x3+x4+x5+x7)*x8)
        -s*x2*x4*x8
        -t*((x4+x5+x7+x8)*x1*x3+(x1+x2+x3+x8)*x5*x7+x8*(x1*x7+x3*x5))
        -(5/18)*((x4+x5+x7+x8)*x2*x3+(x1+x2+x3+x8)*x4*x7+x8*(x2*x7+x3*x4))",
    )?;
    ensure!(f == grouped, "grouped Symanzik identity failed");
    println!("Grouped F=M-sS-uT-(5/18)Z identity: exact");

    let transformed = f
        .replace_with_poly(2, &algebra.parse("a-x2-x3")?)
        .replace_with_poly(6, &algebra.parse("b-x4-x7")?);
    let w1 = algebra.parse("2*t*x3-t*(a-x2)-(5/18)*x2")?;
    let w2 = algebra.parse("2*t*x7-t*(b-x4)-(5/18)*x4")?;
    let d = algebra.parse(
        "(a*b+x8*(a+b))*(t*(4-t)*(a+b)+2*t*(t-5/18)*(x2+x4))
        -(t-5/18)^2*((b+x8)*x2^2+2*x8*x2*x4+(a+x8)*x4^2)
        -4*t*s*x2*x4*x8",
    )?;
    let rhs = &(&algebra.parse("b+x8")? * &(&w1 * &w1))
        + &(&algebra.parse("2*x8")? * &(&w1 * &w2))
        + (&algebra.parse("a+x8")? * &(&w2 * &w2))
        + d;
    ensure!(
        &algebra.parse("4*t")? * &transformed == rhs,
        "centered quadratic identity failed"
    );
    println!("4uF=W^T H W+D centered quadratic identity: exact");
    ensure!(
        algebra.parse("(a+x8)*(b+x8)-x8^2")? == algebra.parse("a*b+x8*(a+b)")?,
        "positive determinant identity failed"
    );
    println!("det(H)=ab+x8(a+b)>0: exact");

    let mut counterexample = f;
    for (variable, value) in [
        (0, "5"),
        (1, "12"),
        (2, "1"),
        (3, "1"),
        (5, "1"),
        (6, "1/100"),
        (7, "1/100"),
        (8, "1/100"),
    ] {
        counterexample = counterexample.replace(variable, &rational(value)?);
    }
    let b = counterexample
        .derivative(4)
        .replace(4, &Rational::zero())
        .get_constant();
    let c = counterexample.replace(4, &Rational::zero()).get_constant();
    let disc = symgcad::projection::discriminant(&counterexample, 4)?.get_constant();
    ensure!(
        b.is_negative() && c > Rational::zero() && disc > Rational::zero(),
        "expected two-positive-root counterexample"
    );
    println!("x3 is not globally monotone: s=5,u=12,x1=x2=x4=1,x5=x7=x8=1/100");
    println!(
        "q(x3)={}; b={b}; c={c}; discriminant={disc}",
        algebra.format(&counterexample)
    );
    Ok(())
}
