//! Safe, non-executing import of the upstream corpus.
//!
//! Fixture files are data, not programs to run. Only literal arguments of the
//! top-level `F = ExponentiatedPolynomial(...)` are accepted.
use crate::Problem;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEntry {
    pub id: String,
    pub source: String,
    pub line: usize,
    pub collected: bool,
    pub slow: bool,
    pub intractable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureEntry {
    pub id: String,
    pub source: String,
    pub kind: String,
    pub collected_by: Vec<String>,
    pub notes: Vec<String>,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub schema_version: u32,
    pub upstream_revision: Option<String>,
    pub tests: Vec<TestEntry>,
    pub fixtures: Vec<FixtureEntry>,
    pub collected_count: usize,
    pub disabled_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreEntry {
    pub source: String,
    pub operation: String,
    pub problem: Problem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquivalenceGroup {
    pub representative_index: usize,
    pub alias_indices: Vec<usize>,
    pub identifiers: Vec<String>,
    pub normalized_sha256: String,
}

/// Deduplicate only by exact Symbolica polynomial equality after parameter
/// substitution, preserving variable/order/group/solver semantics. The full
/// canonical key, rather than its hash alone, decides equality. No numerical
/// sampling, guessed topology symmetry, or sign-changing rescaling is used.
pub fn equivalence_groups(problems: &[Problem]) -> Result<Vec<EquivalenceGroup>> {
    let mut groups: Vec<EquivalenceGroup> = Vec::new();
    let mut keys = BTreeMap::new();
    for (i, problem) in problems.iter().enumerate() {
        let normalized = crate::problem::normalize(problem, &problem.variables)?;
        let mut constraints = normalized
            .positives
            .iter()
            .map(|p| normalized.algebra.format(p))
            .collect::<Vec<_>>();
        constraints.sort();
        constraints.dedup();
        let split = normalized
            .split
            .iter()
            .map(|p| normalized.algebra.format(p))
            .collect::<Vec<_>>();
        let key = serde_json::to_string(&json!({
            "variables":problem.variables,"groups":problem.variable_groups,
            "solver":problem.solver,"constraints":constraints,"split":split
        }))?;
        let identifier = problem
            .name
            .clone()
            .unwrap_or_else(|| format!("problem-{i}"));
        if let Some(&group_index) = keys.get(&key) {
            let group: &mut EquivalenceGroup = &mut groups[group_index];
            group.alias_indices.push(i);
            group.identifiers.push(identifier);
        } else {
            let group_index = groups.len();
            groups.push(EquivalenceGroup {
                representative_index: i,
                alias_indices: vec![i],
                identifiers: vec![identifier],
                normalized_sha256: format!("{:x}", Sha256::digest(key.as_bytes())),
            });
            keys.insert(key, group_index);
        }
    }
    Ok(groups)
}

pub fn write_equivalence_manifest(
    problems: &[Problem],
    output: &Path,
) -> Result<Vec<EquivalenceGroup>> {
    let groups = equivalence_groups(problems)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        output,
        serde_json::to_vec_pretty(
            &json!({"schema_version":1,"source_problems":problems.len(),"distinct_problems":groups.len(),"groups":groups}),
        )?,
    )?;
    Ok(groups)
}

fn source_root(root: &Path) -> PathBuf {
    if root.join("gcad/gcad.py").is_file() {
        root.to_owned()
    } else {
        root.join("DO_NOT_PUSH_REFERENCES/gcad")
    }
}

fn sorted_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    files.sort();
    Ok(files)
}

pub fn inventory(root: &Path) -> Result<Inventory> {
    let root = source_root(root);
    let mut tests = Vec::new();
    let mut references: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in sorted_files(&root.join("tests"))? {
        let name = path.file_name().unwrap().to_string_lossy();
        if !name.starts_with("test_") || path.extension().is_none_or(|s| s != "py") {
            continue;
        }
        let text = fs::read_to_string(&path)?;
        let mut slow = false;
        let mut intractable = false;
        let mut current = None;
        for (line, raw) in text.lines().enumerate() {
            if raw.starts_with("@pytest.mark.slow") {
                slow = true;
            }
            if raw.starts_with("@pytest.mark.intractable") {
                intractable = true;
            }
            if let Some(signature) = raw.strip_prefix("def ") {
                let function = signature.split('(').next().unwrap();
                if function.starts_with("test_")
                    || function.starts_with("hanging_test_")
                    || function.starts_with("failing_test_")
                {
                    let id = format!("{name}::{function}");
                    tests.push(TestEntry {
                        id: id.clone(),
                        source: format!("tests/{name}"),
                        line: line + 1,
                        collected: function.starts_with("test_"),
                        slow,
                        intractable,
                    });
                    current = Some(id);
                } else {
                    current = None;
                }
                slow = false;
                intractable = false;
            }
            if let Some(call) = raw.trim().strip_prefix("_run_integral(") {
                if let (Some(id), Ok(Value::String(fixture))) =
                    (&current, parse_literal(call.trim_end_matches(')')))
                {
                    references.entry(fixture).or_default().push(id.clone());
                }
            }
        }
    }
    let mut fixtures = Vec::new();
    for path in sorted_files(&root.join("tests/ZZ_integrals"))? {
        if path.extension().is_none_or(|s| s != "py") {
            continue;
        }
        let id = path.file_stem().unwrap().to_string_lossy().into_owned();
        let source = fs::read_to_string(&path)?;
        let kind = if source
            .lines()
            .any(|l| l.starts_with("F = ExponentiatedPolynomial("))
        {
            "literal_two_loop"
        } else if source.contains("LoopIntegralFromPropagators(") {
            "generated_one_loop"
        } else {
            continue;
        };
        let mut notes = Vec::new();
        if id.starts_with("finiteEx12") {
            notes.push("The source's first `if 'finiteE' in intname` branch shadows finiteEx12; import preserves the branch actually selected, with its identifier retained.".into());
        }
        fixtures.push(FixtureEntry {
            source: format!("tests/ZZ_integrals/{id}.py"),
            collected_by: references.remove(&id).unwrap_or_default(),
            id,
            kind: kind.into(),
            notes,
            sha256: format!("{:x}", Sha256::digest(source.as_bytes())),
        });
    }
    ensure!(
        references.is_empty(),
        "tests reference missing fixtures: {:?}",
        references.keys()
    );
    let upstream_revision = Command::new("git")
        .args(["-C"])
        .arg(&root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());
    Ok(Inventory {
        schema_version: 1,
        upstream_revision,
        collected_count: tests.iter().filter(|t| t.collected).count(),
        disabled_count: tests.iter().filter(|t| !t.collected).count(),
        tests,
        fixtures,
    })
}

/// Import all fixture identifiers without dropping algebraically equivalent
/// variants. Algebraic deduplication belongs to the Symbolica normalization
/// layer, where equivalence can be checked rather than guessed from filenames.
pub fn import_zz(root: &Path, fixed: bool) -> Result<Vec<Problem>> {
    let root = source_root(root);
    inventory(&root)?
        .fixtures
        .into_iter()
        .map(|fixture| {
            let source = fs::read_to_string(root.join(&fixture.source))?;
            let (expression, xs) = if fixture.kind == "literal_two_loop" {
                literal_f(&source).with_context(|| fixture.id.clone())?
            } else {
                one_loop_f(&fixture.id, &source)?
            };
            make_zz_problem(&fixture.id, expression, xs, fixed)
        })
        .collect()
}

fn make_zz_problem(id: &str, expression: String, xs: Vec<String>, fixed: bool) -> Result<Problem> {
    let mut parameters = BTreeMap::from([("mt2", "1"), ("mz2", "5/18")]);
    let mut variables = Vec::new();
    let mut constraints = Vec::new();
    if fixed {
        parameters.insert("s", "10");
        parameters.insert("t", "-3");
    } else {
        variables.push("s".to_owned());
        constraints.push("s > 4".to_owned());
        if identifiers(&expression).any(|x| x == "t") {
            variables.push("t".into());
        }
    }
    let mut groups = Vec::new();
    if !variables.is_empty() {
        groups.push(variables.clone());
    }
    variables.extend(xs.iter().cloned());
    groups.push(xs.clone());
    constraints.extend(xs.iter().map(|x| format!("{x} > 0")));
    serde_json::from_value(json!({
        "name": format!("{id}/{}", if fixed { "fixed" } else { "symbolic" }),
        "variables": variables, "constraints": constraints, "split": [expression],
        "parameters": parameters, "variable_groups": groups,
        "solver": {"ordering": "mods"}
    }))
    .context("construct imported problem")
}

fn identifiers(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty())
}

