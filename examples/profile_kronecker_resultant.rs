//! Research-only, deterministic binary segmentation of a captured resultant.
//! No production dispatch uses this helper. All integer/polynomial arithmetic
//! remains in Symbolica; a hard worker deadline bounds individual calls.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{path::PathBuf, process::Command, sync::Arc, time::Instant};
use symbolica::prelude::*;
use symgcad::{
    algebra::{Algebra, Poly},
    optimized_projection::{AlgebraOperation, OperationCapture},
};

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    inspect_only: bool,
    /// Independently check a saved resultant on a complete exact degree grid.
    /// This mode does not use integer encoding; partial grids never certify it.
    #[arg(long, conflicts_with = "inspect_only")]
    verify_candidate: Option<PathBuf>,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    #[arg(long, default_value_t = 8192)]
    memory_mib: u64,
    /// Static bound on the bits of the encoded output, before allocating it.
    #[arg(long, default_value_t = 2_000_000_000)]
    max_encoded_bits: usize,
    #[arg(long, hide = true)]
    worker: bool,
}

#[derive(Debug, Serialize)]
struct Layout {
    degrees: Vec<usize>,
    weights: Vec<usize>,
    digits: usize,
    digit_bits: usize,
    output_bit_bound: usize,
    coefficient_bound_bits: u64,
}

fn clear(p: &Poly) -> (Poly, Integer) {
    let denominator = p
        .coefficients
        .iter()
        .fold(Integer::one(), |d, c| d.lcm(c.denominator_ref()));
    (
        p.clone().mul_coeff(Rational::from(denominator.clone())),
        denominator,
    )
}

fn norm(p: &Poly) -> Integer {
    p.coefficients
        .iter()
        .fold(Integer::zero(), |s, c| s + c.numerator_ref().abs())
}

// The pinned Integer small-representation left shift can overflow silently;
// route through Symbolica's arbitrary-precision representation explicitly.
fn shift(value: Integer, bits: usize) -> Integer {
    Integer::from(value.to_multi_prec() << bits)
}

fn layout(p: &Poly, q: &Poly, x: usize) -> Result<(Layout, Integer)> {
    ensure!(
        p.variables() == q.variables() && x < p.nvars(),
        "invalid variable map"
    );
    let m = p.degree(x) as usize;
    let n = q.degree(x) as usize;
    ensure!(
        m > 0 && n > 0,
        "prototype requires positive elimination degrees"
    );
    ensure!(m <= 64 && n <= 64, "prototype elimination degree cap is 64");
    let mut degrees = vec![0; p.nvars()];
    for (i, d) in degrees.iter_mut().enumerate().filter(|(i, _)| *i != x) {
        let weighted = |f: &Poly| {
            f.exponents_iter()
                .map(|e| e[x] as usize + e[i] as usize)
                .max()
                .unwrap_or(0)
        };
        let rectangular = n
            .checked_mul(p.degree(i) as usize)
            .and_then(|a| a.checked_add(m.checked_mul(q.degree(i) as usize)?));
        // Every Sylvester determinant term has eliminated-axis weight m*n.
        let diagonal = n
            .checked_mul(weighted(p))
            .and_then(|a| a.checked_add(m.checked_mul(weighted(q))?))
            .and_then(|a| a.checked_sub(m.checked_mul(n)?));
        let (a, b) = rectangular
            .zip(diagonal)
            .ok_or_else(|| anyhow::anyhow!("degree bound overflow"))?;
        // Include input support too, so encoding cannot erase either leading
        // coefficient through monomial aliasing.
        *d = a.min(b).max(p.degree(i) as usize).max(q.degree(i) as usize);
        ensure!(
            *d <= symgcad::algebra::MAX_DEGREE as usize,
            "exponent limit"
        );
    }
    let pn = norm(p);
    let qn = norm(q);
    let coefficient_bits = pn
        .significant_bits()
        .checked_mul(n as u64)
        .and_then(|b| b.checked_add(qn.significant_bits().checked_mul(m as u64)?))
        .ok_or_else(|| anyhow::anyhow!("coefficient bound overflow"))?;
    ensure!(
        coefficient_bits <= 1_000_000,
        "coefficient bound exceeds static bit cap"
    );
    let bound = pn.pow(n as u64) * qn.pow(m as u64);
    // Product of Sylvester row l1 norms bounds every determinant coefficient.
    let input_bound = p
        .coefficients
        .iter()
        .chain(&q.coefficients)
        .map(|c| c.numerator_ref().abs())
        .max()
        .unwrap_or_else(Integer::zero);
    let digit_bits = usize::try_from(bound.clone().max(input_bound).significant_bits())?
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("digit size overflow"))?;
    let mut weights = vec![0; p.nvars()];
    let mut digits = 1usize;
    // Reverse index order makes increasing digit positions lexicographic in
    // the original polynomial map, allowing linear-time sparse construction.
    for i in (0..p.nvars()).rev().filter(|&i| i != x) {
        weights[i] = digits;
        digits = digits
            .checked_mul(degrees[i] + 1)
            .ok_or_else(|| anyhow::anyhow!("degree box overflow"))?;
    }
    let output_bit_bound = digits
        .checked_mul(digit_bits)
        .ok_or_else(|| anyhow::anyhow!("encoding size overflow"))?;
    Ok((
        Layout {
            degrees,
            weights,
            digits,
            digit_bits,
            output_bit_bound,
            coefficient_bound_bits: bound.significant_bits(),
        },
        bound,
    ))
}

