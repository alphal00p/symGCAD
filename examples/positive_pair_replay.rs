//! Research-only identities and existing certificates for a positive pair lift.
//! This is not a root arrangement, CAD proof, or production certificate format.
//! Run under an external killable deadline and aggregate memory limit.
use anyhow::{Context, Result, bail, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    algebra::{Algebra, Poly, rational},
    domain::{
        CertificateOptions, DomainCertificate, certify_nonzero, derive_domain, verify_certificate,
    },
    projection::exact_resultant,
};

const MAX_BYTES: usize = 256 * 1024;
const MAX_VARIABLES: usize = 12;
const MAX_INPUT_TERMS: usize = 256;
const MAX_TERMS: usize = 2048;
const MAX_INPUT_DEGREE: u32 = 8;
const MAX_DEGREE: u32 = 24;
const MAX_HEIGHT: u64 = 512;
const MAX_WORK: usize = 2_000_000;

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 120.0)]
    seconds: f64,
    #[arg(long, default_value_t = 3)]
    cpu: usize,
    #[arg(long, hide = true)]
    worker: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    name: String,
    left: String,
    right: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    /// Original lifting order; the selected y,x,z must be its final three axes.
    variables: Vec<String>,
    /// Original strict constraints, each expression interpreted as p > 0.
    constraints: Vec<String>,
    polynomial: String,
    axes: [String; 3],
    /// F_original = positive_scale * (q*z+B), with q monic in x.
    positive_scale: String,
    /// Base-only proposals h_x,h_y; no assertion here is trusted.
    endpoint_multipliers: [String; 2],
    #[serde(default)]
    identities: Vec<Identity>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NamedCertificate {
    name: String,
    certificate: DomainCertificate,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    schema_version: u32,
    input_sha256: String,
    polynomials: BTreeMap<String, String>,
    certificates: Vec<NamedCertificate>,
    checked_identity_names: Vec<String>,
}

