//! Bounded research heuristics from literal U/F support; never solves or proves CAD.
use anyhow::{Context, Result, bail, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    Problem,
    algebra::Poly,
    benchmark::{ProcessOutcome, run_bounded},
    config::Limits,
    corpus::{MAX_LITERAL_FIXTURE_BYTES, parse_literal_symanzik},
    problem::normalize,
};

const AXES: usize = 12;
const INPUT_BYTES: usize = 2 * 1024 * 1024;
const OUTPUT_BYTES: usize = 16 * 1024 * 1024;
const WORK: usize = 64 * 1024 * 1024;

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    fixture: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 30.0)]
    seconds: f64,
    #[arg(long, default_value_t = 512)]
    memory_mib: u64,
    #[arg(long, default_value_t = 10)]
    cpu: usize,
    #[arg(long, hide = true)]
    worker: bool,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn file_digest(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    ensure!(
        file.metadata()?.len() <= 512 * 1024 * 1024,
        "executable byte cap"
    );
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn bounded_read(path: &Path, cap: usize) -> Result<Vec<u8>> {
    let file = fs::File::open(path)?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= cap as u64,
        "regular input file byte cap"
    );
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(cap as u64 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= cap, "input grew beyond byte cap");
    Ok(bytes)
}

struct CappedWriter {
    file: fs::File,
    remaining: usize,
}
impl Write for CappedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(std::io::Error::other("report byte cap"));
        }
        self.file.write_all(bytes)?;
        self.remaining -= bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}
fn write_new(path: &Path, value: &impl Serialize) -> Result<()> {
    ensure!(!path.exists(), "refusing existing report");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut writer = CappedWriter {
        file: fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?,
        remaining: OUTPUT_BYTES,
    };
    let result = (|| -> Result<()> {
        serde_json::to_writer_pretty(&mut writer, value)?;
        writer.write_all(b"\n")?;
        writer.file.sync_all()?;
        fs::hard_link(&temp, path)
            .context("publish fresh report without replacing prior evidence")?;
        Ok(())
    })();
    fs::remove_file(&temp)?;
    result
}

