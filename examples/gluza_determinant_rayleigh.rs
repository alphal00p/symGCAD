//! Exact research check of all rank-one determinantal Rayleigh identities.
use anyhow::{Result, ensure};
use clap::Parser;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command, time::Instant};
use symbolica::prelude::*;
use symgcad::{Limits, Problem, algebra::Poly, problem::normalize};

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, hide = true)]
    worker: bool,
}

fn determinant(matrix: &[Vec<Poly>]) -> Poly {
    assert!(!matrix.is_empty() && matrix.len() <= 4);
    if matrix.len() == 1 {
        return matrix[0][0].clone();
    }
    let mut result = matrix[0][0].zero();
    for column in 0..matrix.len() {
        let minor = matrix[1..]
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .filter(|(j, _)| *j != column)
                    .map(|(_, p)| p.clone())
                    .collect()
            })
            .collect::<Vec<_>>();
        let term = &matrix[0][column] * &determinant(&minor);
        result = if column % 2 == 0 {
            &result + &term
        } else {
            &result - &term
        };
    }
    result
}

fn adjugate(matrix: &[Vec<Poly>]) -> Vec<Vec<Poly>> {
    let n = matrix.len();
    (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    let minor = matrix
                        .iter()
                        .enumerate()
                        .filter(|(row, _)| *row != j)
                        .map(|(_, row)| {
                            row.iter()
                                .enumerate()
                                .filter(|(column, _)| *column != i)
                                .map(|(_, p)| p.clone())
                                .collect()
                        })
                        .collect::<Vec<_>>();
                    let value = determinant(&minor);
                    if (i + j) % 2 == 0 { value } else { -value }
                })
                .collect()
        })
        .collect()
}

fn coefficient_sign(p: &Poly) -> i8 {
    if !p.is_zero() && p.coefficients.iter().all(|c| c > &Rational::zero()) {
        1
    } else if !p.is_zero() && p.coefficients.iter().all(|c| c < &Rational::zero()) {
        -1
    } else {
        0
    }
}

fn opposite_sign_witnesses(p: &Poly) -> Result<serde_json::Value> {
    ensure!(!p.is_zero() && coefficient_sign(p) == 0);
    ensure!(
        p.exponents_iter().all(|e| e.iter().all(|&x| x <= 1)),
        "the corner dominance argument requires a multiaffine polynomial"
    );
    let mut rows = Vec::new();
    for sign in [-1, 1] {
        let index = p
            .coefficients
            .iter()
            .position(|c| (c > &Rational::zero()) == (sign > 0))
            .unwrap();
        let exponents = p.exponents_iter().nth(index).unwrap();
        let mut found = None;
        for exponent in 1..=12 {
            let large = Rational::from(1i64 << exponent);
            let small = Rational::one() / &large;
            let point = exponents
                .iter()
                .map(|&e| if e == 1 { large.clone() } else { small.clone() })
                .collect::<Vec<_>>();
            let mut value = p.clone();
            for (axis, coordinate) in point.iter().enumerate() {
                value = value.replace(axis, coordinate);
            }
            ensure!(value.is_constant());
            if !value.is_zero() && (value.coefficients[0] > Rational::zero()) == (sign > 0) {
                found = Some(json!({
                    "sign": sign,
                    "point": point.iter().map(ToString::to_string).collect::<Vec<_>>(),
                    "value": value.coefficients[0].to_string(),
                    "exact_sign_verified": true
                }));
                break;
            }
        }
        rows.push(found.ok_or_else(|| {
            anyhow::anyhow!("bounded corner screen did not certify the expected sign")
        })?);
    }
    Ok(json!({"positive_orthant_zero_by_intermediate_value":true,"witnesses":rows}))
}

fn coordinate(p: &Poly) -> bool {
    p.nterms() == 1
        && p.coefficients[0] > Rational::zero()
        && p.exponents_iter()
            .next()
            .is_some_and(|e| e.iter().map(|&x| u64::from(x)).sum::<u64>() == 1)
}