struct Extracted {
    algebra: Algebra,
    constraints: Vec<Poly>,
    polynomials: BTreeMap<String, Poly>,
    positive: Vec<(&'static str, Poly)>,
}

#[derive(Default)]
struct Budget {
    product_work: usize,
}
impl Budget {
    fn multiply(&mut self, a: &Poly, b: &Poly) -> Result<Poly> {
        self.product_work = self
            .product_work
            .saturating_add(a.nterms().saturating_mul(b.nterms()));
        ensure!(self.product_work <= MAX_WORK, "polynomial product-work cap");
        let result = a * b;
        bounded(&result, MAX_TERMS, MAX_DEGREE)?;
        Ok(result)
    }
}

fn bounded(p: &Poly, terms: usize, degree: u32) -> Result<()> {
    ensure!(
        p.nvars() <= MAX_VARIABLES && p.nterms() <= terms,
        "polynomial size cap"
    );
    ensure!(
        p.exponents_iter()
            .all(|e| e.iter().map(|e| u64::from(*e)).sum::<u64>() <= u64::from(degree)),
        "polynomial degree cap"
    );
    ensure!(
        p.coefficients.iter().all(|c| {
            c.numerator_ref().significant_bits() as u64
                + c.denominator_ref().significant_bits() as u64
                <= MAX_HEIGHT
        }),
        "polynomial coefficient-height cap"
    );
    Ok(())
}

fn parse(algebra: &Algebra, expression: &str) -> Result<Poly> {
    expression_budget(expression, &algebra.names)?;
    let p = algebra.parse(expression)?;
    bounded(&p, MAX_INPUT_TERMS, MAX_INPUT_DEGREE)?;
    Ok(p)
}

/// Reuse Symbolica's unnormalized token parser, before Atom can expand powers.
/// These are conservative expansion/height estimates, not a second algebra.
fn expression_budget(text: &str, names: &[String]) -> Result<()> {
    use symbolica::parser::{Operator, ParseSettings, Token};
    ensure!(text.len() <= MAX_BYTES, "polynomial text cap");
    let (mut depth, mut digits, mut operators) = (0usize, 0usize, 0usize);
    for byte in text.bytes() {
        digits = if byte.is_ascii_digit() { digits + 1 } else { 0 };
        ensure!(digits <= 128, "raw numeric literal cap");
        if matches!(byte, b'+' | b'-' | b'*' | b'/' | b'^') {
            operators += 1;
            ensure!(operators <= 4096, "raw operator cap");
        }
        if byte == b'(' {
            depth += 1;
            ensure!(depth <= 32, "raw nesting cap");
        } else if byte == b')' {
            depth = depth.checked_sub(1).context("unbalanced raw expression")?;
        }
    }
    ensure!(depth == 0, "unbalanced raw expression");
    let token = Token::parse(
        &text.replace("**", "^"),
        ParseSettings::symbolica().convert_mul_to_atom(false),
    )
    .map_err(|e| anyhow::anyhow!("invalid polynomial tokens: {e}"))?;
    fn estimate(
        t: &Token,
        names: &[String],
        visits: &mut usize,
        depth: usize,
    ) -> Result<(usize, u32, usize)> {
        *visits += 1;
        ensure!(*visits <= 8192 && depth <= 64, "raw token traversal cap");
        let result = match t {
            Token::Number(n, false) => {
                let digits = n
                    .strip_prefix('-')
                    .or_else(|| n.strip_prefix('+'))
                    .unwrap_or(n);
                ensure!(
                    !digits.is_empty()
                        && digits.bytes().all(|b| b.is_ascii_digit())
                        && digits.len() <= 128,
                    "integer literal cap"
                );
                (1, 0, n.len().saturating_mul(4))
            }
            Token::ID(name) => {
                ensure!(
                    name.len() <= 64 && names.iter().any(|n| n == name.as_str()),
                    "undeclared symbol or symbol cap"
                );
                (1, 1, 1)
            }
            Token::Op(_, _, Operator::Neg, args) if args.len() == 1 => {
                estimate(&args[0], names, visits, depth + 1)?
            }
            Token::Op(_, _, Operator::Add, args) | Token::Op(_, _, Operator::Mul, args) => {
                let multiply = matches!(t, Token::Op(_, _, Operator::Mul, _));
                let mut result = (usize::from(multiply), 0u32, 0usize);
                for arg in args {
                    let (terms, degree, height) = estimate(arg, names, visits, depth + 1)?;
                    result.0 = if multiply {
                        result.0.saturating_mul(terms)
                    } else {
                        result.0.saturating_add(terms)
                    };
                    result.1 = if multiply {
                        result.1.saturating_add(degree)
                    } else {
                        result.1.max(degree)
                    };
                    result.2 = result.2.saturating_add(height + 1);
                }
                result
            }
            Token::Op(_, _, Operator::Inv, args) if args.len() == 1 => {
                let (_, degree, height) = estimate(&args[0], names, visits, depth + 1)?;
                ensure!(degree == 0, "denominator must be constant");
                (1, 0, height)
            }
            Token::Op(_, _, Operator::Pow, args) if args.len() == 2 => {
                let Token::Number(exponent, false) = &args[1] else {
                    bail!("exponent must be a nonnegative integer literal")
                };
                let exponent: u32 = exponent.parse().context("raw exponent cap")?;
                ensure!(exponent <= MAX_INPUT_DEGREE, "raw exponent cap");
                let (terms, degree, height) = estimate(&args[0], names, visits, depth + 1)?;
                (
                    terms.saturating_pow(exponent),
                    degree.saturating_mul(exponent),
                    height.saturating_mul(exponent as usize),
                )
            }
            _ => bail!("unsupported polynomial token syntax"),
        };
        ensure!(
            result.0 <= MAX_INPUT_TERMS && result.1 <= MAX_INPUT_DEGREE && result.2 <= 4096,
            "raw expansion or height cap"
        );
        Ok(result)
    }
    estimate(&token, names, &mut 0, 0)?;
    Ok(())
}

fn coefficient(p: &Poly, axis: usize, degree: u32) -> Poly {
    let mut result = p.zero();
    for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
        if e[axis] == degree {
            let mut e = e.to_vec();
            e[axis] = 0;
            result.append_monomial(c.clone(), &e);
        }
    }
    result
}

