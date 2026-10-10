//! Research-only exact guard evaluation ablation; never used as proof evidence.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, hint::black_box, path::PathBuf, time::Instant};
use symbolica::prelude::*;
use symgcad::algebra::{Algebra, MAX_DEGREE, Poly, rational};

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 200)]
    iterations: usize,
    #[arg(long, default_value_t = 64)]
    max_guards: usize,
}

#[derive(Deserialize)]
struct Batch {
    variables: Vec<String>,
    prefix: Vec<String>,
    guards: Vec<String>,
    /// Cache inventories can include later-variable polynomials; selection is
    /// explicit and reported, never silently applied to the evaluator itself.
    #[serde(default)]
    select_supported: bool,
}

fn supported(p: &Poly, length: usize) -> bool {
    (length..p.nvars()).all(|axis| p.degree(axis) == 0)
}

fn validate(algebra: &Algebra, p: &Poly, prefix: &[Rational]) -> Result<()> {
    ensure!(
        prefix.len() <= algebra.names.len(),
        "prefix exceeds variable map"
    );
    ensure!(
        p.variables() == &algebra.variables,
        "different variable map"
    );
    ensure!(
        supported(p, prefix.len()),
        "guard uses an unassigned variable"
    );
    ensure!(
        (0..prefix.len()).all(|axis| u64::from(p.degree(axis)) <= MAX_DEGREE),
        "exponent exceeds Symbolica signed range"
    );
    Ok(())
}

/// Reuse Symbolica's existing replace_except power cache. All algebra remains
/// in Symbolica; the extra retained coordinate is checked to be absent.
struct CachedPrefix<'a> {
    algebra: &'a Algebra,
    prefix: Vec<Rational>,
    replacements: Vec<(usize, Rational)>,
    powers: Vec<Vec<Rational>>,
    charged: Vec<usize>,
    retained: usize,
    byte_limit: usize,
    power_limit: usize,
}

impl<'a> CachedPrefix<'a> {
    fn new(
        algebra: &'a Algebra,
        prefix: &[Rational],
        byte_limit: usize,
        power_limit: usize,
    ) -> Result<Self> {
        ensure!(
            prefix.len() <= algebra.names.len(),
            "prefix exceeds variable map"
        );
        ensure!(power_limit <= 64, "research cache power cap is 64");
        Ok(Self {
            algebra,
            prefix: prefix.to_vec(),
            replacements: prefix.iter().cloned().enumerate().collect(),
            powers: vec![vec![]; algebra.names.len()],
            charged: vec![0; algebra.names.len()],
            retained: 0,
            byte_limit,
            power_limit,
        })
    }

    fn evaluate(&mut self, p: &Poly) -> Result<Rational> {
        validate(self.algebra, p, &self.prefix)?;
        if p.is_constant() {
            return Ok(p.get_constant());
        }
        if self.prefix.len() == self.algebra.names.len() {
            return Ok(p.replace_all(&self.prefix));
        }
        for (axis, x) in self.prefix.iter().enumerate() {
            let degree = (p.degree(axis) as usize).min(self.power_limit);
            if degree == 0 || degree < self.powers[axis].len() {
                continue;
            }
            let bits = u128::from(x.numerator_ref().significant_bits())
                + u128::from(x.denominator_ref().significant_bits());
            // Sum of bounds for numerator+denominator bits of powers1..degree,
            // plus conservative owned-rational/vector storage. Rejection only
            // disables caching; Symbolica still evaluates larger powers exactly.
            let charge =
                bits * degree as u128 * (degree + 1) as u128 / 2 / 8 + 128 * (degree + 1) as u128;
            let available = self.byte_limit - self.retained + self.charged[axis];
            if charge > available as u128 {
                continue;
            }
            self.retained = self.retained - self.charged[axis] + charge as usize;
            self.charged[axis] = charge as usize;
            self.powers[axis].resize(degree + 1, Rational::zero());
        }
        let value = p.replace_except(self.prefix.len(), &self.replacements, &mut self.powers);
        ensure!(
            value.is_constant(),
            "guard specialization did not become constant"
        );
        Ok(value.get_constant())
    }
}

