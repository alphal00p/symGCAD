//! Conversion of external problems to exact polynomial constraints.
use crate::{
    Problem,
    algebra::{Algebra, Poly, rational},
};
use anyhow::{Result, bail, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub struct NormalizedProblem {
    pub algebra: Algebra,
    pub positives: Vec<Poly>,
    pub split: Vec<Poly>,
}

/// Replace only complete identifier tokens, never substrings of names.
fn substitute(text: &str, parameters: &BTreeMap<String, String>) -> Result<String> {
    let values: BTreeMap<_, _> = parameters
        .iter()
        .map(|(k, v)| Ok((k.as_str(), format!("({})", rational(v)?))))
        .collect::<Result<_>>()?;
    let mut result = String::new();
    let mut chars = text.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        if c.is_ascii_alphabetic() || c == '_' {
            let mut end = start + c.len_utf8();
            while let Some(&(i, next)) = chars.peek() {
                if !next.is_ascii_alphanumeric() && next != '_' {
                    break;
                }
                chars.next();
                end = i + next.len_utf8();
            }
            let name = &text[start..end];
            result.push_str(values.get(name).map(|s| s.as_str()).unwrap_or(name));
        } else {
            result.push(c);
        }
    }
    Ok(result)
}

pub fn normalize(problem: &Problem, order: &[String]) -> Result<NormalizedProblem> {
    problem.validate()?;
    ensure!(
        order.len() == problem.variables.len()
            && order.iter().collect::<BTreeSet<_>>()
                == problem.variables.iter().collect::<BTreeSet<_>>(),
        "normalization order must be a permutation of all declared variables"
    );
    let algebra = Algebra::new(order)?;
    let mut positives = Vec::new();
    for constraint in &problem.constraints {
        let text = substitute(constraint, &problem.parameters)?;
        let text = text.trim();
        let expression = match text {
            "true" => "1".into(),
            "false" => "0".into(),
            _ => {
                ensure!(
                    !text.contains('=') && !text.contains('!'),
                    "only strict inequalities are supported: {constraint}"
                );
                let relations: Vec<_> = text.match_indices(['<', '>']).collect();
                ensure!(
                    relations.len() == 1,
                    "expected one strict inequality: {constraint}"
                );
                let (i, op) = relations[0];
                let (left, right) = (&text[..i], &text[i + 1..]);
                if left.trim().is_empty() || right.trim().is_empty() {
                    bail!("missing side of constraint {constraint}");
                }
                if op == ">" {
                    format!("({left})-({right})")
                } else {
                    format!("({right})-({left})")
                }
            }
        };
        positives.push(positive_primitive(algebra.parse(&expression)?));
    }
    let split = problem
        .split
        .iter()
        .map(|p| {
            Ok(positive_primitive(
                algebra.parse(&substitute(p, &problem.parameters)?)?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(NormalizedProblem {
        algebra,
        positives,
        split,
    })
}

/// Clear rational denominators and content using a strictly positive scale.
/// In particular a negative one-term polynomial must retain its sign.
fn positive_primitive(polynomial: Poly) -> Poly {
    if polynomial.is_zero() {
        return polynomial;
    }
    let positive_content = polynomial.content().abs();
    polynomial.div_coeff(&positive_content)
}