fn semantic_hash(input: &Input) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(input)?)))
}

fn extract(input: &Input) -> Result<Extracted> {
    ensure!(
        serde_json::to_vec(input)?.len() <= MAX_BYTES,
        "input byte cap"
    );
    let n = input.variables.len();
    ensure!((3..=MAX_VARIABLES).contains(&n), "variable-count cap");
    ensure!(
        input.variables.iter().all(|v| v.len() <= 64),
        "variable name cap"
    );
    ensure!(
        input.variables[n - 3..] == input.axes,
        "axes must be final y,x,z in order"
    );
    ensure!(
        input.constraints.len() <= 32 && input.identities.len() <= 16,
        "input item cap"
    );
    let (y, x, z) = (n - 3, n - 2, n - 1);
    let algebra = Algebra::new(&input.variables)?;
    let coordinates = input
        .variables
        .iter()
        .map(|v| algebra.parse(v))
        .collect::<Result<Vec<_>>>()?;
    let constraints = input
        .constraints
        .iter()
        .map(|p| parse(&algebra, p))
        .collect::<Result<Vec<_>>>()?;
    let mut positive_axes = [false; 3];
    for p in &constraints {
        if (y..n).all(|i| p.degree(i) == 0) {
            continue;
        }
        let mut accepted = false;
        if p.nterms() == 1 && p.coefficients[0] > Rational::zero() {
            for axis in y..n {
                if p == &coordinates[axis]
                    .clone()
                    .mul_coeff(p.coefficients[0].clone())
                {
                    positive_axes[axis - y] = true;
                    accepted = true;
                }
            }
        }
        ensure!(
            accepted,
            "fiber constraints must be original positive coordinate rays"
        );
    }
    ensure!(
        positive_axes.iter().all(|p| *p),
        "missing original positive fiber constraint"
    );

    expression_budget(&input.positive_scale, &[])?;
    let scale = rational(&input.positive_scale)?;
    ensure!(
        scale > Rational::zero(),
        "normalization scale must be strictly positive"
    );
    bounded(&coordinates[0].constant(scale.clone()), 1, 0)?;
    let original = parse(&algebra, &input.polynomial)?;
    let f = original.mul_coeff(Rational::one() / &scale);
    bounded(&f, MAX_INPUT_TERMS, MAX_INPUT_DEGREE)?;
    ensure!(f.degree(z) == 1, "F must be affine in final z");
    let q = coefficient(&f, z, 1);
    let b_poly = coefficient(&f, z, 0);
    ensure!(
        q.degree(x) == 2 && coefficient(&q, x, 2).is_one(),
        "q must be monic quadratic in x"
    );
    ensure!(
        b_poly.degree(x) == 2 && b_poly.degree(y) == 2,
        "B must be quadratic in x and y"
    );
    let hx = parse(&algebra, &input.endpoint_multipliers[0])?;
    let hy = parse(&algebra, &input.endpoint_multipliers[1])?;
    ensure!(
        [&hx, &hy].iter().all(|p| (y..n).all(|i| p.degree(i) == 0)),
        "multipliers must be base-only"
    );
    let mut budget = Budget::default();
    ensure!(
        &budget.multiply(&q, &coordinates[z])? + &b_poly == f,
        "signed original F identity"
    );

    let a = coefficient(&b_poly, x, 2);
    let e = coefficient(&b_poly, x, 1);
    let c = coefficient(&b_poly, x, 0);
    let ay = coefficient(&b_poly, y, 2);
    let ey = coefficient(&b_poly, y, 1);
    let d = coefficient(&b_poly, y, 0);
    let c2 = coefficient(&c, y, 2);
    let c1 = coefficient(&c, y, 1);
    let h0 = coefficient(&c, y, 0);
    let d2 = coefficient(&d, x, 2);
    let d1 = coefficient(&d, x, 1);
    ensure!(coefficient(&d, x, 0) == h0, "double endpoint identity");
    let px = &budget.multiply(&hx, &e)? - &c;
    let py = &budget.multiply(&hy, &ey)? - &d;
    let pc = &budget.multiply(&hy, &c1)? - &h0;
    let pd = &budget.multiply(&hx, &d1)? - &h0;

    let bx = b_poly.derivative(x);
    let by = b_poly.derivative(y);
    let wx = &budget.multiply(&(&coordinates[x] + &hx), &bx)? - &b_poly;
    let wy = &budget.multiply(&(&coordinates[y] + &hy), &by)? - &b_poly;
    let sx = budget.multiply(&coordinates[x], &coordinates[x])?
        + budget
            .multiply(&hx, &coordinates[x])?
            .mul_coeff(Rational::from(2));
    let sy = budget.multiply(&coordinates[y], &coordinates[y])?
        + budget
            .multiply(&hy, &coordinates[y])?
            .mul_coeff(Rational::from(2));
    ensure!(
        wx == &budget.multiply(&a, &sx)? + &px,
        "x normalized derivative identity"
    );
    ensure!(
        wy == &budget.multiply(&ay, &sy)? + &py,
        "y normalized derivative identity"
    );

    let qb = coefficient(&q, x, 1);
    let qc = coefficient(&q, x, 0);
    let j = &e - &budget.multiply(&a, &qb)?;
    let k = &c - &budget.multiply(&a, &qc)?;
    let linear = &budget.multiply(&j, &coordinates[x])? + &k;
    ensure!(
        &b_poly - &budget.multiply(&a, &q)? == linear,
        "linear remainder identity"
    );
    let j2 = budget.multiply(&j, &j)?;
    let jk = budget.multiply(&j, &k)?;
    let r = budget.multiply(&k, &k)? - budget.multiply(&qb, &jk)? + budget.multiply(&qc, &j2)?;
    ensure!(
        exact_resultant(&q, &b_poly, x)? == r,
        "shared resultant identity"
    );
    let qx = q.derivative(x);
    let qy = q.derivative(y);
    let delta = budget.multiply(&qy, &bx)? - budget.multiply(&qx, &by)?;
    let jy = j.derivative(y);
    let ly = linear.derivative(y);
    let aqy = a.derivative(y);
    let delta_remainder = budget.multiply(&j, &qy)? - budget.multiply(&qx, &ly)?;
    let aqy_q = budget.multiply(&aqy, &q)?;
    ensure!(
        &delta - &delta_remainder == -budget.multiply(&aqy_q, &qx)?,
        "Jacobian reduction identity"
    );
    let l_coefficient = ly.mul_coeff(Rational::from(2))
        - budget.multiply(&jy, &qx)?
        - budget.multiply(&j, &qx.derivative(y))?;
    let j_aqy = budget.multiply(&j, &aqy)?;
    let q_coefficient =
        budget.multiply(&j, &jy)?.mul_coeff(Rational::from(2)) + budget.multiply(&j_aqy, &qx)?;
    ensure!(
        r.derivative(y) - budget.multiply(&j, &delta)?
            == budget.multiply(&linear, &l_coefficient)? + budget.multiply(&q, &q_coefficient)?,
        "R_y-J*Delta ideal identity"
    );
    ensure!(
        budget.multiply(&qx, &f.derivative(y))? - budget.multiply(&qy, &f.derivative(x))?
            == -delta.clone(),
        "original fiber critical derivative identity"
    );

    let mut names = std::collections::BTreeSet::new();
    for identity in &input.identities {
        ensure!(
            !identity.name.is_empty() && identity.name.len() <= 128 && names.insert(&identity.name),
            "identity name cap or duplicate"
        );
        ensure!(
            parse(&algebra, &identity.left)? == parse(&algebra, &identity.right)?,
            "extra identity failed: {}",
            identity.name
        );
    }
    let positive = vec![
        ("x_leading", a),
        ("y_leading", ay),
        ("h_x", hx),
        ("h_y", hy),
        ("x_conditional", px),
        ("y_conditional", py),
        ("x_zero_y_leading", c2),
        ("x_zero_conditional", pc),
        ("y_zero_x_leading", d2),
        ("y_zero_conditional", pd),
    ];
    let mut polynomials = BTreeMap::new();
    for (name, p) in [
        ("F", f),
        ("q", q),
        ("B", b_poly),
        ("C", c),
        ("D", d),
        ("H0", h0),
        ("J", j),
        ("K", k),
        ("R", r),
        ("Delta", delta),
        ("x_normalized_derivative", wx),
        ("y_normalized_derivative", wy),
    ]
    .into_iter()
    .chain(positive.iter().map(|(name, p)| (*name, p.clone())))
    {
        bounded(&p, MAX_TERMS, MAX_DEGREE)?;
        polynomials.insert(name.into(), p);
    }
    Ok(Extracted {
        algebra,
        constraints,
        polynomials,
        positive,
    })
}

