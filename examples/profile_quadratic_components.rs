//! Research-only exact PSD quadratic components with a strict positive residual.
//! Discovery uses negative cross-term connectivity, never topology identifiers.
#[allow(dead_code)]
#[path = "profile_weighted_squares.rs"]
mod weighted;

use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf, time::Instant};
use symbolica::prelude::*;
use symgcad::{
    algebra::{Algebra, Poly, rational},
    domain::{CertificateOptions, certify_nonzero, derive_domain, verify_certificate},
};

const MAX_VARIABLES: usize = 16;
const MAX_TERMS: usize = 256;
const MAX_HEIGHT: u64 = 512;
const MAX_WORK: usize = 100_000;

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
}
#[derive(Deserialize)]
struct Input {
    variables: Vec<String>,
    constraints: Vec<String>,
    polynomials: Vec<NamedPolynomial>,
}
#[derive(Deserialize)]
struct NamedPolynomial {
    label: String,
    polynomial: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct LinearSquare {
    weight: String,
    coefficients: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Certificate {
    sign: i8,
    shifts: Vec<weighted::Shift>,
    squares: Vec<LinearSquare>,
    residual: String,
}
#[derive(Default, Serialize)]
struct Stats {
    components_attempted: usize,
    matrix_updates: usize,
    positive_pivots: usize,
    zero_pivots: usize,
    indefinite_components: usize,
    budget_exhausted: bool,
}

fn height(c: &Rational) -> u64 {
    c.numerator_ref().significant_bits() as u64 + c.denominator_ref().significant_bits() as u64
}
fn bounded(p: &Poly) -> bool {
    p.nvars() <= MAX_VARIABLES
        && p.nterms() <= MAX_TERMS
        && p.coefficients.iter().all(|c| height(c) <= MAX_HEIGHT)
        && p.exponents_iter().all(|e| e.iter().all(|v| *v <= 32))
}
fn strict(p: &Poly, positive: &[bool]) -> bool {
    !p.is_zero()
        && p.coefficients.iter().all(|c| *c > Rational::zero())
        && p.exponents_iter()
            .all(|e| e.iter().zip(positive).all(|(e, pos)| *e == 0 || *pos))
}

fn replay_square(p: &Poly, square: &LinearSquare, positive: &[bool]) -> Result<Poly> {
    ensure!(
        square.coefficients.len() == p.nvars(),
        "linear square variable map"
    );
    let weight = rational(&square.weight)?;
    ensure!(
        weight > Rational::zero() && height(&weight) <= MAX_HEIGHT,
        "strict rational weight"
    );
    let mut form = p.zero();
    for (i, value) in square.coefficients.iter().enumerate() {
        let c = rational(value)?;
        ensure!(height(&c) <= MAX_HEIGHT, "linear coefficient height");
        if c.is_zero() {
            continue;
        }
        ensure!(positive[i], "unrestricted coordinate in research square");
        let mut powers = vec![0; p.nvars()];
        powers[i] = 1;
        form.append_monomial(c, &powers);
    }
    ensure!(!form.is_zero(), "zero linear form");
    let square = form.pow(2).mul_coeff(weight);
    ensure!(bounded(&square), "square size bound");
    Ok(square)
}

fn verify(a: &Algebra, p: &Poly, constraints: &[Poly], c: &Certificate) -> Result<()> {
    ensure!(
        matches!(c.sign, -1 | 1) && c.squares.len() <= MAX_VARIABLES,
        "certificate bounds"
    );
    let (shifted, shifts, positive) =
        weighted::shift(a, p, constraints)?.context("positive shift unavailable")?;
    ensure!(shifts == c.shifts, "original domain shift binding");
    let residual = a.parse(&c.residual)?;
    ensure!(
        bounded(&residual) && strict(&residual, &positive),
        "strict positive residual required"
    );
    let mut replayed = residual;
    for square in &c.squares {
        replayed = replayed + replay_square(p, square, &positive)?;
        ensure!(bounded(&replayed), "reconstruction size bound");
    }
    ensure!(
        replayed == shifted.mul_coeff(Rational::from(c.sign as i64)),
        "full signed reconstruction"
    );
    Ok(())
}

fn root(parent: &[usize], mut i: usize) -> usize {
    while parent[i] != i {
        i = parent[i];
    }
    i
}
fn quadratic_component_squares(
    p: &Poly,
    stats: &mut Stats,
    work_limit: usize,
) -> Result<Option<Vec<LinearSquare>>> {
    let mut parent: Vec<_> = (0..p.nvars()).collect();
    for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
        if *c >= Rational::zero() || e.iter().sum::<u32>() != 2 {
            continue;
        }
        let active: Vec<_> = e
            .iter()
            .enumerate()
            .filter(|(_, e)| **e == 1)
            .map(|(i, _)| i)
            .collect();
        if let [i, j] = active.as_slice() {
            let r = root(&parent, *i);
            let s = root(&parent, *j);
            parent[s] = r;
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..p.nvars() {
        groups.entry(root(&parent, i)).or_default().push(i);
    }
    let mut squares = Vec::new();
    for axes in groups.values().filter(|axes| axes.len() > 1) {
        stats.components_attempted += 1;
        let n = axes.len();
        let mut matrix = vec![vec![Rational::zero(); n]; n];
        for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
            if e.iter().sum::<u32>() != 2
                || e.iter()
                    .enumerate()
                    .any(|(i, e)| *e > 0 && !axes.contains(&i))
            {
                continue;
            }
            let factors: Vec<_> = axes
                .iter()
                .enumerate()
                .flat_map(|(i, &v)| std::iter::repeat_n(i, e[v] as usize))
                .collect();
            let [i, j] = factors.as_slice() else {
                unreachable!()
            };
            let value = if i == j {
                c.clone()
            } else {
                c.clone() / Rational::from(2)
            };
            matrix[*i][*j] += value.clone();
            if i != j {
                matrix[*j][*i] += value;
            }
        }
        for k in 0..n {
            let pivot = matrix[k][k].clone();
            if pivot < Rational::zero() {
                stats.indefinite_components += 1;
                return Ok(None);
            }
            if pivot.is_zero() {
                if (k..n).any(|i| !matrix[i][k].is_zero()) {
                    stats.indefinite_components += 1;
                    return Ok(None);
                }
                stats.zero_pivots += 1;
                continue;
            }
            stats.positive_pivots += 1;
            let mut coefficients = vec![String::from("0"); p.nvars()];
            coefficients[axes[k]] = "1".into();
            for i in k + 1..n {
                let entry = &matrix[i][k] / &pivot;
                if height(&entry) > MAX_HEIGHT {
                    return Ok(None);
                }
                coefficients[axes[i]] = entry.to_string();
            }
            squares.push(LinearSquare {
                weight: pivot.to_string(),
                coefficients,
            });
            for i in k + 1..n {
                for j in i..n {
                    stats.matrix_updates += 1;
                    if stats.matrix_updates > work_limit {
                        stats.budget_exhausted = true;
                        return Ok(None);
                    }
                    let correction = (&matrix[i][k] * &matrix[j][k]) / pivot.clone();
                    matrix[i][j] -= correction;
                    if height(&matrix[i][j]) > MAX_HEIGHT {
                        return Ok(None);
                    }
                    matrix[j][i] = matrix[i][j].clone();
                }
            }
        }
    }
    Ok(Some(squares))
}

fn discover(
    a: &Algebra,
    p: &Poly,
    constraints: &[Poly],
    work_limit: usize,
) -> Result<(Option<Certificate>, Stats)> {
    let mut stats = Stats::default();
    let Some((shifted, shifts, positive)) = weighted::shift(a, p, constraints)? else {
        return Ok((None, stats));
    };
    for sign in [1i8, -1] {
        let signed = shifted.clone().mul_coeff(Rational::from(sign as i64));
        let Some(squares) = quadratic_component_squares(&signed, &mut stats, work_limit)? else {
            if stats.budget_exhausted {
                break;
            }
            continue;
        };
        let mut residual = signed.clone();
        for square in &squares {
            residual = residual - replay_square(p, square, &positive)?;
        }
        if !bounded(&residual) || !strict(&residual, &positive) {
            continue;
        }
        let c = Certificate {
            sign,
            shifts,
            squares,
            residual: a.format(&residual),
        };
        verify(a, p, constraints, &c)?;
        return Ok((Some(c), stats));
    }
    Ok((None, stats))
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(!args.output.exists(), "fresh output required");
    let bytes = fs::read(&args.input)?;
    ensure!(bytes.len() <= 4 * 1024 * 1024, "input bytes bound");
    let input: Input = serde_json::from_slice(&bytes)?;
    ensure!(
        input.variables.len() <= MAX_VARIABLES && input.polynomials.len() <= 32,
        "input item bound"
    );
    let a = Algebra::new(&input.variables)?;
    let constraints = input
        .constraints
        .iter()
        .map(|s| a.parse(s))
        .collect::<Result<Vec<_>>>()?;
    let domain = derive_domain(&a, &constraints, &[])?;
    let mut rows = Vec::new();
    for item in input.polynomials {
        ensure!(
            item.polynomial.len() <= 65536,
            "polynomial source bytes bound"
        );
        let p = a.parse(&item.polynomial)?;
        let baseline = certify_nonzero(&a, &p, &domain, &CertificateOptions::default())?;
        if let Some(c) = &baseline {
            verify_certificate(&a, &p, &constraints, &[], c)?;
        }
        let start = Instant::now();
        let (certificate, stats) = discover(&a, &p, &constraints, MAX_WORK)?;
        let discovery_seconds = start.elapsed().as_secs_f64();
        let start = Instant::now();
        if let Some(c) = &certificate {
            let roundtrip: Certificate = serde_json::from_slice(&serde_json::to_vec(c)?)?;
            verify(&a, &p, &constraints, &roundtrip)?;
        }
        rows.push(
            json!({"label":item.label,"polynomial":a.format(&p),"current_portfolio":baseline,
            "certificate":certificate,"verified":certificate.is_some(),"stats":stats,
            "discovery_seconds":discovery_seconds,"replay_seconds":start.elapsed().as_secs_f64()}),
        );
    }
    fs::write(
        args.output,
        serde_json::to_vec_pretty(&json!({"schema_version":1,
        "kind":"research_quadratic_component_squares","production_certificate":false,
        "original_completion":false,"input":args.input,"input_sha256":format!("{:x}",Sha256::digest(&bytes)),
        "variables":input.variables,"constraints":input.constraints,
        "limits":{"variables":MAX_VARIABLES,"terms":MAX_TERMS,"rational_height":MAX_HEIGHT,
            "matrix_updates":MAX_WORK,"squares":MAX_VARIABLES},"rows":rows}))?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> Result<(Algebra, Vec<Poly>)> {
        let names = ["x", "y", "z", "w", "h"].map(String::from).to_vec();
        let a = Algebra::new(&names)?;
        let constraints = names
            .iter()
            .map(|s| a.parse(s))
            .collect::<Result<Vec<_>>>()?;
        Ok((a, constraints))
    }
    #[test]
    fn broad_support_psd_components() -> Result<()> {
        let (a, c) = setup()?;
        for text in [
            "(2*x-3*y+z)^2+h*x*y*z+w",
            "(x-y)^2+(y-z)^2+h*x*z+w",
            "-(x-y+z-w)^2-h*x*y",
        ] {
            let p = a.parse(text)?;
            let cert = discover(&a, &p, &c, MAX_WORK)?
                .0
                .context("PSD component proof")?;
            verify(&a, &p, &c, &cert)?;
        }
        Ok(())
    }
    #[test]
    fn rejects_semidefinite_seam_and_indefinite_zero_pivot() -> Result<()> {
        let (a, c) = setup()?;
        assert!(
            discover(&a, &a.parse("(x+y-z-w)^2")?, &c, MAX_WORK)?
                .0
                .is_none()
        );
        assert!(
            discover(&a, &a.parse("-2*x*y+h")?, &c, MAX_WORK)?
                .0
                .is_none()
        );
        assert!(
            discover(&a, &a.parse("x^2+y^2-3*x*y+h")?, &c, MAX_WORK)?
                .0
                .is_none()
        );
        Ok(())
    }
    #[test]
    fn rejects_forged_linear_form_and_unrestricted_residual() -> Result<()> {
        let (a, c) = setup()?;
        let p = a.parse("(x-y+z)^2+h*w")?;
        let proof = discover(&a, &p, &c, MAX_WORK)?.0.context("proof")?;
        let mut wrong = proof.clone();
        wrong.squares[0].coefficients[0] = "2".into();
        assert!(verify(&a, &p, &c, &wrong).is_err());
        let unbounded = vec![a.parse("x")?, a.parse("y")?, a.parse("z")?];
        assert!(verify(&a, &p, &unbounded, &proof).is_err());
        let (proof, stats) = discover(&a, &p, &c, 0)?;
        assert!(proof.is_none() && stats.budget_exhausted);
        Ok(())
    }
}
