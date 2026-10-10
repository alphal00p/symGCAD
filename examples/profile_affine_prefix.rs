//! Research-only affine-prefix microbenchmark against the retained pre-shortcut evaluator.
//! The legacy module below is copied from prefix_value.rs before this change.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, hint::black_box, path::PathBuf, time::Instant};
use symbolica::prelude::*;
#[cfg(test)]
use symgcad::algebra;
use symgcad::algebra::{Algebra, Poly, rational};

#[path = "../src/algebra/prefix_value.rs"]
mod current;

mod legacy {
    //! Exact prefix evaluation with a bounded integer numerator fast path.
    //!
    //! For integer coefficients and rational coordinates `n_i/d_i`, multiply every
    //! term by the common positive denominator `product(d_i^degree_i)`. Symbolica
    //! performs integer arithmetic until one final rational normalization. The
    //! ordinary Symbolica evaluator handles inputs outside the fast-path bounds.

    use anyhow::{Result, ensure};
    use symbolica::prelude::*;

    use super::Poly;

    const MAX_TERMS: usize = 4096;
    const MAX_ACTIVE_AXES: usize = 16;
    const MAX_AXIS_DEGREE: usize = 64;
    const MAX_COEFFICIENT_BITS: u64 = 4096;
    const MAX_POWER_BITS: u128 = 16384;
    const MAX_SCRATCH_ESTIMATE: u128 = 1024 * 1024;
    const MAX_EXPONENT_VISITS: usize = 1024 * 1024;

    /// Evaluate a polynomial whose *entire support* is already assigned. A later
    /// coordinate must not be ignored even if substituting an earlier zero could
    /// make its coefficient vanish. Prefix coordinates use the polynomial's exact
    /// variable-map order; no compaction or remapping takes place here.
    pub(crate) fn evaluate(p: &Poly, prefix: &[Rational]) -> Result<Rational> {
        ensure!(prefix.len() <= p.nvars(), "prefix exceeds variable map");
        ensure!(
            (prefix.len()..p.nvars()).all(|i| p.degree(i) == 0),
            "guard depends on a later coordinate"
        );
        Ok(integer_value(p, prefix).unwrap_or_else(|| p.replace_all(prefix)))
    }

    /// `evaluate` validates complete support before calling this private helper.
    /// These bounds constrain temporary powers and arithmetic work, not the
    /// allocation size of the input polynomial or the fallback backend operation.
    fn integer_value(p: &Poly, prefix: &[Rational]) -> Option<Rational> {
        if p.is_constant() {
            return Some(p.get_constant());
        }
        if p.nterms() > MAX_TERMS
            || prefix.len().checked_mul(p.nterms())? > MAX_EXPONENT_VISITS
            || p.coefficients.iter().any(|c| {
                !c.is_integer() || c.numerator_ref().significant_bits() > MAX_COEFFICIENT_BITS
            })
        {
            return None;
        }

        let mut active = Vec::new();
        for (i, value) in prefix.iter().enumerate() {
            let degree = p.degree(i) as usize;
            if degree == 0 {
                continue;
            }
            if degree > MAX_AXIS_DEGREE
                || active.len() == MAX_ACTIVE_AXES
                || value.denominator_ref().is_zero()
                || value.denominator_ref().is_negative()
            {
                return None;
            }
            active.push((i, degree));
        }

        // u128 accounting prevents overflow even for a very large input integer.
        // At most 16 degrees, each <=64, enter these sums. Include all three power
        // arrays in the estimate, although numerator/denominator powers for earlier
        // axes have been dropped by the time a later axis is constructed.
        let mut power_bits = 0u128;
        let mut storage = 0u128;
        for &(i, degree) in &active {
            let bits = u128::from(prefix[i].numerator_ref().significant_bits())
                + u128::from(prefix[i].denominator_ref().significant_bits())
                + 2;
            let axis_bits = bits * degree as u128;
            power_bits += axis_bits;
            storage += 3 * (degree + 1) as u128 * (64 + axis_bits.div_ceil(8));
        }
        if power_bits > MAX_POWER_BITS || storage > MAX_SCRATCH_ESTIMATE {
            return None;
        }

        let mut denominator = Integer::one();
        let mut powers = Vec::with_capacity(active.len());
        for &(i, degree) in &active {
            let mut numerators = vec![Integer::one(); degree + 1];
            let mut denominators = vec![Integer::one(); degree + 1];
            for exponent in 1..=degree {
                numerators[exponent] = Z.mul(&numerators[exponent - 1], prefix[i].numerator_ref());
                denominators[exponent] =
                    Z.mul(&denominators[exponent - 1], prefix[i].denominator_ref());
            }
            Z.mul_assign(&mut denominator, &denominators[degree]);
            powers.push(
                (0..=degree)
                    .map(|exponent| Z.mul(&numerators[exponent], &denominators[degree - exponent]))
                    .collect::<Vec<_>>(),
            );
        }

        let mut numerator = Integer::zero();
        for term in p {
            let mut value = term.coefficient.numerator_ref().clone();
            for (axis, &(i, _)) in active.iter().enumerate() {
                let power = &powers[axis][term.exponents[i] as usize];
                if !power.is_one() {
                    Z.mul_assign(&mut value, power);
                }
                if value.is_zero() {
                    break;
                }
            }
            Z.add_assign(&mut numerator, &value);
        }
        Some(Rational::from((numerator, denominator)))
    }
}