fn discover(input: &Input) -> Result<Evidence> {
    let e = extract(input)?;
    let domain = derive_domain(&e.algebra, &e.constraints, &[])?;
    ensure!(!domain.empty, "original domain is empty");
    let options = CertificateOptions {
        max_terms: MAX_TERMS,
        polya_order: 0,
        bernstein_depth: 0,
        bernstein_nodes: 1,
        work_limit: 100_000,
        univariate_sturm_degree: 0,
        square_residual_candidates: 0,
        weighted_square_candidates: 0,
        quadratic_component_candidates: 0,
        quadratic_no_real_roots_candidates: 0,
        ..Default::default()
    };
    let mut certificates = Vec::new();
    for (name, p) in &e.positive {
        let certificate = certify_nonzero(&e.algebra, p, &domain, &options)?
            .filter(|c| c.sign == 1)
            .with_context(|| format!("strict positive premise inconclusive: {name}"))?;
        certificates.push(NamedCertificate {
            name: (*name).into(),
            certificate,
        });
    }
    Ok(Evidence {
        schema_version: 1,
        input_sha256: semantic_hash(input)?,
        polynomials: e
            .polynomials
            .iter()
            .map(|(name, p)| (name.clone(), e.algebra.format(p)))
            .collect(),
        certificates,
        checked_identity_names: input.identities.iter().map(|i| i.name.clone()).collect(),
    })
}