/// Bound raw tokens before any normalizing Atom conversion can evaluate powers.
fn expression_budget(text: &str) -> Result<usize> {
    use symbolica::parser::{Operator, ParseSettings, Token};
    ensure!(text.len() <= INPUT_BYTES, "expression byte cap");
    let mut depth = 0usize;
    let mut numeric_run = 0usize;
    let mut operators = 0usize;
    for ch in text.chars() {
        numeric_run = if ch.is_ascii_digit() {
            numeric_run + 1
        } else {
            0
        };
        ensure!(numeric_run <= 512, "numeric literal height cap");
        if matches!(ch, '+' | '-' | '*' | '/' | '^') {
            operators += 1;
            ensure!(operators <= 4096, "expression operator cap");
        }
        if ch == '(' {
            depth += 1;
            ensure!(depth <= 32, "expression nesting cap");
        }
        if ch == ')' {
            depth = depth.checked_sub(1).context("unbalanced expression")?;
        }
    }
    ensure!(depth == 0, "unbalanced expression");
    let token = Token::parse(
        &text.replace("**", "^"),
        ParseSettings::symbolica().convert_mul_to_atom(false),
    )
    .map_err(|e| anyhow::anyhow!("invalid expression: {e}"))?;
    fn exponent(token: &Token) -> Result<i32> {
        fn literal(token: &Token) -> Result<i32> {
            if let Token::Number(n, false) = token {
                let value = n.parse::<i32>().context("exponent cap")?;
                ensure!(value.unsigned_abs() <= 32, "exponent cap");
                Ok(value)
            } else {
                bail!("exponent must be a bounded integer literal; towers are unsupported")
            }
        }
        match token {
            Token::Number(_, false) => literal(token),
            Token::Op(_, _, Operator::Neg, args) if args.len() == 1 => Ok(-literal(&args[0])?),
            _ => bail!("exponent must be a bounded integer literal; towers are unsupported"),
        }
    }
    // Height bounds the numerator coefficient l1 norm and common denominator
    // before cancellation. All arithmetic below is saturating machine arithmetic.
    fn estimate(token: &Token, depth: usize, visits: &mut usize) -> Result<(usize, u32, usize)> {
        *visits += 1;
        ensure!(depth <= 64 && *visits <= 32768, "token traversal cap");
        let value = match token {
            Token::Number(n, false) => {
                let digits = n
                    .strip_prefix('-')
                    .or_else(|| n.strip_prefix('+'))
                    .unwrap_or(n);
                ensure!(
                    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()),
                    "integer literals only"
                );
                ensure!(n.len() <= 512, "numeric literal height cap");
                (1, 0, n.len().saturating_mul(4))
            }
            Token::ID(name) => {
                ensure!(name.len() <= 64, "symbol text cap");
                (1, 1, 1)
            }
            Token::Op(_, _, Operator::Add, args) => {
                let mut count = 0usize;
                let mut degree = 0;
                let mut height = 0usize;
                for term in args {
                    let (n, d, h) = estimate(term, depth + 1, visits)?;
                    count = count.saturating_add(n);
                    degree = degree.max(d);
                    height = height.saturating_add(h);
                }
                (count, degree, height.saturating_add(args.len()))
            }
            Token::Op(_, _, Operator::Mul, args) => {
                let mut count = 1usize;
                let mut degree = 0u32;
                let mut height = 0usize;
                for term in args {
                    let (n, d, h) = estimate(term, depth + 1, visits)?;
                    count = count.saturating_mul(n);
                    degree = degree.saturating_add(d);
                    height = height.saturating_add(h);
                }
                (count, degree, height)
            }
            Token::Op(_, _, Operator::Neg, args) if args.len() == 1 => {
                estimate(&args[0], depth + 1, visits)?
            }
            Token::Op(_, _, Operator::Inv, args) if args.len() == 1 => {
                let (_, degree, height) = estimate(&args[0], depth + 1, visits)?;
                ensure!(degree == 0, "only constant rational denominators");
                (1, 0, height)
            }
            Token::Op(_, _, Operator::Pow, args) if args.len() == 2 => {
                let e = exponent(&args[1])?;
                let (count, degree, height) = estimate(&args[0], depth + 1, visits)?;
                ensure!(e >= 0 || degree == 0, "only constant rational denominators");
                let power = e.unsigned_abs();
                (
                    if e < 0 {
                        1
                    } else {
                        count.saturating_pow(power)
                    },
                    degree.saturating_mul(power),
                    height.saturating_mul(power as usize),
                )
            }
            _ => bail!("functions and nonliteral coefficients are unsupported"),
        };
        ensure!(
            value.0 <= 32768 && value.1 <= 64,
            "polynomial expansion cap"
        );
        ensure!(value.2 <= 4096, "pre-normalization coefficient height cap");
        Ok(value)
    }
    Ok(estimate(&token, 0, &mut 0)?.0)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Feature {
    valuation: u32,
    width: u32,
    interaction: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Subset {
    mask: usize,
    u: Feature,
    f: Feature,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Choice {
    variable: String,
    subset: Subset,
    delta_u: u32,
    delta_f: u32,
    original_position: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Step {
    selected: String,
    candidates: Vec<Choice>,
    reason: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Proposal {
    policy: String,
    lifting_order: Vec<String>,
    seed_index: usize,
    steps: Vec<Step>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Seed {
    lifting_order: Vec<String>,
    policies: Vec<String>,
}

fn support(p: &Poly, axes: &[usize]) -> Result<Vec<Vec<u32>>> {
    ensure!(
        !p.is_zero() && p.nterms() <= 32768,
        "zero polynomial/normalized term cap"
    );
    ensure!(
        p.coefficients
            .iter()
            .all(|c| c.numerator_ref().significant_bits() <= 4096
                && c.denominator_ref().significant_bits() <= 4096),
        "normalized coefficient height cap"
    );
    let support = p
        .exponents_iter()
        .map(|m| axes.iter().map(|i| m[*i]).collect())
        .collect::<BTreeSet<Vec<u32>>>();
    ensure!(support.len() <= 4096, "Feynman support cap");
    Ok(support.into_iter().collect())
}
fn features(support: &[Vec<u32>], mask: usize) -> Feature {
    let mut min = u32::MAX;
    let mut max = 0u32;
    let mut blocks = BTreeMap::<[u32; AXES], u64>::new();
    for exponents in support {
        let mut key = [0; AXES];
        let mut sum = 0;
        for (i, e) in exponents.iter().enumerate() {
            if mask & (1 << i) != 0 {
                key[i] = *e;
                sum += e;
            }
        }
        min = min.min(sum);
        max = max.max(sum);
        *blocks.entry(key).or_default() += 1;
    }
    Feature {
        valuation: min,
        width: max - min,
        interaction: blocks.values().map(|n| n * n).sum(),
    }
}
fn subset_table(u: &[Vec<u32>], f: &[Vec<u32>], axes: usize) -> Result<Vec<Subset>> {
    ensure!((1..=AXES).contains(&axes), "Feynman axis cap 1..12");
    let subsets = 1usize << axes;
    ensure!(
        subsets * (u.len() + f.len()) * axes <= WORK,
        "subset-work cap"
    );
    ensure!(
        u.iter()
            .chain(f)
            .all(|m| m.len() == axes && m.iter().all(|e| *e <= 64)),
        "invalid support shape/degree"
    );
    Ok((0..subsets)
        .map(|mask| Subset {
            mask,
            u: features(u, mask),
            f: features(f, mask),
        })
        .collect())
}

fn proposals(
    problem: &Problem,
    axes: &[usize],
    table: &[Subset],
) -> Result<(Vec<Seed>, Vec<Proposal>)> {
    let axis_index = axes
        .iter()
        .enumerate()
        .map(|(i, a)| (*a, i))
        .collect::<BTreeMap<_, _>>();
    let position = problem
        .variables
        .iter()
        .enumerate()
        .map(|(i, n)| (n.clone(), i))
        .collect::<BTreeMap<_, _>>();
    let groups = if problem.variable_groups.is_empty() {
        vec![problem.variables.clone()]
    } else {
        problem.variable_groups.clone()
    };
    let fixed = !problem.solver.order.is_empty();
    let policies = if fixed {
        vec!["fixed_original"]
    } else {
        vec![
            "delay_valuation",
            "expose_valuation",
            "coefficient_interaction",
        ]
    };
    let mut seeds = Vec::<Seed>::new();
    let mut proposals = Vec::new();
    for policy in policies {
        let mut mask = 0usize;
        let mut elimination = Vec::new();
        let mut steps = Vec::new();
        let work_groups = if fixed {
            vec![problem.solver.order.clone()]
        } else {
            groups.clone()
        };
        for group in work_groups.iter().rev() {
            let mut remaining = group
                .iter()
                .filter(|n| axis_index.contains_key(&position[*n]))
                .cloned()
                .collect::<Vec<_>>();
            for slot in group.iter().rev() {
                if !axis_index.contains_key(&position[slot]) {
                    elimination.push(slot.clone());
                    steps.push(Step {
                        selected: slot.clone(),
                        candidates: vec![],
                        reason: "kinematic slot and relative order retained".into(),
                    });
                    continue;
                }
                let options = if fixed {
                    vec![slot.clone()]
                } else {
                    remaining
                        .iter()
                        .filter(|candidate| {
                            problem.solver.homogeneous_anchor.as_ref() != Some(*candidate)
                                || problem.solver.homogeneous_block.iter().all(|member| {
                                    member == *candidate || elimination.contains(member)
                                })
                        })
                        .cloned()
                        .collect()
                };
                let mut choices = options
                    .into_iter()
                    .map(|variable| {
                        let original_position = position[&variable];
                        let next = mask | (1 << axis_index[&original_position]);
                        Choice {
                            variable,
                            subset: table[next].clone(),
                            delta_u: table[next].u.valuation - table[mask].u.valuation,
                            delta_f: table[next].f.valuation - table[mask].f.valuation,
                            original_position,
                        }
                    })
                    .collect::<Vec<_>>();
                choices.sort_by_key(|c| match policy {
                    "expose_valuation" => (
                        -(c.delta_u as i64),
                        -(c.delta_f as i64),
                        c.subset.f.interaction as i64,
                        c.subset.u.interaction as i64,
                        c.original_position,
                    ),
                    "coefficient_interaction" => (
                        c.subset.f.interaction as i64,
                        c.subset.u.interaction as i64,
                        c.subset.f.width as i64,
                        c.subset.u.width as i64,
                        c.original_position,
                    ),
                    _ => (
                        c.delta_u as i64,
                        c.delta_f as i64,
                        c.subset.f.interaction as i64,
                        c.subset.u.interaction as i64,
                        c.original_position,
                    ),
                });
                let best = choices.first().context("empty legal action set")?;
                mask = best.subset.mask;
                let selected = best.variable.clone();
                remaining.retain(|n| n != &selected);
                elimination.push(selected.clone());
                steps.push(Step {
                    selected,
                    candidates: choices,
                    reason: policy.into(),
                });
            }
        }
        elimination.reverse();
        let mut checked = problem.clone();
        checked.solver.order = elimination.clone();
        checked.validate()?;
        if let Some(anchor) = &problem.solver.homogeneous_anchor {
            let rank = elimination
                .iter()
                .position(|n| n == anchor)
                .context("anchor absent from order")?;
            ensure!(
                problem.solver.homogeneous_block.iter().all(|n| n == anchor
                    || elimination
                        .iter()
                        .position(|v| v == n)
                        .is_some_and(|i| i > rank)),
                "homogeneous anchor order is incompatible"
            );
        }
        let index = if let Some(index) = seeds.iter().position(|s| s.lifting_order == elimination) {
            seeds[index].policies.push(policy.into());
            index
        } else {
            seeds.push(Seed {
                lifting_order: elimination.clone(),
                policies: vec![policy.into()],
            });
            seeds.len() - 1
        };
        proposals.push(Proposal {
            policy: policy.into(),
            lifting_order: elimination,
            seed_index: index,
            steps,
        });
    }
    Ok((seeds, proposals))
}

fn prepare(problem: &Problem, source: &str) -> Result<serde_json::Value> {
    let started = Instant::now();
    problem.validate()?;
    ensure!(
        problem.variables.len() <= 24
            && problem.parameters.len() <= 16
            && problem.constraints.len() <= 64
            && problem.split.len() == 1,
        "problem shape cap; exactly one F split required"
    );
    let literal_u = parse_literal_symanzik(source, "U")?;
    let literal_f = parse_literal_symanzik(source, "F")?;
    ensure!(
        literal_u.variables == literal_f.variables,
        "U/F variable maps differ"
    );
    let names = literal_f.variables.iter().collect::<BTreeSet<_>>();
    ensure!(
        problem.solver.homogeneous_anchor.is_none()
            || problem
                .solver
                .homogeneous_block
                .iter()
                .all(|n| names.contains(n)),
        "research anchor block must use Feynman axes"
    );
    let axes = problem
        .variables
        .iter()
        .enumerate()
        .filter_map(|(i, n)| names.contains(n).then_some(i))
        .collect::<Vec<_>>();
    ensure!(
        (1..=AXES).contains(&axes.len()) && axes.len() == names.len(),
        "Feynman variable map/cap mismatch"
    );
    ensure!(
        problem
            .variables
            .iter()
            .filter(|n| n.len() > 1
                && n.starts_with('x')
                && n[1..].bytes().all(|b| b.is_ascii_digit()))
            .all(|n| names.contains(n)),
        "unmapped Feynman variable"
    );
    for value in problem.parameters.values() {
        ensure!(value.len() <= 512, "parameter text cap");
        expression_budget(value)?;
    }
    for expression in problem
        .split
        .iter()
        .map(String::as_str)
        .chain(problem.constraints.iter().flat_map(|c| c.split(['<', '>'])))
        .chain(literal_u.coefficients.iter().map(String::as_str))
        .chain(literal_f.coefficients.iter().map(String::as_str))
    {
        expression_budget(expression)?;
    }
    for literal in [&literal_u, &literal_f] {
        let mut terms = 0usize;
        for coefficient in &literal.coefficients {
            terms = terms.saturating_add(expression_budget(coefficient)?);
            ensure!(terms <= 32768, "total literal expansion cap");
        }
    }
    let parsed_seconds = started.elapsed().as_secs_f64();
    let original = normalize(problem, &problem.variables)?;
    let mut comparison = problem.clone();
    comparison.constraints.clear();
    comparison.split = vec![literal_u.expression.clone(), literal_f.expression.clone()];
    let normalized = normalize(&comparison, &problem.variables)?;
    ensure!(
        normalized.split[1] == original.split[0],
        "literal F differs from original normalized F/parameters"
    );
    let u = &normalized.split[0];
    let f = &normalized.split[1];
    ensure!(
        !u.is_zero() && u.coefficients.iter().all(|c| c > &Rational::zero()),
        "U must have positive exact coefficients"
    );
    ensure!(
        u.exponents_iter().all(|m| m
            .iter()
            .enumerate()
            .all(|(i, e)| axes.contains(&i) || *e == 0)),
        "U has non-Feynman variable support"
    );
    let us = support(u, &axes)?;
    let fs = support(f, &axes)?;
    ensure!(
        us.iter().all(|m| m.iter().all(|e| *e <= 1))
            && us
                .iter()
                .map(|m| m.iter().sum::<u32>())
                .collect::<BTreeSet<_>>()
                .len()
                == 1,
        "U support is not homogeneous multiaffine"
    );
    let normalized_seconds = started.elapsed().as_secs_f64() - parsed_seconds;
    let table = subset_table(&us, &fs, axes.len())?;
    let (seeds, proposals) = proposals(problem, &axes, &table)?;
    let math = json!({"variables":problem.variables,"parameters":problem.parameters,"constraints":original.positives.iter().map(|p|original.algebra.format(p)).collect::<Vec<_>>(),"split":original.split.iter().map(|p|original.algebra.format(p)).collect::<Vec<_>>(),"groups":problem.variable_groups,"fixed_order":problem.solver.order});
    Ok(
        json!({"schema_version":1,"policy_version":"support-seeds-v1","kind":"untrusted_ordering_heuristics","coverage_credit":false,"normalized_math_sha256":digest(&serde_json::to_vec(&math)?),"binding":math,"identity":{"normalized_f_equal":true,"u_f_literal_maps_equal":true,"literal_symbols":literal_f.symbols,"parameter_map":problem.parameters,"normalized_u_sha256":digest(original.algebra.format(u).as_bytes()),"normalized_f_sha256":digest(original.algebra.format(f).as_bytes())},"feynman_axes":axes.iter().map(|i|&problem.variables[*i]).collect::<Vec<_>>(),"support":{"u":us,"f":fs,"normalized_u_terms":u.nterms(),"normalized_f_terms":f.nterms(),"literal_u_terms":literal_u.exponents.len(),"literal_f_terms":literal_f.exponents.len(),"subsets":table},"seeds":seeds,"proposals":proposals,"metrics":{"parse_preflight_seconds":parsed_seconds,"normalize_identity_seconds":normalized_seconds,"total_preprocessing_seconds":started.elapsed().as_secs_f64()},"limitations":"No graph connectivity is inferred, no solve is run, and no support score proves a cost bound or permits projection omission. Kinematic coefficient cancellation is resolved only for the exact supplied parameter map; symbolic coefficient-zero loci are not stratified."}),
    )
}

fn worker_completed(outcome: &ProcessOutcome, output: &Path) -> bool {
    outcome.status == "completed"
        && outcome.exit_code == Some(0)
        && !outcome.output_truncated
        && output.is_file()
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds.is_finite()
            && args.seconds > 0.0
            && args.seconds <= 60.0
            && (256..=2048).contains(&args.memory_mib),
        "research worker bounds"
    );
    ensure!(
        !args.output.exists() && !args.output.with_extension("process.json").exists(),
        "refusing existing output"
    );
    if !args.worker {
        let mut command = Command::new("taskset");
        command
            .args(["-c", &args.cpu.to_string()])
            .arg(std::env::current_exe()?)
            .args(std::env::args_os().skip(1))
            .arg("--worker")
            .env("RAYON_NUM_THREADS", "1")
            .env("OMP_NUM_THREADS", "1");
        let outcome = run_bounded(
            &mut command,
            &Limits {
                wall_time_secs: args.seconds,
                memory_mib: args.memory_mib,
                workers: 1,
                max_cells: None,
            },
        )?;
        write_new(&args.output.with_extension("process.json"), &outcome)?;
        ensure!(
            worker_completed(&outcome, &args.output),
            "support worker did not finish successfully: {}",
            outcome.stderr
        );
        println!("{}", args.output.display());
        return Ok(());
    }
    let worker_started = Instant::now();
    let input = bounded_read(&args.input, INPUT_BYTES)?;
    let fixture = bounded_read(&args.fixture, MAX_LITERAL_FIXTURE_BYTES)?;
    let problem = Problem::from_toml(std::str::from_utf8(&input)?)?;
    let mut report = prepare(&problem, std::str::from_utf8(&fixture)?)?;
    report["source"] = json!({"input":args.input,"input_sha256":digest(&input),"fixture":args.fixture,"fixture_sha256":digest(&fixture),"executable_sha256":file_digest(&std::env::current_exe()?)?});
    report["worker_limits"] = json!({"wall_time_secs":args.seconds,"memory_mib":args.memory_mib,"cpu":args.cpu,"workers":1});
    report["metrics"]["worker_seconds_before_publication"] =
        json!(worker_started.elapsed().as_secs_f64());
    write_new(&args.output, &report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> &'static str {
        "U = ExponentiatedPolynomial(expolist=[[1,0,0],[0,1,0]], coeffs=['1','1'], polysymbols=['x0','x1','eps'], exponent='-2')\nF = ExponentiatedPolynomial(expolist=[[1,0,0],[0,1,0],[1,1,0]], coeffs=['a','1','s'], polysymbols=['x0','x1','eps'], exponent='-1')"
    }
    fn problem() -> Problem {
        serde_json::from_value(json!({"variables":["s","x0","x1"],"parameters":{"a":"2"},"constraints":["s>4","x0>0","x1>0"],"split":["2*x0+x1+s*x0*x1"],"variable_groups":[["s"],["x0","x1"]]})).unwrap()
    }
    #[test]
    fn deterministic_exact_profiles_and_order_groups() {
        let a = prepare(&problem(), fixture()).unwrap();
        let b = prepare(&problem(), fixture()).unwrap();
        assert_eq!(a["support"], b["support"]);
        assert_eq!(a["proposals"], b["proposals"]);
        for seed in a["seeds"].as_array().unwrap() {
            assert_eq!(seed["lifting_order"][0], "s");
        }
        assert_eq!(a["support"]["subsets"][0]["f"]["interaction"], 9);
        assert_eq!(a["support"]["subsets"][3]["f"]["interaction"], 3);
        assert_eq!(a["proposals"][0]["steps"][0]["selected"], "x0");
    }
    #[test]
    fn fixed_order_is_preserved_and_mixed_slots_stay_put() {
        let mut p = problem();
        p.solver.order = vec!["s".into(), "x1".into(), "x0".into()];
        let a = prepare(&p, fixture()).unwrap();
        assert_eq!(a["seeds"].as_array().unwrap().len(), 1);
        assert_eq!(a["seeds"][0]["lifting_order"], json!(p.solver.order));
        p.solver.order.clear();
        p.variable_groups.clear();
        p.variables = vec!["x0".into(), "s".into(), "x1".into()];
        for seed in prepare(&p, fixture()).unwrap()["seeds"].as_array().unwrap() {
            assert_eq!(seed["lifting_order"][1], "s");
        }
    }
    #[test]
    fn rejects_wrong_f_or_map_and_generated_source() {
        let mut p = problem();
        p.split = vec!["2*x0+x1-s*x0*x1".into()];
        assert!(prepare(&p, fixture()).is_err());
        assert!(
            prepare(
                &problem(),
                &fixture().replacen("['x0','x1','eps']", "['x0','x2','eps']", 1)
            )
            .is_err()
        );
        assert!(prepare(&problem(), "li = LoopIntegralFromPropagators() ").is_err());
    }
    #[test]
    fn exact_parameter_cancellation_changes_support() {
        let mut p = problem();
        p.split = vec!["x1+s*x0*x1".into()];
        let report = prepare(
            &p,
            &fixture().replace("coeffs=['a','1','s']", "coeffs=['a-2','1','s']"),
        )
        .unwrap();
        assert_eq!(report["support"]["f"].as_array().unwrap().len(), 2);
        assert_eq!(report["support"]["subsets"][2]["f"]["valuation"], 1);
    }
    #[test]
    fn requested_homogeneous_anchor_precedes_its_ratios() {
        let mut p = problem();
        p.solver.homogeneous_anchor = Some("x0".into());
        p.solver.homogeneous_block = vec!["x0".into(), "x1".into()];
        let result = prepare(&p, fixture()).unwrap();
        for seed in result["seeds"].as_array().unwrap() {
            assert_eq!(seed["lifting_order"], json!(["s", "x0", "x1"]));
        }
        p.solver.order = vec!["s".into(), "x1".into(), "x0".into()];
        assert!(prepare(&p, fixture()).is_err());
    }
    #[test]
    fn caps_precede_large_subset_or_expansion_allocation() {
        assert!(subset_table(&[vec![0; 13]], &[vec![1; 13]], 13).is_err());
        assert!(expression_budget("(x0+x1)^32").is_err());
        assert!(expression_budget("x0^1000000000").is_err());
        assert!(expression_budget("2^1000000000").is_err());
        assert!(expression_budget("2^(2^30)").is_err());
        assert!(expression_budget("((2^32)^32)^32").is_err());
        assert!(expression_budget("2^(1/2)").is_err());
        assert!(expression_budget("(x0+x1)^4/18").is_ok());
        let mut p = problem();
        p.parameters.insert("a".into(), "2^1000000000".into());
        assert!(prepare(&p, fixture()).is_err());
        assert!(parse_literal_symanzik(&" ".repeat(MAX_LITERAL_FIXTURE_BYTES + 1), "F").is_err());
        assert!(
            parse_literal_symanzik(
                "F = ExponentiatedPolynomial(expolist=__import__('os'), coeffs=[], polysymbols=[])",
                "F"
            )
            .is_err()
        );
    }
    #[test]
    fn read_and_report_caps_and_history_preservation() {
        let dir = std::env::temp_dir().join(format!("support-seeds-tests-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let input = dir.join("input");
        fs::write(&input, b"12345").unwrap();
        assert!(bounded_read(&input, 4).is_err());
        let report = dir.join("report.json");
        let _ = fs::remove_file(&report);
        write_new(&report, &json!({"a":1})).unwrap();
        assert!(write_new(&report, &json!({"a":2})).is_err());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(&report).unwrap()).unwrap()["a"],
            1
        );
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn worker_success_requires_completed_process_and_report() {
        let output = std::env::temp_dir().join(format!(
            "support-seeds-worker-gate-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&output);
        let success = ProcessOutcome {
            status: "completed".into(),
            wall_time_secs: 0.25,
            user_cpu_secs: 0.1,
            system_cpu_secs: 0.1,
            peak_rss_kib: 1024,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            output_truncated: false,
        };
        assert!(!worker_completed(&success, &output));
        fs::write(&output, b"{}").unwrap();
        assert!(worker_completed(&success, &output));
        for status in ["timeout", "failed", "success"] {
            let mut changed = success.clone();
            changed.status = status.into();
            assert!(!worker_completed(&changed, &output));
        }
        let mut changed = success.clone();
        changed.exit_code = Some(1);
        assert!(!worker_completed(&changed, &output));
        changed = success.clone();
        changed.output_truncated = true;
        assert!(!worker_completed(&changed, &output));
        fs::remove_file(output).unwrap();
    }
}
