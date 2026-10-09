//! Compare exact Sturm chains on a captured univariate guard.
use anyhow::{Context, Result, ensure};
use clap::{Parser, ValueEnum};
use serde_json::json;
use std::{path::PathBuf, time::Instant};
use symbolica::prelude::*;
use symgcad::{algebra::Algebra, optimized_projection::OperationCapture};

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Strategy {
    Rational,
    Primitive,
    Pseudo,
}
#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long, value_enum)]
    strategy: Strategy,
    #[arg(long)]
    output: PathBuf,
}
fn variations<F: Ring>(p: &[UnivariatePolynomial<F>], x: &F::Element, zero: &F::Element) -> usize
where
    F::Element: PartialOrd,
{
    let mut last = 0;
    let mut count = 0;
    for p in p {
        let v = p.evaluate(x);
        let sign = if &v < zero {
            -1
        } else if &v > zero {
            1
        } else {
            0
        };
        if sign != 0 {
            count += usize::from(last != 0 && sign != last);
            last = sign;
        }
    }
    count
}
fn main() -> Result<()> {
    let args = Args::parse();
    let capture: OperationCapture = serde_json::from_reader(std::fs::File::open(&args.capture)?)?;
    let algebra = Algebra::new(&capture.variable_names)?;
    let p = algebra.parse(&capture.operands.context("missing capture operands")?[0])?;
    let active: Vec<_> = (0..p.nvars()).filter(|&i| p.degree(i) > 0).collect();
    ensure!(active.len() == 1, "capture must be univariate");
    let p = p.to_univariate_from_univariate(active[0]);
    let primitive = |p: UnivariatePolynomial<Q>| {
        let c = p.content().abs();
        p.div_coeff(&c)
    };
    let p = primitive(p);
    ensure!(
        !p.coefficients()[0].is_zero(),
        "zero endpoint unsupported by this profile"
    );
    let radius = p.coefficients().iter().map(|c| c.abs()).max().unwrap() + Rational::from(2);
    let started = Instant::now();
    let (count, length, max_bits) = match args.strategy {
        Strategy::Rational | Strategy::Primitive => {
            let mut seq = vec![p.clone(), p.derivative()];
            loop {
                let n = seq.len();
                let r = seq[n - 2].rem(&seq[n - 1]);
                if r.is_zero() {
                    break;
                }
                seq.push(if matches!(args.strategy, Strategy::Primitive) {
                    primitive(-r)
                } else {
                    -r
                });
            }
            let bits = seq
                .iter()
                .flat_map(|p| p.coefficients())
                .map(|c| {
                    c.numerator_ref()
                        .significant_bits()
                        .max(c.denominator_ref().significant_bits())
                })
                .max()
                .unwrap();
            (
                variations(&seq, &Rational::zero(), &Rational::zero())
                    - variations(&seq, &radius, &Rational::zero()),
                seq.len(),
                bits,
            )
        }
        Strategy::Pseudo => {
            let p = p.map_coeff(|c| c.numerator().clone(), Z);
            let primitive = |p: UnivariatePolynomial<Z>| {
                let c = p.content().abs();
                p.div_coeff(&c)
            };
            let mut seq = vec![p.clone(), primitive(p.derivative())];
            loop {
                let n = seq.len();
                let q = if seq[n - 1].lcoeff().is_negative() {
                    -seq[n - 1].clone()
                } else {
                    seq[n - 1].clone()
                };
                let r = seq[n - 2].pseudo_remainder(&q);
                if r.is_zero() {
                    break;
                }
                seq.push(primitive(-r));
            }
            let bits = seq
                .iter()
                .flat_map(|p| p.coefficients())
                .map(|c| c.significant_bits())
                .max()
                .unwrap();
            (
                variations(&seq, &Integer::zero(), &Integer::zero())
                    - variations(&seq, &radius.numerator(), &Integer::zero()),
                seq.len(),
                bits,
            )
        }
    };
    let report = json!({"capture":args.capture,"strategy":format!("{:?}",args.strategy),"degree":p.degree(),"positive_roots":count,"chain_length":length,"maximum_coefficient_bits":max_bits,"seconds":started.elapsed().as_secs_f64()});
    std::fs::write(args.output, serde_json::to_vec_pretty(&report)?)?;
    println!("{report}");
    Ok(())
}
