//! Checked import of fastSecDec's pre-sector U/F bundle.
//!
//! This validates an exact interchange contract, not a graph derivation or a
//! CAD certificate. Native graph/routing metadata remains provenance. No sector
//! decomposition, integration, or implicit physical-domain selection occurs.

use crate::{
    Problem,
    algebra::{Algebra, Poly, rational},
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
use symbolica::prelude::*;

const MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_TERMS: usize = 100_000;
const MAX_DEGREE: u32 = 64;
const MAX_COEFFICIENT_BITS: u64 = 16_384;
const MAX_SPECIALIZATION_WORK: u128 = 64_000_000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    schema: String,
    version: u32,
    stage: String,
    symbols: Vec<ExportSymbol>,
    symbolic: Uf,
    specialized: Uf,
    substitutions: Vec<Substitution>,
    domain: Domain,
    graph: Graph,
    problem: Companion,
    // These fields are retained only in the original bundle, whose SHA256 is
    // reported. They never establish a polynomial identity or a domain fact.
    provenance: serde_json::Value,
    #[serde(default)]
    native_symbolic: Option<serde_json::Value>,
    #[serde(default)]
    limitations: Vec<String>,
    #[serde(default)]
    preprocessing_seconds: Option<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportSymbol {
    name: String,
    native_symbol: String,
    role: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Uf {
    u: SparsePolynomial,
    f: SparsePolynomial,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SparsePolynomial {
    variables: Vec<String>,
    terms: Vec<SparseTerm>,
    expression: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SparseTerm {
    coefficient: String,
    exponents: Vec<u32>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Substitution {
    name: String,
    native_symbol: String,
    value: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Domain {
    source: String,
    analysis: String,
    feynman_positive: Vec<String>,
    kinematic_constraints: Vec<String>,
    u_degree: usize,
    f_degree: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Companion {
    file: String,
    blake3: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Graph {
    loop_count: usize,
    propagators: Vec<Propagator>,
    numerator_contracted: bool,
    stable_dot: String,
    vertices: Vec<u64>,
    loop_edges: Vec<u64>,
    external_edges: Vec<u64>,
    dependent_external_edges: Vec<u64>,
    independent_externals: serde_json::Value,
    dependent_externals: serde_json::Value,
    gram: serde_json::Value,
    scalar_bindings: serde_json::Value,
    measure_multiplier: String,
    #[serde(default)]
    routing: Option<serde_json::Value>,
    #[serde(default)]
    native_numerator: Option<String>,
    #[serde(default)]
    numerator_prefactor: Option<String>,
    #[serde(default)]
    projector: Option<String>,
    #[serde(default)]
    overall_factor: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Propagator {
    edge_id: u64,
    denominator_index: usize,
    parameter: String,
    source: Option<u64>,
    target: Option<u64>,
    particle: String,
    power: u32,
    denominator: String,
}

/// An import summary is deliberately separate from `SolveResult` and cannot
/// claim sign-decomposition coverage or independent verification of a graph.
#[derive(Debug, Serialize)]
pub struct ImportSummary {
    pub schema: &'static str,
    pub version: u32,
    pub purpose: &'static str,
    pub coverage_claimed: bool,
    pub graph_derivation_verified: bool,
    pub bundle_sha256: String,
    pub companion_blake3: String,
    pub loop_count: usize,
    pub variables: Vec<String>,
    pub fixed_parameters: usize,
    pub symbolic_terms: [usize; 2],
    pub specialized_terms: [usize; 2],
}

pub struct CheckedImport {
    pub problem: Problem,
    pub summary: ImportSummary,
}

fn bounded_read(path: &Path) -> Result<Vec<u8>> {
    // Check before opening (a FIFO may otherwise block); on Unix also open
    // nonblocking and check the opened handle to close the replacement race.
    ensure!(
        fs::metadata(path)?.is_file(),
        "import input must be a regular file"
    );
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let mut file = options.open(path)?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= MAX_BYTES as u64,
        "bundle and companion must be regular files of at most {MAX_BYTES} bytes"
    );
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= MAX_BYTES,
        "input grew beyond import byte cap"
    );
    Ok(bytes)
}

fn exact_rational(text: &str) -> Result<Rational> {
    ensure!(
        !text.is_empty() && text.len() <= 4096,
        "rational literal length cap"
    );
    fn integer(s: &str) -> bool {
        let s = s
            .strip_prefix('-')
            .or_else(|| s.strip_prefix('+'))
            .unwrap_or(s);
        !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
    }
    let parts: Vec<_> = text.split('/').collect();
    ensure!(
        (1..=2).contains(&parts.len()) && parts.iter().all(|s| integer(s)),
        "only exact integer or integer/integer literals are accepted"
    );
    if parts.len() == 2 {
        ensure!(
            !parts[1]
                .trim_start_matches(['+', '-'])
                .bytes()
                .all(|b| b == b'0'),
            "zero rational denominator"
        );
    }
    let value = rational(text)?;
    ensure!(
        rational_bits(&value) <= MAX_COEFFICIENT_BITS,
        "rational coefficient height cap"
    );
    Ok(value)
}

fn rational_bits(value: &Rational) -> u64 {
    value.numerator_ref().significant_bits() + value.denominator_ref().significant_bits() + 2
}

fn sparse(
    poly: &SparsePolynomial,
    algebra: &Algebra,
    kinematics: usize,
    degree: usize,
    is_u: bool,
) -> Result<Poly> {
    ensure!(
        poly.variables == algebra.names,
        "sparse variable map differs from declared complete alias map"
    );
    ensure!(
        !poly.terms.is_empty(),
        "degenerate zero U/F is unsupported; no split may be dropped"
    );
    ensure!(poly.terms.len() <= MAX_TERMS, "sparse term cap exceeded");
    let mut result = algebra.parse("0")?;
    let mut expression = String::new();
    let mut previous: Option<&[u32]> = None;
    for term in &poly.terms {
        ensure!(
            term.exponents.len() == algebra.names.len(),
            "sparse exponent-map length mismatch"
        );
        ensure!(
            previous.is_none_or(|p| p < term.exponents.as_slice()),
            "sparse terms must have unique, lexicographically increasing exponents"
        );
        previous = Some(&term.exponents);
        let total: u64 = term.exponents.iter().map(|e| u64::from(*e)).sum();
        ensure!(
            total <= u64::from(MAX_DEGREE),
            "sparse total degree cap exceeded"
        );
        ensure!(
            term.exponents[kinematics..]
                .iter()
                .map(|e| u64::from(*e))
                .sum::<u64>()
                == degree as u64,
            "U/F does not have the declared Feynman homogeneity degree"
        );
        let coefficient = exact_rational(&term.coefficient)?;
        ensure!(!coefficient.is_zero(), "zero sparse coefficient");
        ensure!(
            coefficient.to_string() == term.coefficient,
            "sparse rational must use canonical exact spelling"
        );
        if is_u {
            ensure!(
                !coefficient.is_negative() && term.exponents[..kinematics].iter().all(|e| *e == 0),
                "U must have positive rational coefficients and no kinematic dependence"
            );
        }
        if !expression.is_empty() {
            expression.push('+');
        }
        expression.push_str(&format!("({})", term.coefficient));
        for (name, exponent) in algebra.names.iter().zip(&term.exponents) {
            if *exponent != 0 {
                expression.push_str(&format!("*{name}^{exponent}"));
            }
        }
        ensure!(
            expression.len() <= MAX_BYTES,
            "sparse expression byte cap exceeded"
        );
        result.append_monomial(coefficient, &term.exponents);
    }
    ensure!(
        expression == poly.expression,
        "sparse expression does not exactly match its signed coefficients"
    );
    Ok(result)
}

// The raw support is validated first; specialization never hides a later-axis
// dependency or changes the complete Symbolica variable map.
fn specialize(poly: &Poly, values: &BTreeMap<usize, Rational>) -> Result<Poly> {
    if values.is_empty() {
        return Ok(poly.clone());
    }
    let mut work = 0u128;
    let mut collision_bits = BTreeMap::<Vec<u32>, u128>::new();
    for term in poly {
        let bits = u128::from(rational_bits(term.coefficient))
            + values
                .iter()
                .map(|(i, v)| u128::from(term.exponents[*i]) * u128::from(rational_bits(v)))
                .sum::<u128>();
        ensure!(
            bits <= 65_536,
            "specialized coefficient height estimate exceeded"
        );
        work += bits * (values.len() as u128 + 1);
        ensure!(
            work <= MAX_SPECIALIZATION_WORK,
            "exact specialization work cap exceeded"
        );
        let mut remaining = term.exponents.to_vec();
        for i in values.keys() {
            remaining[*i] = 0;
        }
        // Even pairwise coprime rational denominators cannot create an
        // unbounded intermediate when many monomials coalesce at the point.
        let accumulated = collision_bits.entry(remaining).or_default();
        *accumulated += bits + 1;
        ensure!(
            *accumulated <= 65_536,
            "specialization collision height cap exceeded"
        );
    }
    let mut powers = BTreeMap::new();
    for (i, value) in values {
        let mut axis = vec![Rational::one()];
        for e in 1..=poly.degree(*i) as usize {
            axis.push(&axis[e - 1] * value);
        }
        powers.insert(*i, axis);
    }
    let mut terms: BTreeMap<Vec<u32>, Rational> = BTreeMap::new();
    for term in poly {
        let mut coefficient = term.coefficient.clone();
        let mut exponents = term.exponents.to_vec();
        for (i, powers) in &powers {
            coefficient *= &powers[exponents[*i] as usize];
            exponents[*i] = 0;
        }
        *terms.entry(exponents).or_insert_with(Rational::zero) += coefficient;
    }
    let mut result = poly.zero();
    for (exponents, coefficient) in terms {
        result.append_monomial(coefficient, &exponents);
    }
    Ok(result)
}

/// Preflight constraint syntax before Symbolica can expand powers or normalize
/// numeric expressions. The estimated expansion and coefficient heights are
/// conservative; a bounded unsupported constraint is rejected explicitly.
fn constraint_polynomial(text: &str, algebra: &Algebra, allowed: &BTreeSet<&str>) -> Result<()> {
    use symbolica::parser::{Operator, ParseSettings, Token};
    ensure!(
        text.len() <= 4096,
        "kinematic constraint expression byte cap"
    );
    let mut depth = 0usize;
    let mut digits = 0usize;
    for b in text.bytes() {
        digits = if b.is_ascii_digit() { digits + 1 } else { 0 };
        ensure!(digits <= 256, "constraint numeric literal cap");
        if b == b'(' {
            depth += 1;
            ensure!(depth <= 32, "constraint nesting cap");
        }
        if b == b')' {
            depth = depth.checked_sub(1).context("unbalanced constraint")?;
        }
    }
    ensure!(depth == 0, "unbalanced constraint");
    let token = Token::parse(
        &text.replace("**", "^"),
        ParseSettings::symbolica().convert_mul_to_atom(false),
    )
    .map_err(|e| anyhow::anyhow!("invalid constraint expression: {e}"))?;
    fn estimate(
        t: &Token,
        allowed: &BTreeSet<&str>,
        visits: &mut usize,
        depth: usize,
    ) -> Result<(usize, u32, usize)> {
        *visits += 1;
        ensure!(*visits <= 4096 && depth <= 64, "constraint token/depth cap");
        let value = match t {
            Token::Number(n, false) => {
                let digits = n
                    .strip_prefix('-')
                    .or_else(|| n.strip_prefix('+'))
                    .unwrap_or(n);
                ensure!(
                    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()),
                    "integer constraint literals only"
                );
                (1, 0, n.len() * 4)
            }
            Token::ID(name) => {
                ensure!(
                    allowed.contains(name.as_str()),
                    "constraint uses undeclared or Feynman alias"
                );
                (1, 1, 1)
            }
            Token::Op(_, _, Operator::Neg, args) if args.len() == 1 => {
                estimate(&args[0], allowed, visits, depth + 1)?
            }
            Token::Op(_, _, Operator::Add, args) | Token::Op(_, _, Operator::Mul, args) => {
                let mul = matches!(t, Token::Op(_, _, Operator::Mul, _));
                let mut value = (usize::from(mul), 0u32, 0usize);
                for a in args {
                    let (n, d, h) = estimate(a, allowed, visits, depth + 1)?;
                    value.0 = if mul {
                        value.0.saturating_mul(n)
                    } else {
                        value.0.saturating_add(n)
                    };
                    value.1 = if mul {
                        value.1.saturating_add(d)
                    } else {
                        value.1.max(d)
                    };
                    value.2 = value.2.saturating_add(h + 1);
                }
                value
            }
            Token::Op(_, _, Operator::Inv, args) if args.len() == 1 => {
                let (_, d, h) = estimate(&args[0], allowed, visits, depth + 1)?;
                ensure!(d == 0, "constraint denominator must be constant");
                (1, 0, h)
            }
            Token::Op(_, _, Operator::Pow, args) if args.len() == 2 => {
                let Token::Number(e, false) = &args[1] else {
                    bail!("constraint exponent must be a nonnegative integer literal")
                };
                let e: u32 = e.parse().context("constraint exponent cap")?;
                ensure!(e <= MAX_DEGREE, "constraint exponent cap");
                let (n, d, h) = estimate(&args[0], allowed, visits, depth + 1)?;
                (
                    n.saturating_pow(e),
                    d.saturating_mul(e),
                    h.saturating_mul(e as usize),
                )
            }
            _ => bail!("unsupported kinematic constraint syntax"),
        };
        ensure!(
            value.0 <= 4096 && value.1 <= MAX_DEGREE && value.2 <= 4096,
            "constraint expansion cap"
        );
        Ok(value)
    }
    estimate(&token, allowed, &mut 0, 0)?;
    algebra.parse(text)?;
    Ok(())
}

/// Validate exact sparse U/F, point substitution, native-order edge bindings,
/// and the raw companion problem. Signed scale is checked before normalization.
/// All four polynomial maps must remain the complete alias map.
pub fn validate(bundle_bytes: &[u8], problem_bytes: &[u8]) -> Result<CheckedImport> {
    ensure!(
        bundle_bytes.len() <= MAX_BYTES && problem_bytes.len() <= MAX_BYTES,
        "import byte cap exceeded"
    );
    let bundle: Bundle =
        serde_json::from_slice(bundle_bytes).context("invalid fastSecDec bundle")?;
    ensure!(
        bundle.schema == "fastsecdec.symanzik"
            && bundle.version == 1
            && bundle.stage == "pre_sector_projective",
        "unsupported fastSecDec schema/version/stage"
    );
    ensure!(
        bundle.problem.file == "problem.toml",
        "unsupported companion reference"
    );
    let digest = blake3::hash(problem_bytes).to_hex().to_string();
    ensure!(
        bundle.problem.blake3 == digest,
        "companion BLAKE3 digest mismatch"
    );
    ensure!(
        !bundle.symbols.is_empty() && bundle.symbols.len() <= 64,
        "alias map size cap"
    );
    let kinematics = bundle
        .symbols
        .iter()
        .take_while(|s| s.role == "kinematic")
        .count();
    let feynman = &bundle.symbols[kinematics..];
    ensure!(
        !feynman.is_empty() && feynman.len() <= 32 && feynman.iter().all(|s| s.role == "feynman"),
        "aliases must list kinematics followed by one to 32 Feynman coordinates"
    );
    let names = bundle
        .symbols
        .iter()
        .map(|s| s.name.clone())
        .collect::<Vec<_>>();
    let mut native = BTreeSet::new();
    for symbol in &bundle.symbols {
        ensure!(
            symbol.name.len() <= 64
                && !symbol.native_symbol.is_empty()
                && symbol.native_symbol.len() <= 4096,
            "alias/native symbol length cap"
        );
        ensure!(
            native.insert(&symbol.native_symbol),
            "duplicate native symbol binding"
        );
    }
    let algebra = Algebra::new(&names)?;
    let feynman_names = feynman.iter().map(|s| s.name.clone()).collect::<Vec<_>>();
    ensure!(
        bundle.domain.source == "projective_simplex"
            && bundle.domain.analysis == "positive_orthant",
        "unsupported projective/domain semantics"
    );
    ensure!(
        bundle.domain.feynman_positive == feynman_names,
        "positive coordinate map mismatch"
    );
    ensure!(
        bundle.graph.loop_count <= 8
            && bundle.domain.u_degree == bundle.graph.loop_count
            && bundle.domain.f_degree == bundle.graph.loop_count + 1,
        "native loop count and U/F degrees disagree"
    );
    ensure!(
        !bundle.graph.numerator_contracted,
        "numerator-contracted export is not pre-sector U/F"
    );
    ensure!(
        bundle.graph.propagators.len() == feynman.len(),
        "propagator/parameter cardinality mismatch"
    );
    let mut edges = BTreeSet::new();
    let vertices = bundle
        .graph
        .vertices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    ensure!(
        vertices.len() == bundle.graph.vertices.len(),
        "duplicate native vertex id"
    );
    for (i, edge) in bundle.graph.propagators.iter().enumerate() {
        ensure!(
            edge.denominator_index == i && edge.parameter == feynman[i].name,
            "native denominator/parameter binding mismatch"
        );
        ensure!(edges.insert(edge.edge_id), "duplicate propagator edge id");
        ensure!(
            edge.power > 0 && !edge.denominator.is_empty(),
            "invalid propagator provenance"
        );
        ensure!(
            edge.source.is_some_and(|v| vertices.contains(&v))
                && edge.target.is_some_and(|v| vertices.contains(&v)),
            "internal propagator endpoint is missing from native vertices"
        );
    }
    let mut values = BTreeMap::new();
    let mut parameters = BTreeMap::new();
    ensure!(
        bundle.substitutions.len() <= kinematics,
        "point substitution cardinality mismatch"
    );
    for value in &bundle.substitutions {
        let i = bundle
            .symbols
            .iter()
            .position(|s| s.name == value.name)
            .context("point uses undeclared alias")?;
        ensure!(
            i < kinematics && value.native_symbol == bundle.symbols[i].native_symbol,
            "point/native binding mismatch or fixed Feynman coordinate"
        );
        let rational = exact_rational(&value.value)?;
        ensure!(
            values.insert(i, rational).is_none(),
            "duplicate point binding"
        );
        parameters.insert(value.name.clone(), value.value.clone());
    }
    let total_terms = [
        &bundle.symbolic.u,
        &bundle.symbolic.f,
        &bundle.specialized.u,
        &bundle.specialized.f,
    ]
    .iter()
    .map(|p| p.terms.len())
    .sum::<usize>();
    ensure!(total_terms <= 200_000, "aggregate sparse term cap exceeded");
    let u = sparse(
        &bundle.symbolic.u,
        &algebra,
        kinematics,
        bundle.domain.u_degree,
        true,
    )?;
    let f = sparse(
        &bundle.symbolic.f,
        &algebra,
        kinematics,
        bundle.domain.f_degree,
        false,
    )?;
    let specialized_u = sparse(
        &bundle.specialized.u,
        &algebra,
        kinematics,
        bundle.domain.u_degree,
        true,
    )?;
    let specialized_f = sparse(
        &bundle.specialized.f,
        &algebra,
        kinematics,
        bundle.domain.f_degree,
        false,
    )?;
    ensure!(
        values
            .keys()
            .all(|i| specialized_u.degree(*i) == 0 && specialized_f.degree(*i) == 0),
        "specialized U/F still depends on a fixed point coordinate"
    );
    ensure!(
        specialize(&u, &values)? == specialized_u,
        "signed U specialization identity failed"
    );
    ensure!(
        specialize(&f, &values)? == specialized_f,
        "signed F specialization identity failed"
    );

    let problem = Problem::from_toml(std::str::from_utf8(problem_bytes)?)?;
    let free = names[..kinematics]
        .iter()
        .filter(|n| !parameters.contains_key(*n))
        .cloned()
        .collect::<Vec<_>>();
    let mut expected_variables = free.clone();
    expected_variables.extend(feynman_names.clone());
    let mut groups = Vec::new();
    if !free.is_empty() {
        groups.push(free);
    }
    groups.push(feynman_names.clone());
    ensure!(
        problem.variables == expected_variables && problem.variable_groups == groups,
        "companion variable order/groups mismatch"
    );
    ensure!(
        problem.parameters == parameters,
        "companion point bindings mismatch"
    );
    ensure!(
        problem.split == [bundle.symbolic.f.expression.clone()],
        "companion signed F split mismatch"
    );
    let mut constraints = feynman_names
        .iter()
        .map(|n| format!("{n} > 0"))
        .collect::<Vec<_>>();
    ensure!(
        bundle.domain.kinematic_constraints.len() <= 64,
        "kinematic constraint count cap"
    );
    let allowed = names[..kinematics]
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for constraint in &bundle.domain.kinematic_constraints {
        ensure!(
            constraint.len() <= 4096 && !constraint.contains('='),
            "expected bounded strict kinematic inequality"
        );
        let parts = constraint.split(['<', '>']).collect::<Vec<_>>();
        ensure!(parts.len() == 2, "expected one strict kinematic inequality");
        for part in parts {
            constraint_polynomial(part, &algebra, &allowed)?;
        }
        constraints.push(constraint.clone());
    }
    ensure!(
        problem.constraints == constraints,
        "companion explicit domain mismatch"
    );
    // Deliberately read (but never interpret as proof) auxiliary provenance.
    let _ = (
        &bundle.provenance,
        &bundle.native_symbolic,
        &bundle.limitations,
        bundle.preprocessing_seconds,
        &bundle.graph.stable_dot,
        &bundle.graph.loop_edges,
        &bundle.graph.external_edges,
        &bundle.graph.dependent_external_edges,
        &bundle.graph.independent_externals,
        &bundle.graph.dependent_externals,
        &bundle.graph.gram,
        &bundle.graph.scalar_bindings,
        &bundle.graph.measure_multiplier,
        &bundle.graph.routing,
        &bundle.graph.native_numerator,
        &bundle.graph.numerator_prefactor,
        &bundle.graph.projector,
        &bundle.graph.overall_factor,
    );
    for edge in &bundle.graph.propagators {
        let _ = &edge.particle;
    }
    let summary = ImportSummary {
        schema: "symgcad.fastsecdec-import",
        version: 1,
        purpose: "graph-bundle import / U/F sign decomposition preparation",
        coverage_claimed: false,
        graph_derivation_verified: false,
        bundle_sha256: format!("{:x}", Sha256::digest(bundle_bytes)),
        companion_blake3: digest,
        loop_count: bundle.graph.loop_count,
        variables: problem.variables.clone(),
        fixed_parameters: values.len(),
        symbolic_terms: [u.nterms(), f.nterms()],
        specialized_terms: [specialized_u.nterms(), specialized_f.nterms()],
    };
    Ok(CheckedImport { problem, summary })
}

/// Validate both regular files and write a new ordinary Problem TOML. The
/// output is never used as a certificate, and an existing file is not replaced.
pub fn import(bundle: &Path, companion: &Path, output: &Path) -> Result<ImportSummary> {
    let checked = validate(&bounded_read(bundle)?, &bounded_read(companion)?)?;
    let text = toml::to_string_pretty(&checked.problem)?;
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let temporary = parent.join(format!(
        ".symgcad-import-{}-{}.tmp",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| -> Result<()> {
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        fs::hard_link(&temporary, output).context("checked import output must be a fresh path")?;
        Ok(())
    })();
    drop(file);
    let _ = fs::remove_file(&temporary);
    result?;
    Ok(checked.summary)
}