fn verify(input: &Input, proof: &Evidence) -> Result<()> {
    ensure!(
        proof.schema_version == 1 && proof.input_sha256 == semantic_hash(input)?,
        "evidence version or original input changed"
    );
    ensure!(
        serde_json::to_vec(proof)?.len() <= 16 * MAX_BYTES,
        "evidence byte cap"
    );
    let e = extract(input)?;
    let expected: BTreeMap<_, _> = e
        .polynomials
        .iter()
        .map(|(name, p)| (name.clone(), e.algebra.format(p)))
        .collect();
    ensure!(
        proof.polynomials == expected,
        "reconstructed polynomials changed"
    );
    ensure!(
        proof.checked_identity_names
            == input
                .identities
                .iter()
                .map(|i| i.name.clone())
                .collect::<Vec<_>>(),
        "extra identity inventory changed"
    );
    ensure!(
        proof.certificates.len() == e.positive.len(),
        "certificate inventory changed"
    );
    for ((name, p), proof) in e.positive.iter().zip(&proof.certificates) {
        ensure!(
            proof.name == *name && proof.certificate.sign == 1,
            "strict certificate identity changed"
        );
        ensure!(
            proof.certificate.polynomial == e.algebra.format(p),
            "certificate polynomial changed"
        );
        verify_certificate(&e.algebra, p, &e.constraints, &[], &proof.certificate)?;
    }
    Ok(())
}