fn code(e: &[u32], layout: &Layout) -> Result<usize> {
    e.iter()
        .zip(&layout.weights)
        .try_fold(0usize, |a, (&e, &w)| {
            a.checked_add(
                (e as usize)
                    .checked_mul(w)
                    .ok_or_else(|| anyhow::anyhow!("encoding exponent overflow"))?,
            )
            .ok_or_else(|| anyhow::anyhow!("encoding exponent overflow"))
        })
}

fn encode(p: &Poly, x: usize, layout: &Layout) -> Result<UnivariatePolynomial<Z>> {
    let mut terms = vec![Vec::<(usize, Integer)>::new(); p.degree(x) as usize + 1];
    for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
        ensure!(
            c.denominator_ref().is_one(),
            "encoding requires integer coefficients"
        );
        terms[e[x] as usize].push((code(e, layout)?, c.numerator()));
    }
    let coefficients = terms
        .into_iter()
        .map(|mut terms| {
            terms.sort_unstable_by_key(|(e, _)| *e);
            let mut value = Integer::zero();
            let mut previous = terms.last().map(|(e, _)| *e).unwrap_or(0);
            for (e, c) in terms.into_iter().rev() {
                value = shift(value, (previous - e) * layout.digit_bits);
                value += c;
                previous = e;
            }
            shift(value, previous * layout.digit_bits)
        })
        .collect();
    let result = UnivariatePolynomial::from_coefficients(
        &Z,
        coefficients,
        Arc::new(p.variables()[x].clone()),
    );
    ensure!(
        result.degree() == p.degree(x) as usize && !result.is_zero(),
        "encoding lost leading degree"
    );
    Ok(result)
}

fn integer_resultant(p: &UnivariatePolynomial<Z>, q: &UnivariatePolynomial<Z>) -> Integer {
    let mut r = p.resultant(q);
    // Match the production workaround for the reference backend's odd linear
    // small-case sign convention. The higher-degree Ducos path is unchanged.
    if (p.degree() == 1 || q.degree() == 1) && p.degree() % 2 == 1 && q.degree() % 2 == 1 {
        r = -r;
    }
    r
}

fn decode(
    value: Integer,
    offset: usize,
    count: usize,
    layout: &Layout,
    bound: &Integer,
    output: &mut Poly,
) -> Result<()> {
    if value.is_zero() {
        return Ok(());
    }
    if count == 1 {
        ensure!(
            value.abs() <= *bound,
            "decoded coefficient exceeds certified bound"
        );
        let mut e = vec![0u32; output.nvars()];
        let mut index = offset;
        for (i, &weight) in layout.weights.iter().enumerate().filter(|(_, w)| **w != 0) {
            e[i] = u32::try_from(index / weight)?;
            ensure!(
                e[i] as usize <= layout.degrees[i],
                "decoded exponent exceeds bound"
            );
            index %= weight;
        }
        ensure!(index == 0, "invalid decoded exponent");
        output.append_monomial_back(Rational::from(value), &e);
        return Ok(());
    }
    let low_count = count / 2;
    let radix = shift(Integer::one(), low_count * layout.digit_bits);
    let (mut high, mut low) = value.quot_rem(&radix);
    // Centered blocks preserve negative digits without a sequential carry
    // walk. Since |c| < B/2, every block fits strictly inside +/- B^k/2.
    if &low + &low >= radix {
        low -= &radix;
        high += Integer::one();
    }
    decode(low, offset, low_count, layout, bound, output)?;
    decode(
        high,
        offset + low_count,
        count - low_count,
        layout,
        bound,
        output,
    )
}