fn batch(algebra: &Algebra, guards: &[Poly], prefix: &[Rational]) -> Result<Vec<Rational>> {
    for p in guards {
        validate(algebra, p, prefix)?;
    }
    if prefix.len() == algebra.names.len() {
        return Ok(guards.iter().map(|p| p.replace_all(prefix)).collect());
    }
    Ok(algebra
        .specialize_univariate_batch(guards, prefix.len(), prefix)?
        .into_iter()
        .map(|p| {
            p.coefficients()
                .first()
                .cloned()
                .unwrap_or_else(Rational::zero)
        })
        .collect())
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        (1..=10000).contains(&args.iterations),
        "iteration cap is10000"
    );
    ensure!((1..=256).contains(&args.max_guards), "guard cap is256");
    let bytes = fs::read(&args.input)?;
    let input: Batch = serde_json::from_slice(&bytes)?;
    let algebra = Algebra::new(&input.variables)?;
    let prefix = input
        .prefix
        .iter()
        .map(|s| rational(s))
        .collect::<Result<Vec<_>>>()?;
    let mut guards = input
        .guards
        .iter()
        .map(|p| algebra.parse(p))
        .collect::<Result<Vec<_>>>()?;
    let input_count = guards.len();
    if input.select_supported {
        guards.retain(|p| supported(p, prefix.len()));
    }
    let supported_count = guards.len();
    guards.sort_by_key(|p| std::cmp::Reverse(p.nterms()));
    guards.truncate(args.max_guards);
    ensure!(!guards.is_empty(), "no selected guards");
    for p in &guards {
        validate(&algebra, p, &prefix)?;
    }
    let expected = guards
        .iter()
        .map(|p| p.replace_all(&prefix))
        .collect::<Vec<_>>();
    ensure!(
        batch(&algebra, &guards, &prefix)? == expected,
        "batch values differ"
    );
    let mut cached = CachedPrefix::new(&algebra, &prefix, 65536, 64)?;
    let values = guards
        .iter()
        .map(|p| cached.evaluate(p))
        .collect::<Result<Vec<_>>>()?;
    ensure!(values == expected, "retained-cache values differ");
    let started = Instant::now();
    for _ in 0..args.iterations {
        for p in &guards {
            validate(&algebra, p, &prefix)?;
            black_box(p.replace_all(&prefix));
        }
    }
    let replace_all = started.elapsed().as_secs_f64();
    let started = Instant::now();
    for _ in 0..args.iterations {
        black_box(batch(&algebra, &guards, &prefix)?);
    }
    let shared_batch = started.elapsed().as_secs_f64();
    let started = Instant::now();
    for _ in 0..args.iterations {
        for p in &guards {
            black_box(cached.evaluate(p)?);
        }
    }
    let retained_cache = started.elapsed().as_secs_f64();
    fs::write(
        &args.output,
        serde_json::to_vec_pretty(&json!({
            "kind":"research_guard_evaluation_microbenchmark_no_completion_claim",
            "input_sha256":format!("{:x}",Sha256::digest(&bytes)),
            "variables":input.variables,"prefix":input.prefix,
            "input_guards":input_count,"supported_guards":supported_count,"selected_guards":guards.len(),
            "terms":guards.iter().map(Poly::nterms).sum::<usize>(),
            "selected_guard_sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(&guards.iter().map(|p|algebra.format(p)).collect::<Vec<_>>())?)),
            "exact_outputs_equal":true,"iterations":args.iterations,"charged_cache_bytes":cached.retained,
            "seconds":{"replace_all":replace_all,"shared_batch":shared_batch,"retained_cache":retained_cache}
        }))?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_values_cover_zero_negative_rationals_constants_and_inactive_axes() {
        let a = Algebra::new(&["x".into(), "unused".into(), "y".into(), "later".into()]).unwrap();
        let guards = ["0", "7/3", "x^5+2*y^3-x*y+1", "x^64+y^2"].map(|p| a.parse(p).unwrap());
        for prefix in [["0", "9", "-2/3"], ["-3/2", "0", "5/7"]] {
            let prefix = prefix.map(|s| rational(s).unwrap());
            let mut cache = CachedPrefix::new(&a, &prefix, 65536, 64).unwrap();
            let expected = guards
                .iter()
                .map(|p| p.replace_all(&prefix))
                .collect::<Vec<_>>();
            assert_eq!(batch(&a, &guards, &prefix).unwrap(), expected);
            for _ in 0..2 {
                assert_eq!(
                    guards
                        .iter()
                        .map(|p| cache.evaluate(p).unwrap())
                        .collect::<Vec<_>>(),
                    expected
                );
            }
        }
    }
    #[test]
    fn bounded_cache_falls_back_without_changing_values() {
        let a = Algebra::new(&["x".into(), "z".into()]).unwrap();
        let p = a.parse("x^1000+3*x^67-x^2+1").unwrap();
        let prefix = [rational("-2/3").unwrap()];
        for bytes in [0, 128, 1024, 65536] {
            let mut cache = CachedPrefix::new(&a, &prefix, bytes, 64).unwrap();
            assert_eq!(cache.evaluate(&p).unwrap(), p.replace_all(&prefix));
            assert!(cache.retained <= bytes);
            assert!(cache.powers.iter().all(|v| v.len() <= 65));
        }
    }
    #[test]
    fn unsupported_axes_maps_and_exponents_are_rejected() {
        let a = Algebra::new(&["x".into(), "y".into(), "z".into()]).unwrap();
        let mut cache = CachedPrefix::new(&a, &[Rational::zero()], 65536, 64).unwrap();
        for text in ["y", "x*z"] {
            assert!(cache.evaluate(&a.parse(text).unwrap()).is_err());
        }
        let other = Algebra::new(&["z".into(), "y".into(), "x".into()]).unwrap();
        assert!(cache.evaluate(&other.parse("z").unwrap()).is_err());
        let p = a
            .parse("x")
            .unwrap()
            .monomial(Rational::one(), vec![i32::MAX as u32, 0, 0]);
        assert!(cache.evaluate(&p).is_err());
    }
    #[test]
    fn full_and_empty_prefixes_use_exact_fallbacks() {
        let a = Algebra::new(&["x".into()]).unwrap();
        let mut full = CachedPrefix::new(&a, &[rational("1/3").unwrap()], 0, 0).unwrap();
        assert_eq!(
            full.evaluate(&a.parse("3*x-1").unwrap()).unwrap(),
            Rational::zero()
        );
        let mut empty = CachedPrefix::new(&a, &[], 0, 0).unwrap();
        assert_eq!(
            empty.evaluate(&a.parse("2").unwrap()).unwrap(),
            Rational::from(2)
        );
        assert!(empty.evaluate(&a.parse("x").unwrap()).is_err());
        let zero = Algebra::new(&[]).unwrap();
        assert_eq!(
            CachedPrefix::new(&zero, &[], 0, 0)
                .unwrap()
                .evaluate(&zero.parse("-1").unwrap())
                .unwrap(),
            Rational::from(-1)
        );
    }
}