fn literal_f(source: &str) -> Result<(String, Vec<String>)> {
    let start = source
        .lines()
        .find(|l| l.starts_with("F = ExponentiatedPolynomial("))
        .context("missing literal F assignment")?;
    let fields = named_arguments(start.strip_prefix("F = ExponentiatedPolynomial(").unwrap())?;
    let symbols = fields
        .get("polysymbols")
        .and_then(Value::as_array)
        .context("missing polysymbols")?
        .iter()
        .map(|v| v.as_str().map(str::to_owned).context("non-string symbol"))
        .collect::<Result<Vec<_>>>()?;
    let coeffs = fields
        .get("coeffs")
        .and_then(Value::as_array)
        .context("missing coeffs")?;
    let exponents = fields
        .get("expolist")
        .and_then(Value::as_array)
        .context("missing expolist")?;
    ensure!(
        coeffs.len() == exponents.len(),
        "coefficient/exponent count differs"
    );
    let mut terms = Vec::new();
    for (coefficient, exponents) in coeffs.iter().zip(exponents) {
        let coefficient = match coefficient {
            Value::String(s) => s.replace("**", "^"),
            Value::Number(n) => n.to_string(),
            _ => bail!("unsupported coefficient literal"),
        };
        let exponents = exponents.as_array().context("non-list exponent vector")?;
        ensure!(
            exponents.len() == symbols.len(),
            "wrong exponent-vector length"
        );
        let mut term = format!("({coefficient})");
        for (symbol, exponent) in symbols.iter().zip(exponents) {
            let exponent = exponent
                .as_u64()
                .context("non-integer or negative exponent")?;
            if exponent != 0 {
                ensure!(
                    symbol.starts_with('x'),
                    "unexpected active auxiliary symbol {symbol}"
                );
                term.push_str(&format!("*{symbol}"));
                if exponent != 1 {
                    term.push_str(&format!("^{exponent}"));
                }
            }
        }
        terms.push(term);
    }
    Ok((
        terms.join(" + "),
        symbols.into_iter().filter(|s| s.starts_with('x')).collect(),
    ))
}