fn compute(p: &Poly, q: &Poly, x: usize, max_bits: usize) -> Result<(Poly, serde_json::Value)> {
    let started = Instant::now();
    let (p, pd) = clear(p);
    let (q, qd) = clear(q);
    let (layout, bound) = layout(&p, &q, x)?;
    ensure!(
        layout.output_bit_bound <= max_bits,
        "encoded output exceeds static bit cap"
    );
    let ep = encode(&p, x, &layout)?;
    let eq = encode(&q, x, &layout)?;
    let encoding_seconds = started.elapsed().as_secs_f64();
    let numeric_started = Instant::now();
    let encoded = integer_resultant(&ep, &eq);
    let resultant_seconds = numeric_started.elapsed().as_secs_f64();
    let decoding_started = Instant::now();
    let mut result = p.zero();
    decode(
        encoded.clone(),
        0,
        layout.digits,
        &layout,
        &bound,
        &mut result,
    )?;
    // Round trip plus the proven coefficient and rectangular degree bounds
    // makes this reconstruction deterministic, including a zero resultant.
    let reencoded = if result.is_zero() {
        Integer::zero()
    } else {
        encode(&result, x, &layout)?.get_constant()
    };
    ensure!(
        reencoded == encoded,
        "resultant encoding reconstruction failed"
    );
    let scale = pd.pow(q.degree(x) as u64) * qd.pow(p.degree(x) as u64);
    result = result.mul_coeff(Rational::one() / Rational::from(scale));
    Ok((
        result,
        json!({"layout":layout,"encoding_seconds":encoding_seconds,
        "resultant_seconds":resultant_seconds,"decoding_seconds":decoding_started.elapsed().as_secs_f64(),
        "numeric_result_bits":encoded.significant_bits(),"total_seconds":started.elapsed().as_secs_f64(),
        "reconstruction":"exact bounded mixed-radix injection and round trip"}),
    ))
}

fn verify_grid(
    p: &Poly,
    q: &Poly,
    candidate: &Poly,
    x: usize,
    layout: &Layout,
    axes: &[usize],
    checked: &mut usize,
    progress: &mut impl FnMut(usize) -> Result<()>,
) -> Result<()> {
    if axes.is_empty() {
        let expected = symgcad::projection::exact_resultant_ducos(p, q, x)?;
        ensure!(
            expected.is_constant() && candidate.is_constant(),
            "grid leaf is not numeric"
        );
        ensure!(
            expected.get_constant() == candidate.get_constant(),
            "exact grid identity failed"
        );
        *checked += 1;
        if *checked % 128 == 0 {
            progress(*checked)?;
        }
        return Ok(());
    }
    let axis = axes[0];
    let required = layout.degrees[axis] + 1;
    let max_skips = p.degree(axis) as usize + q.degree(axis) as usize;
    let mut done = 0;
    for attempt in 0..required + max_skips {
        let magnitude = i64::try_from(attempt.div_ceil(2))?;
        let value = Rational::from(if attempt % 2 == 0 {
            -magnitude
        } else {
            magnitude
        });
        let left = p.replace(axis, &value);
        let right = q.replace(axis, &value);
        if left.degree(x) != p.degree(x) || right.degree(x) != q.degree(x) {
            continue;
        }
        verify_grid(
            &left,
            &right,
            &candidate.replace(axis, &value),
            x,
            layout,
            &axes[1..],
            checked,
            progress,
        )?;
        done += 1;
        if done == required {
            break;
        }
    }
    ensure!(
        done == required,
        "grid has too many leading-coefficient zeros"
    );
    Ok(())
}

fn verify(
    p: &Poly,
    q: &Poly,
    candidate: &Poly,
    x: usize,
    mut progress: impl FnMut(usize) -> Result<()>,
) -> Result<serde_json::Value> {
    let started = Instant::now();
    let (ip, _) = clear(p);
    let (iq, _) = clear(q);
    let (layout, _) = layout(&ip, &iq, x)?;
    ensure!(
        p.variables() == candidate.variables(),
        "candidate variable map differs"
    );
    ensure!(
        candidate.degree(x) == 0,
        "candidate contains eliminated axis"
    );
    ensure!(
        (0..p.nvars()).all(|i| candidate.degree(i) as usize <= layout.degrees[i]),
        "candidate degree exceeds bound"
    );
    ensure!(
        layout.digits <= 100_000,
        "verification grid exceeds100000 points"
    );
    let axes: Vec<_> = (0..p.nvars())
        .filter(|&i| i != x && layout.degrees[i] > 0)
        .collect();
    let mut checked = 0;
    verify_grid(
        p,
        q,
        candidate,
        x,
        &layout,
        &axes,
        &mut checked,
        &mut progress,
    )?;
    ensure!(checked == layout.digits, "incomplete verification grid");
    progress(checked)?;
    Ok(
        json!({"verified":true,"method":"complete nested degree grid; exact numeric Ducos resultants; degree drops skipped",
        "checked_points":checked,"degree_bounds":layout.degrees,"seconds":started.elapsed().as_secs_f64()}),
    )
}