fn candidate(p: &Poly, prefix: &[Rational]) -> Result<Rational> {
    current::evaluate(p, prefix)
}

#[derive(Deserialize)]
struct Batch {
    variables: Vec<String>,
    prefix: Vec<String>,
    guards: Vec<String>,
    provenance: serde_json::Value,
}

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 100000)]
    iterations: usize,
    #[arg(long, default_value_t = 5)]
    repeats: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!((1..=2_000_000).contains(&args.iterations), "iteration cap");
    ensure!((1..=9).contains(&args.repeats), "repetition cap");
    let bytes = fs::read(&args.input)?;
    ensure!(bytes.len() <= 1024 * 1024, "input cap");
    let batch: Batch = serde_json::from_slice(&bytes)?;
    ensure!((1..=64).contains(&batch.guards.len()), "guard cap");
    let a = Algebra::new(&batch.variables)?;
    let prefix = batch
        .prefix
        .iter()
        .map(|s| rational(s))
        .collect::<Result<Vec<_>>>()?;
    let guards = batch
        .guards
        .iter()
        .map(|s| a.parse(s))
        .collect::<Result<Vec<_>>>()?;
    let mut values = Vec::new();
    for p in &guards {
        let value = candidate(p, &prefix)?;
        ensure!(
            value == legacy::evaluate(p, &prefix)? && value == p.replace_all(&prefix),
            "exact value mismatch"
        );
        values.push(value.to_string());
    }
    let mut trials = Vec::new();
    for repeat in 0..args.repeats {
        let mut seconds = [0.0; 2];
        for mode in [repeat % 2, 1 - repeat % 2] {
            let start = Instant::now();
            for _ in 0..args.iterations {
                for p in &guards {
                    let p = black_box(p);
                    let prefix = black_box(prefix.as_slice());
                    black_box(if mode == 0 {
                        legacy::evaluate(p, prefix)?
                    } else {
                        candidate(p, prefix)?
                    });
                }
            }
            seconds[mode] = start.elapsed().as_secs_f64();
        }
        trials.push(json!({"repeat":repeat,"first":if repeat%2==0 {"legacy"} else {"affine"},"legacy_seconds":seconds[0],"affine_seconds":seconds[1]}));
    }
    fs::write(
        args.output,
        serde_json::to_vec_pretty(&json!({
            "schema_version":1,"kind":"supplemental_microbenchmark_not_solver_speedup",
            "input_sha256":format!("{:x}",Sha256::digest(&bytes)),"provenance":batch.provenance,
            "variables":batch.variables,"prefix":batch.prefix,"guards":batch.guards,"values":values,
            "exact_symbolica_equal":true,"iterations":args.iterations,"trials":trials
        }))?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_single_axis_families_match_symbolica_and_legacy() {
        let a = Algebra::new(&["s".into(), "unused".into(), "x".into(), "later".into()]).unwrap();
        let mut state = 19377u64;
        for _ in 0..1000 {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let coefficient = (state % 41) as i64 - 20;
            let offset = ((state >> 8) % 53) as i64 - 26;
            let axis = if state & 1 == 0 { "s" } else { "x" };
            let p = a
                .parse(&format!("({coefficient})*{axis}+({offset})"))
                .unwrap();
            let prefix = [
                Rational::from((coefficient, 17)),
                Rational::from(-999),
                Rational::from((offset, 23)),
            ];
            assert_eq!(candidate(&p, &prefix).unwrap(), p.replace_all(&prefix));
            assert_eq!(
                candidate(&p, &prefix).unwrap(),
                legacy::evaluate(&p, &prefix).unwrap()
            );
        }
    }

    #[test]
    fn unsupported_support_and_caps_never_bypass_validation() {
        let a = Algebra::new(&["x".into(), "unused".into(), "y".into()]).unwrap();
        let prefix = [
            rational("-2/3").unwrap(),
            Rational::zero(),
            rational("7/11").unwrap(),
        ];
        for text in ["x*y", "x+y", "x^2-1", "x/7+1/3", "7", "0"] {
            let p = a.parse(text).unwrap();
            assert_eq!(candidate(&p, &prefix).unwrap(), p.replace_all(&prefix));
        }
        for text in ["y", "x*y", "y-4"] {
            assert!(candidate(&a.parse(text).unwrap(), &[Rational::zero()]).is_err());
        }
        assert!(candidate(&a.parse("0").unwrap(), &vec![Rational::zero(); 4]).is_err());
        let huge = Integer::from(Integer::one().to_multi_prec() << 17000usize);
        let p = a.parse("x-4").unwrap();
        let point = [Rational::from((huge, Integer::from(3)))];
        assert_eq!(candidate(&p, &point).unwrap(), p.replace_all(&point));
        let coefficient = Integer::from(Integer::one().to_multi_prec() << 4096usize);
        let p = a.parse(&format!("{coefficient}*x-1")).unwrap();
        assert_eq!(candidate(&p, &prefix).unwrap(), p.replace_all(&prefix));
    }
}