/// One-loop F = (sum x)(sum x*m²) - sum_{i<j} x_i*x_j*(q_i-q_j)².
/// The finite corpus has only these literal propagator families. Validate the
/// source spelling before applying the known momentum-invariant table.
fn one_loop_f(id: &str, source: &str) -> Result<(String, Vec<String>)> {
    ensure!(
        source.contains("['k1**2-mt2','(k1+p1)**2-mt2'"),
        "unknown one-loop momentum convention in {id}"
    );
    let powers = id
        .rsplit("__")
        .next()
        .unwrap()
        .split('_')
        .map(str::parse::<i32>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        powers.len() == 4 && powers.iter().all(|p| *p >= 0),
        "unsupported propagator powers in {id}"
    );
    // Match actual Python branch semantics: 'finiteE' also matches finiteEx12.
    let pairs = if id.starts_with("finiteE") {
        ["0", "s", "mz2", "0", "2*mz2-s-t", "mz2"]
    } else if id.starts_with("finiteF") {
        ["0", "t", "mz2", "mz2", "2*mz2-s-t", "0"]
    } else {
        bail!("unsupported generated family {id}");
    };
    let active = (0..4).filter(|i| powers[*i] > 0).collect::<Vec<_>>();
    ensure!(!active.is_empty(), "no active propagators");
    let xs = active.iter().map(|i| format!("x{i}")).collect::<Vec<_>>();
    let mut expression = format!("mt2*({})^2", xs.join("+"));
    let mut k = 0;
    for i in 0..4 {
        for j in i + 1..4 {
            if powers[i] > 0 && powers[j] > 0 && pairs[k] != "0" {
                expression.push_str(&format!("-({})*x{i}*x{j}", pairs[k]));
            }
            k += 1;
        }
    }
    Ok((expression, xs))
}