fn main() -> Result<()> {
    let args = Args::parse();
    if !args.worker {
        ensure!(
            args.seconds.is_finite() && args.seconds > 0. && args.seconds <= 3600.,
            "invalid deadline"
        );
        ensure!(
            args.memory_mib > 0 && args.memory_mib <= 65536,
            "invalid memory cap"
        );
        let sidecar = args.output.with_extension("process.json");
        ensure!(
            !args.output.exists() && !sidecar.exists(),
            "refusing to overwrite evidence"
        );
        if let Some(parent) = args.output.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let binary = std::env::current_exe()?;
        let mut command = Command::new(&binary);
        command
            .args(std::env::args_os().skip(1))
            .arg("--worker")
            .env("RAYON_NUM_THREADS", "1")
            .env("OMP_NUM_THREADS", "1");
        let limits = symgcad::Limits {
            wall_time_secs: args.seconds,
            memory_mib: args.memory_mib,
            workers: 1,
            max_cells: None,
        };
        let process = symgcad::benchmark::run_bounded(&mut command, &limits)?;
        let report = json!({"schema_version":1,"limits":limits,"process":process,
            "capture_sha256":format!("{:x}",Sha256::digest(std::fs::read(&args.capture)?)),
            "binary_sha256":format!("{:x}",Sha256::digest(std::fs::read(binary)?))});
        std::fs::write(sidecar, serde_json::to_vec_pretty(&report)?)?;
        println!("{}", serde_json::to_string(&report)?);
        return Ok(());
    }
    let capture: OperationCapture = serde_json::from_slice(&std::fs::read(&args.capture)?)?;
    let operands = capture
        .operands
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing operands"))?;
    ensure!(
        capture.input_hash == format!("{:x}", Sha256::digest(serde_json::to_vec(operands)?)),
        "input hash mismatch"
    );
    let algebra = Algebra::new(&capture.variable_names)?;
    let x = capture
        .variable
        .ok_or_else(|| anyhow::anyhow!("missing variable"))?;
    let p = algebra.parse(
        operands
            .first()
            .ok_or_else(|| anyhow::anyhow!("missing first operand"))?,
    )?;
    ensure!(x < p.nvars(), "invalid variable");
    let discriminant = capture.operation == AlgebraOperation::Discriminant;
    let q = if discriminant {
        ensure!(operands.len() == 1, "discriminant arity");
        p.derivative(x)
    } else {
        ensure!(
            capture.operation == AlgebraOperation::Resultant && operands.len() == 2,
            "resultant capture required"
        );
        algebra.parse(&operands[1])?
    };
    let report = if let Some(path) = &args.verify_candidate {
        ensure!(
            !discriminant,
            "grid verification currently accepts resultant captures only"
        );
        let bytes = std::fs::read(path)?;
        let saved: serde_json::Value = serde_json::from_slice(&bytes)?;
        ensure!(
            saved["input_hash"].as_str() == Some(capture.input_hash.as_str()),
            "candidate input mismatch"
        );
        let expression = saved["result"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing candidate"))?;
        let digest = format!("{:x}", Sha256::digest(expression.as_bytes()));
        ensure!(
            saved["result_sha256"].as_str() == Some(digest.as_str()),
            "candidate checksum mismatch"
        );
        let candidate = algebra.parse(expression)?;
        let progress_path = args.output.with_extension("progress.json");
        let verification = verify(&p, &q, &candidate, x, |checked| {
            let temporary = progress_path.with_extension("tmp");
            std::fs::write(
                &temporary,
                serde_json::to_vec(&json!({"checked_points":checked,"verified":false}))?,
            )?;
            std::fs::rename(temporary, &progress_path)?;
            Ok(())
        })?;
        json!({"schema_version":1,"input_hash":capture.input_hash,"candidate_sha256":format!("{:x}",Sha256::digest(bytes)),"verification":verification})
    } else if args.inspect_only {
        let (ip, _) = clear(&p);
        let (iq, _) = clear(&q);
        let (layout, _) = layout(&ip, &iq, x)?;
        json!({"schema_version":1,"input_hash":capture.input_hash,"layout":layout,
            "within_static_cap":layout.output_bit_bound<=args.max_encoded_bits})
    } else {
        let (mut result, stats) = compute(&p, &q, x, args.max_encoded_bits)?;
        if discriminant {
            let (quotient, remainder) = result.quot_rem(&p.univariate_lcoeff(x), false);
            ensure!(remainder.is_zero(), "non-exact discriminant division");
            result = quotient;
            let d = p.degree(x) as u64;
            if d * (d - 1) / 2 % 2 == 1 {
                result = -result;
            }
        }
        let text = algebra.format(&result);
        json!({"schema_version":1,"input_hash":capture.input_hash,"operation":capture.operation,
            "statistics":stats,"result_terms":result.nterms(),"result":text,
            "result_sha256":format!("{:x}",Sha256::digest(text.as_bytes()))})
    };
    std::fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn algebra() -> Algebra {
        Algebra::new(&["a", "b", "x"].map(str::to_owned)).unwrap()
    }
    #[test]
    fn exact_resultants_cover_rational_scale_shared_roots_and_odd_linear_sign() {
        let a = algebra();
        for (p, q) in [
            ("x^3+a*x+b", "x^2-a*b*x+2"),
            ("x^3/7-a*x/3+b/11", "x^2/5+b*x/13-a/17"),
            ("(x-a)*(x+b)", "(x-a)*(x^2+b)"),
            ("x^3+a", "2*x-b"),
            ("2*x-b", "x^3+a"),
            ("a*x+b", "b*x-a"),
            ("x^2-a", "x^2-b"),
            ("x^5-a*b*x+b^3", "x^3+a^2*x-b"),
        ] {
            let p = a.parse(p).unwrap();
            let q = a.parse(q).unwrap();
            let (r, _) = compute(&p, &q, 2, 100_000_000).unwrap();
            assert_eq!(
                r,
                symgcad::projection::exact_resultant_ducos(&p, &q, 2).unwrap()
            );
        }
    }
    #[test]
    fn centered_blocks_handle_negative_values_and_sparse_high_terms() {
        let a = algebra();
        let p = a.parse("x^2+a^2+b^2-2").unwrap();
        let q = a.parse("x^2-a*b-3").unwrap();
        let (layout, bound) = layout(&p, &q, 2).unwrap();
        for text in ["-a^2*b^2+7*a*b-3", "a^2*b^2-7*a*b+3", "-3", "0"] {
            let target = a.parse(text).unwrap();
            let value = if target.is_zero() {
                Integer::zero()
            } else {
                encode(&target, 2, &layout).unwrap().get_constant()
            };
            let mut result = p.zero();
            decode(value, 0, layout.digits, &layout, &bound, &mut result).unwrap();
            assert_eq!(result, target);
        }
    }
    #[test]
    fn static_limits_and_invalid_inputs_are_errors() {
        let a = algebra();
        let p = a.parse("x^2+a+b").unwrap();
        let q = a.parse("x^2-a*b").unwrap();
        assert!(compute(&p, &q, 2, 1).is_err());
        assert!(layout(&p, &a.parse("a").unwrap(), 2).is_err());
        assert!(layout(&p, &q, 3).is_err());
        let (l, b) = layout(&p, &q, 2).unwrap();
        let mut r = p.zero();
        assert!(decode(&b + Integer::one(), 0, 1, &l, &b, &mut r).is_err());
    }

    #[test]
    fn complete_grid_checks_identity_and_rejects_perturbed_candidate() {
        let a = algebra();
        let p = a.parse("a*x^3+b*x+1").unwrap();
        let q = a.parse("b*x^2-a*x+3").unwrap();
        let (r, _) = compute(&p, &q, 2, 100_000_000).unwrap();
        let checked = verify(&p, &q, &r, 2, |_| Ok(())).unwrap();
        assert_eq!(checked["verified"], true);
        assert!(verify(&p, &q, &(&r + &a.parse("a*b").unwrap()), 2, |_| Ok(())).is_err());
        assert!(verify(&p, &q, &a.parse("a^1000").unwrap(), 2, |_| Ok(())).is_err());
    }
}