fn save_fresh(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    ensure!(bytes.len() <= 16 * MAX_BYTES, "output byte cap");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(&bytes)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 120.0,
        "runtime limit must be within 120 seconds"
    );
    ensure!(!args.output.exists(), "fresh output required");
    if let Some(parent) = args.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    if !args.worker {
        let sidecar = args.output.with_extension("process.json");
        ensure!(!sidecar.exists(), "fresh process evidence required");
        let mut command = Command::new("taskset");
        command
            .args(["-c", &args.cpu.to_string()])
            .arg(std::env::current_exe()?)
            .args(std::env::args_os().skip(1))
            .arg("--worker")
            .env("RAYON_NUM_THREADS", "1")
            .env("OMP_NUM_THREADS", "1");
        let outcome = symgcad::benchmark::run_bounded(
            &mut command,
            &symgcad::Limits {
                wall_time_secs: args.seconds,
                memory_mib: 2048,
                workers: 1,
                max_cells: None,
            },
        )?;
        save_fresh(&sidecar, &outcome)?;
        ensure!(
            outcome.status == "completed" && !outcome.output_truncated && args.output.is_file(),
            "bounded replay incomplete; process evidence retained"
        );
        println!("bounded research replay completed; no CAD coverage claim");
        return Ok(());
    }
    ensure!(
        fs::metadata(&args.input)?.len() <= MAX_BYTES as u64,
        "input byte cap"
    );
    let bytes = fs::read(&args.input)?;
    let input: Input = serde_json::from_slice(&bytes)?;
    let start = Instant::now();
    let proof = discover(&input)?;
    let discovery_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let serialized = serde_json::to_vec(&proof)?;
    let restored: Evidence = serde_json::from_slice(&serialized)?;
    verify(&input, &restored)?;
    let replay_seconds = start.elapsed().as_secs_f64();
    let report = serde_json::json!({
        "status": "research_identities_and_premises_replayed",
        "coverage_claim": false,
        "input_file_sha256": format!("{:x}", Sha256::digest(&bytes)),
        "discovery_seconds": discovery_seconds, "serialization_replay_seconds": replay_seconds,
        "evidence": restored,
    });
    save_fresh(&args.output, &report)?;
    println!(
        "research identities and 10 strict premises replayed; no root topology or CAD coverage claim"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn family_one() -> Input {
        Input {
            variables: ["v", "y", "x", "z"].map(String::from).to_vec(),
            constraints: ["v", "y", "x", "z"].map(String::from).to_vec(),
            polynomial: "(3/7)*((x^2+(1-v*y)*x+y^2+1-v)*z+(x+1)*(y+2)*(x+y+3)-v*(y+2)-3*v*(x+1))"
                .into(),
            axes: ["y", "x", "z"].map(String::from),
            positive_scale: "3/7".into(),
            endpoint_multipliers: ["1", "2"].map(String::from),
            identities: Vec::new(),
        }
    }

    #[test]
    fn unrelated_families_replay_existing_certificates() -> Result<()> {
        let first = family_one();
        let mut second = first.clone();
        second.positive_scale = "1".into();
        second.constraints[0] = "v-2".into();
        second.endpoint_multipliers = ["v+1", "2*v+3"].map(String::from);
        second.polynomial = "(x^2+(v-2*y)*x+y^2-v)*z+2*((x+v+1)*(y+2*v+3)*(3*x+5*y+7*v+11)-v*(y+2*v+3)-v^2*(x+v+1))".into();
        for input in [first, second] {
            let proof = discover(&input)?;
            let restored = serde_json::from_slice(&serde_json::to_vec(&proof)?)?;
            verify(&input, &restored)?;
            assert_eq!(proof.certificates.len(), 10);
        }
        Ok(())
    }

    #[test]
    fn shared_remainder_can_vanish_without_division() -> Result<()> {
        let mut input = family_one();
        input.positive_scale = "1".into();
        input.polynomial =
            "(x^2+(5-v)*x+4-2*v+(y-1)*(x+y))*z+(x+1)*(y+2)*(x+y+3)-v*(y+2)-3*v*(x+1)".into();
        let e = extract(&input)?;
        for name in ["J", "K"] {
            assert!(
                e.polynomials[name]
                    .replace_with_poly(1, &e.algebra.parse("1")?)
                    .is_zero()
            );
        }
        verify(&input, &discover(&input)?)
    }

    #[test]
    fn evidence_and_input_mutations_rejected() -> Result<()> {
        let input = family_one();
        let proof = discover(&input)?;
        let mut changed = proof.clone();
        changed.polynomials.insert("R".into(), "1".into());
        assert!(verify(&input, &changed).is_err());
        let mut changed = proof.clone();
        changed.certificates.pop();
        assert!(verify(&input, &changed).is_err());
        let mut changed = proof.clone();
        changed.certificates[0].certificate.sign = -1;
        assert!(verify(&input, &changed).is_err());
        let mut changed = proof.clone();
        changed.certificates[0].certificate.polynomial = "1".into();
        assert!(verify(&input, &changed).is_err());
        let mut changed_input = input.clone();
        changed_input.constraints[0] = "v-1".into();
        assert!(verify(&changed_input, &proof).is_err());
        Ok(())
    }

    #[test]
    fn invalid_theorem_scope_rejected() {
        for change in 0..8 {
            let mut input = family_one();
            match change {
                0 => {
                    input.constraints.pop();
                }
                1 => input.constraints.push("x+y".into()),
                2 => input.endpoint_multipliers[0] = "x+1".into(),
                3 => input.positive_scale = "-1".into(),
                4 => input.axes.swap(0, 1),
                5 => input.polynomial = "x^2*z^2+x^2+y^2+1".into(),
                6 => input.polynomial = "x^2*z+x^2-3*x+y^2+1".into(),
                7 => input.polynomial = "x^9*z+x^2+y^2+1".into(),
                _ => unreachable!(),
            }
            assert!(discover(&input).is_err(), "mutation {change}");
        }
    }

    #[test]
    fn extra_identity_requires_exact_reconstruction() -> Result<()> {
        let mut input = family_one();
        input.identities.push(Identity {
            name: "ordinary-square".into(),
            left: "(x+y)^2".into(),
            right: "x^2+2*x*y+y^2".into(),
        });
        verify(&input, &discover(&input)?)?;
        input.identities[0].right = "x^2+y^2".into();
        assert!(discover(&input).is_err());
        Ok(())
    }

    #[test]
    fn raw_input_bombs_and_report_overwrite_rejected() -> Result<()> {
        let names = ["x".into(), "y".into()];
        for text in [
            "2^1000000000",
            "2^(8^8)",
            "(((2^8)^8)^8)^8",
            "(x+y+1)^8",
            "x^-1",
            "f(x)",
        ] {
            assert!(expression_budget(text, &names).is_err(), "{text}");
        }
        let mut input = family_one();
        input.positive_scale = "2^1000000000".into();
        assert!(discover(&input).is_err());
        let path =
            std::env::temp_dir().join(format!("symgcad-positive-pair-{}.json", std::process::id()));
        ensure!(!path.exists(), "fresh test path required");
        save_fresh(&path, &serde_json::json!({"first": true}))?;
        let original = fs::read(&path)?;
        assert!(save_fresh(&path, &serde_json::json!({"second": true})).is_err());
        assert_eq!(fs::read(&path)?, original);
        fs::remove_file(path)?;
        Ok(())
    }
}