pub fn export_toml(problems: &[Problem], out: &Path) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(out)?;
    problems
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let name = p
                .name
                .as_deref()
                .unwrap_or("problem")
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect::<String>();
            let path = out.join(format!("{i:04}_{name}.toml"));
            fs::write(&path, toml::to_string_pretty(p)?)?;
            Ok(path)
        })
        .collect()
}

/// Read inputs extracted by the independent reference adapter. Refuse a
/// partially extracted corpus rather than silently dropping failed tests.
pub fn import_core_json(path: &Path) -> Result<Vec<Problem>> {
    Ok(import_core_entries(path)?
        .into_iter()
        .map(|entry| entry.problem)
        .collect())
}

/// Keep operation identity: an ordering-only example must not silently be
/// counted as a completed decomposition benchmark.
pub fn import_core_entries(path: &Path) -> Result<Vec<CoreEntry>> {
    let data: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    ensure!(
        data.get("schema_version").and_then(Value::as_u64) == Some(1),
        "unsupported reference extraction schema"
    );
    let failures = data
        .get("failures")
        .and_then(Value::as_array)
        .context("missing extraction failure report")?;
    ensure!(
        failures.is_empty(),
        "reference extraction has unresolved failures: {failures:?}"
    );
    data.get("entries")
        .and_then(Value::as_array)
        .context("missing extracted entries")?
        .iter()
        .map(|entry| {
            let entry: CoreEntry = serde_json::from_value(entry.clone())?;
            ensure!(
                ["solve", "order", "project"].contains(&entry.operation.as_str()),
                "unknown extracted operation"
            );
            entry.problem.validate()?;
            Ok(entry)
        })
        .collect()
}

fn named_arguments(text: &str) -> Result<BTreeMap<String, Value>> {
    let mut parser = LiteralParser {
        text: text.as_bytes(),
        position: 0,
    };
    let mut fields = BTreeMap::new();
    loop {
        parser.space();
        if parser.peek() == Some(b')') || parser.peek().is_none() {
            break;
        }
        let start = parser.position;
        while parser
            .peek()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            parser.position += 1;
        }
        ensure!(parser.position > start, "expected argument name");
        let name = std::str::from_utf8(&parser.text[start..parser.position])?.to_owned();
        parser.space();
        parser.expect(b'=')?;
        let value = parser.value()?;
        ensure!(
            fields.insert(name, value).is_none(),
            "duplicate named argument"
        );
        parser.space();
        if parser.peek() == Some(b',') {
            parser.position += 1;
        } else {
            ensure!(parser.peek() == Some(b')'), "expected argument separator");
        }
    }
    Ok(fields)
}

fn parse_literal(text: &str) -> Result<Value> {
    let mut parser = LiteralParser {
        text: text.as_bytes(),
        position: 0,
    };
    let value = parser.value()?;
    parser.space();
    ensure!(
        parser.position == parser.text.len(),
        "trailing expression after literal"
    );
    Ok(value)
}