fn generate(args: &Args) -> Result<()> {
    ensure!(!args.output.exists(), "report must be fresh");
    let started = Instant::now();
    let source = fs::read(&args.input)?;
    let problem = Problem::from_toml(std::str::from_utf8(&source)?)?;
    ensure!(problem.variables.len() == 7 && problem.split.is_empty());
    let normalized = normalize(&problem, &problem.variables)?;
    let algebra = &normalized.algebra;
    for axis in 0..algebra.names.len() {
        ensure!(
            normalized
                .positives
                .iter()
                .any(|p| coordinate(p) && p.degree(axis) == 1),
            "strict coefficient-sign claims require every original coordinate positive"
        );
    }
    let targets = normalized
        .positives
        .iter()
        .filter(|p| !coordinate(p))
        .collect::<Vec<_>>();
    ensure!(targets.len() == 1);
    let p = targets[0];
    let routing = [
        ("x0", [1, -1, 0, 0]),
        ("x1", [0, 1, 0, 0]),
        ("x2", [1, 0, -1, 0]),
        ("x4", [0, 0, 1, 0]),
        ("x5", [1, 0, 0, 1]),
        ("x6", [1, -1, 0, 1]),
        ("x7", [0, 0, 1, 1]),
    ];
    let mut matrix = vec![vec![p.zero(); 4]; 4];
    let fixed = [0, 1, -1, 0];
    for i in 0..4 {
        for j in 0..4 {
            matrix[i][j] = p.constant(Rational::from(fixed[i] * fixed[j]));
        }
    }
    matrix[3][3] = p.constant(Rational::from(-1));
    for (name, vector) in routing {
        let x = algebra.parse(name)?;
        for i in 0..4 {
            for j in 0..4 {
                matrix[i][j] =
                    &matrix[i][j] + &x.clone().mul_coeff(Rational::from(vector[i] * vector[j]));
            }
        }
    }
    let det = determinant(&matrix);
    ensure!(
        -det == *p,
        "rank-one determinant does not reconstruct original positive F"
    );
    let adj = adjugate(&matrix);
    let mut rows = Vec::new();
    for (i, (first, vi)) in routing.iter().enumerate() {
        for (second, vj) in &routing[i + 1..] {
            ensure!(started.elapsed().as_secs_f64() < 55.);
            let xi = algebra.names.iter().position(|x| x == first).unwrap();
            let xj = algebra.names.iter().position(|x| x == second).unwrap();
            let fi = p.derivative(xi);
            let fj = p.derivative(xj);
            let fij = fi.derivative(xj);
            let delta = &fi * &fj - p * &fij;
            let mut h = p.zero();
            for row in 0..4 {
                for column in 0..4 {
                    h = &h
                        + &adj[row][column]
                            .clone()
                            .mul_coeff(Rational::from(vi[row] * vj[column]));
                }
            }
            ensure!(delta == h.pow(2), "determinantal Rayleigh identity failed");
            ensure!(h.degree(xi) == 0 && h.degree(xj) == 0);
            let b = fi.replace(xj, &Rational::zero());
            let c = fj.replace(xi, &Rational::zero());
            let d = p
                .replace(xi, &Rational::zero())
                .replace(xj, &Rational::zero());
            ensure!(
                &b * &c - &fij * &d == delta,
                "bilinear coefficient identity failed"
            );
            let endpoint_i = p.replace(xi, &Rational::zero());
            let endpoint_j = p.replace(xj, &Rational::zero());
            ensure!(&fi * &c - &endpoint_i * &fij == delta);
            ensure!(&fj * &b - &endpoint_j * &fij == delta);
            let obstruction = if coefficient_sign(&h) == 0 && !h.is_zero() {
                Some(opposite_sign_witnesses(&h)?)
            } else {
                None
            };
            rows.push(json!({"axes":[first,second],"identity_verified":true,"square":algebra.format(&h),"square_terms":h.nterms(),"square_coefficient_sign":coefficient_sign(&h),"strict_coefficient_certificate":coefficient_sign(&h)!=0,"nonvanishing_obstruction":obstruction,"comparison":algebra.format(&delta),"bilinear_a":algebra.format(&fij),"bilinear_b":algebra.format(&b),"bilinear_c":algebra.format(&c),"bilinear_d":algebra.format(&d),"first_leading":algebra.format(&fi),"first_endpoint":algebra.format(&endpoint_i),"second_leading":algebra.format(&fj),"second_endpoint":algebra.format(&endpoint_j)}));
        }
    }
    let report = json!({"source":args.input,"source_sha256":format!("{:x}",Sha256::digest(&source)),"matrix":matrix.iter().map(|row|row.iter().map(|p|algebra.format(p)).collect::<Vec<_>>()).collect::<Vec<_>>(),"determinant_identity_verified":true,"pairs":rows,"strict_pairs":rows.iter().filter(|r|r["strict_coefficient_certificate"]==true).count(),"elapsed_seconds":started.elapsed().as_secs_f64(),"original_completion":false,"scope":"Exact rank-one determinant and Rayleigh identities only. Mixed-sign square factors are retained as inconclusive nonvanishing proposals, never accepted from sampling."});
    fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    if args.worker {
        return generate(&args);
    }
    ensure!(!args.output.exists(), "report must be fresh");
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg(&args.input)
        .arg("--output")
        .arg(&args.output)
        .arg("--worker");
    let process = symgcad::benchmark::run_bounded(
        &mut command,
        &Limits {
            wall_time_secs: 60.,
            memory_mib: 4096,
            workers: 1,
            max_cells: None,
        },
    )?;
    fs::write(
        args.output.with_extension("process.json"),
        serde_json::to_vec_pretty(&process)?,
    )?;
    ensure!(
        process.status == "completed" && process.exit_code == Some(0),
        "diagnostic stopped: {}",
        process.status
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use symgcad::algebra::Algebra;

    #[test]
    fn two_rank_one_coordinates_have_exact_cofactor_square() -> Result<()> {
        let a = Algebra::new(&["x".into(), "y".into()])?;
        let matrix = vec![
            vec![a.parse("x+1")?, a.parse("1")?],
            vec![a.parse("1")?, a.parse("y+1")?],
        ];
        let p = determinant(&matrix);
        assert_eq!(p, a.parse("x*y+x+y")?);
        let h = &adjugate(&matrix)[0][1];
        assert_eq!(
            &p.derivative(0) * &p.derivative(1) - &p * &p.derivative(0).derivative(1),
            h.pow(2)
        );
        Ok(())
    }

    #[test]
    fn indefinite_rank_one_pencil_still_has_nonnegative_rayleigh_square() -> Result<()> {
        let a = Algebra::new(&["x".into(), "y".into()])?;
        let matrix = vec![
            vec![a.parse("x+y")?, a.parse("y")?],
            vec![a.parse("y")?, a.parse("y-1")?],
        ];
        let p = -determinant(&matrix);
        assert_eq!(p, a.parse("x+y-x*y")?);
        assert_eq!(
            &p.derivative(0) * &p.derivative(1) - &p * &p.derivative(0).derivative(1),
            p.one()
        );
        Ok(())
    }

    #[test]
    fn mixed_multiaffine_coefficients_give_exact_positive_orthant_sign_witnesses() -> Result<()> {
        let a = Algebra::new(&["x".into(), "y".into()])?;
        let report = opposite_sign_witnesses(&a.parse("100*x*y+x-3*y-1")?)?;
        assert_eq!(report["witnesses"].as_array().unwrap().len(), 2);
        assert_eq!(report["positive_orthant_zero_by_intermediate_value"], true);
        assert!(opposite_sign_witnesses(&a.parse("x^2-2*x+1")?).is_err());
        assert!(opposite_sign_witnesses(&a.parse("x+y")?).is_err());
        Ok(())
    }
}