struct LiteralParser<'a> {
    text: &'a [u8],
    position: usize,
}
impl LiteralParser<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.get(self.position).copied()
    }
    fn space(&mut self) {
        while self.peek().is_some_and(|b| b.is_ascii_whitespace()) {
            self.position += 1;
        }
    }
    fn expect(&mut self, b: u8) -> Result<()> {
        ensure!(
            self.peek() == Some(b),
            "expected '{}' at byte {}",
            b as char,
            self.position
        );
        self.position += 1;
        Ok(())
    }
    fn value(&mut self) -> Result<Value> {
        self.space();
        match self.peek().context("missing literal")? {
            b'\'' | b'"' => {
                let quote = self.peek().unwrap();
                self.position += 1;
                let mut result = Vec::new();
                loop {
                    let b = self.peek().context("unterminated string")?;
                    self.position += 1;
                    if b == quote {
                        break;
                    }
                    if b == b'\\' {
                        let escaped = self.peek().context("unterminated escape")?;
                        self.position += 1;
                        result.push(match escaped {
                            b'n' => b'\n',
                            b'r' => b'\r',
                            b't' => b'\t',
                            b'\\' | b'\'' | b'"' => escaped,
                            _ => bail!("unsupported escape"),
                        });
                    } else {
                        result.push(b);
                    }
                }
                Ok(Value::String(String::from_utf8(result)?))
            }
            b'[' | b'(' => {
                let close = if self.peek() == Some(b'[') {
                    b']'
                } else {
                    b')'
                };
                self.position += 1;
                let mut values = Vec::new();
                loop {
                    self.space();
                    if self.peek() == Some(close) {
                        self.position += 1;
                        break;
                    }
                    values.push(self.value()?);
                    self.space();
                    if self.peek() == Some(b',') {
                        self.position += 1;
                    } else {
                        self.expect(close)?;
                        break;
                    }
                }
                Ok(Value::Array(values))
            }
            b'-' | b'+' | b'0'..=b'9' => {
                let start = self.position;
                self.position += 1;
                while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                    self.position += 1;
                }
                let integer =
                    std::str::from_utf8(&self.text[start..self.position])?.parse::<i64>()?;
                Ok(json!(integer))
            }
            _ => bail!(
                "only strings, integer literals, and literal lists are accepted at byte {}",
                self.position
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixture_literals_never_execute() {
        assert!(parse_literal("__import__('os').system('echo unsafe')").is_err());
        assert!(parse_literal("[1, 2] + [3]").is_err());
        assert_eq!(
            parse_literal("['x', [1,-2], 's/2']").unwrap(),
            json!(["x", [1, -2], "s/2"])
        );
    }
    #[test]
    fn sparse_fixture_to_expression() {
        let input = "F = ExponentiatedPolynomial(expolist=[[2,0],[0,0]], coeffs=['mt2', '-s'], polysymbols=['x0','eps'], exponent='-2-eps')";
        let (f, xs) = literal_f(input).unwrap();
        assert_eq!(f, "(mt2)*x0^2 + (-s)");
        assert_eq!(xs, ["x0"]);
    }
    #[test]
    fn local_reference_inventory_and_import_are_complete() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("DO_NOT_PUSH_REFERENCES/gcad");
        if !root.join("tests/test_gcad.py").exists() {
            return;
        }
        let inventory = inventory(&root).unwrap();
        assert_eq!(inventory.collected_count, 177);
        assert_eq!(inventory.disabled_count, 3);
        assert_eq!(inventory.fixtures.len(), 277);
        assert_eq!(
            inventory
                .fixtures
                .iter()
                .filter(|f| !f.collected_by.is_empty())
                .count(),
            115
        );
        for fixed in [false, true] {
            let problems = import_zz(&root, fixed).unwrap();
            assert_eq!(problems.len(), inventory.fixtures.len());
            for problem in &problems {
                problem.validate().unwrap();
            }
        }
    }

    #[test]
    fn equivalence_substitutes_exactly_and_preserves_sign_and_order() {
        let first: Problem = serde_json::from_value(json!({"name":"parameterized", "variables":["x"],"constraints":["a*x > 0"],"parameters":{"a":"2"}})).unwrap();
        let second: Problem = serde_json::from_value(
            json!({"name":"expanded", "variables":["x"],"constraints":["x+x > 0"]}),
        )
        .unwrap();
        let third: Problem = serde_json::from_value(
            json!({"name":"opposite", "variables":["x"],"constraints":["-2*x > 0"]}),
        )
        .unwrap();
        let mut fourth = second.clone();
        fourth.solver.ordering = "mods".into();
        let groups = equivalence_groups(&[first, second, third, fourth]).unwrap();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].alias_indices, [0, 1]);
        assert_eq!(groups[0].identifiers, ["parameterized", "expanded"]);
    }
}
