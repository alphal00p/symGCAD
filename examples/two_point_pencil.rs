//! Bounded research replay of two-point Gram pencils and massless rank-one peels.
//! This exports identities, not a decomposition or a new production rule.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::PathBuf, process::Command, time::Instant};
use symbolica::prelude::*;
use symgcad::{Limits, Problem, algebra::Poly, problem::normalize};

#[derive(Parser)]
struct Args {
    input: PathBuf,
    routing: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, hide = true)]
    worker: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Routing {
    s: String,
    /// One augmented (loop, external) vector per original variable, in order.
    vectors: Vec<Vec<i64>>,
    squared_masses: Vec<String>,
    peel_axes: Vec<usize>,
    /// Optional rational basis isolating these axes into distinct final diagonals.
    #[serde(default)]
    joint_basis: Option<Vec<Vec<String>>>,
}

fn det(m: &[Vec<Poly>]) -> Poly {
    if m.len() == 1 {
        return m[0][0].clone();
    }
    let mut d = m[0][0].zero();
    for j in 0..m.len() {
        let minor = m[1..]
            .iter()
            .map(|r| {
                r.iter()
                    .enumerate()
                    .filter(|(k, _)| *k != j)
                    .map(|(_, p)| p.clone())
                    .collect()
            })
            .collect::<Vec<_>>();
        let t = &m[0][j] * &det(&minor);
        d = if j % 2 == 0 { &d + &t } else { &d - &t };
    }
    d
}

fn coordinate(p: &Poly, axis: usize) -> bool {
    p.nterms() == 1
        && p.coefficients[0] > Rational::zero()
        && p.exponents_iter().next().is_some_and(|e| {
            e.iter()
                .enumerate()
                .all(|(i, &d)| d == u32::from(i == axis))
        })
}

fn congruence(m: &[Vec<Poly>], q: &[Vec<Poly>]) -> Result<Vec<Vec<Poly>>> {
    let size = m.len();
    let qdet = det(q);
    ensure!(!qdet.is_zero(), "singular rational basis");
    let mut transformed = vec![vec![m[0][0].zero(); size]; size];
    for i in 0..size {
        for j in 0..size {
            for k in 0..size {
                for l in 0..size {
                    transformed[i][j] = &transformed[i][j] + &(&q[k][i] * &m[k][l] * &q[l][j]);
                }
            }
        }
    }
    ensure!(
        det(&transformed) == &qdet.pow(2) * &det(m),
        "congruence identity failed"
    );
    Ok(transformed)
}

fn replay(problem: &Problem, routing: &Routing) -> Result<serde_json::Value> {
    ensure!(
        (2..=12).contains(&problem.variables.len()),
        "edge-count cap"
    );
    ensure!(
        routing.vectors.len() == problem.variables.len()
            && routing.squared_masses.len() == problem.variables.len(),
        "edge map mismatch"
    );
    let size = routing.vectors[0].len();
    ensure!((2..=4).contains(&size), "cycle-rank cap");
    ensure!(
        routing
            .vectors
            .iter()
            .all(|v| v.len() == size && v.iter().all(|x| (-16..=16).contains(x))),
        "routing-vector cap"
    );
    ensure!(
        routing.peel_axes.len() <= problem.variables.len(),
        "peel-count cap"
    );
    let n = normalize(problem, &problem.variables)?;
    let a = &n.algebra;
    for axis in 0..a.names.len() {
        ensure!(
            n.positives.iter().any(|p| coordinate(p, axis)),
            "missing strict coordinate bound"
        );
    }
    let other = n
        .positives
        .iter()
        .filter(|p| !(0..a.names.len()).any(|i| coordinate(p, i)))
        .collect::<Vec<_>>();
    let f = match (other.as_slice(), n.split.as_slice()) {
        ([p], []) => *p,
        ([], [p]) => p,
        _ => anyhow::bail!("requires exactly one signed F and no coupled domain restriction"),
    };
    ensure!(
        f.nterms() <= 4096
            && f.exponents_iter()
                .all(|e| e.iter().map(|&d| u64::from(d)).sum::<u64>() <= 8),
        "input polynomial cap"
    );
    let constant = |text: &str| -> Result<Rational> {
        ensure!(
            text.len() <= 128
                && text
                    .bytes()
                    .all(|c| c.is_ascii_digit() || matches!(c, b'/' | b'-' | b'+')),
            "rational literal required"
        );
        let p = a.parse(text)?;
        ensure!(p.is_constant(), "constant required");
        Ok(if p.is_zero() {
            Rational::zero()
        } else {
            p.coefficients[0].clone()
        })
    };
    let s = constant(&routing.s)?;
    ensure!(s > Rational::zero(), "strict positive fixed s required");
    let masses = routing
        .squared_masses
        .iter()
        .map(|m| constant(m))
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        masses.iter().all(|m| m >= &Rational::zero()),
        "negative squared mass"
    );
    let mut pencil = vec![vec![f.zero(); size]; size];
    let mut gram = vec![vec![f.zero(); size - 1]; size - 1];
    let mut constant_gram = gram.clone();
    for (axis, v) in routing.vectors.iter().enumerate() {
        let x = a.parse(&a.names[axis])?;
        for i in 0..size {
            for j in 0..size {
                let c = Rational::from(v[i] * v[j]);
                pencil[i][j] = &pencil[i][j] + &x.clone().mul_coeff(&s * &c);
                if i + 1 < size && j + 1 < size {
                    gram[i][j] = &gram[i][j] + &x.clone().mul_coeff(c.clone());
                    constant_gram[i][j] = &constant_gram[i][j] + &f.constant(c);
                }
            }
        }
        pencil[size - 1][size - 1] =
            &pencil[size - 1][size - 1] - &x.mul_coeff(masses[axis].clone());
    }
    let rank = det(&constant_gram);
    ensure!(
        rank.is_constant() && !rank.is_zero() && rank.coefficients[0] > Rational::zero(),
        "loop Gram vectors do not span"
    );
    let u = det(&gram);
    let mut scale = Rational::one();
    for _ in 0..size - 1 {
        scale *= &s;
    }
    ensure!(
        det(&pencil) == -f.clone().mul_coeff(scale),
        "signed original F mismatch"
    );
    let mut peels = Vec::new();
    for &axis in &routing.peel_axes {
        ensure!(
            axis < a.names.len() && masses[axis].is_zero(),
            "peel must be massless"
        );
        let v = &routing.vectors[axis];
        let pivot = v
            .iter()
            .position(|&c| c != 0)
            .ok_or_else(|| anyhow::anyhow!("zero peel direction"))?;
        // First columns span ker(v^T); the last is the pivot unit vector.
        let mut q = vec![vec![f.zero(); size]; size];
        let mut col = 0;
        for j in 0..size {
            if j == pivot {
                continue;
            }
            q[j][col] = f.one();
            q[pivot][col] = f.constant(-Rational::from(v[j]) / Rational::from(v[pivot]));
            col += 1;
        }
        q[pivot][size - 1] = f.one();
        let transformed = congruence(&pencil, &q)?;
        let gamma = &s * &Rational::from(v[pivot] * v[pivot]);
        for i in 0..size {
            for j in 0..size {
                let derivative = transformed[i][j].derivative(axis);
                let expected = if i + 1 == size && j + 1 == size {
                    f.constant(gamma.clone())
                } else {
                    f.zero()
                };
                ensure!(derivative == expected, "rank-one update isolation failed");
            }
        }
        let h = transformed[..size - 1]
            .iter()
            .map(|r| r[..size - 1].to_vec())
            .collect::<Vec<_>>();
        peels.push(json!({"axis":a.names[axis], "gamma":gamma.to_string(),
            "basis":q.iter().map(|r| r.iter().map(|p| a.format(p)).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "H":h.iter().map(|r| r.iter().map(|p| a.format(p)).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "H_determinant":a.format(&det(&h)), "exact_congruence":true,
            "scope":"Existence of a negative-F positive fiber is equivalent to H positive definite; endpoint switches and the positive-F complement are not solved."}));
    }
    let joint = if let Some(basis) = &routing.joint_basis {
        ensure!(
            basis.len() == size && basis.iter().all(|r| r.len() == size),
            "joint basis shape"
        );
        ensure!(
            !routing.peel_axes.is_empty() && routing.peel_axes.len() < size,
            "joint peel rank"
        );
        let q = basis
            .iter()
            .map(|r| {
                r.iter()
                    .map(|x| Ok(f.constant(constant(x)?)))
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        let transformed = congruence(&pencil, &q)?;
        let retained = size - routing.peel_axes.len();
        let mut gammas = Vec::new();
        for (number, &axis) in routing.peel_axes.iter().enumerate() {
            let diagonal = retained + number;
            let gamma = transformed[diagonal][diagonal].derivative(axis);
            ensure!(
                gamma.is_constant() && !gamma.is_zero() && gamma.coefficients[0] > Rational::zero(),
                "strict positive joint update required"
            );
            for i in 0..size {
                for j in 0..size {
                    let expected = if i == diagonal && j == diagonal {
                        gamma.clone()
                    } else {
                        f.zero()
                    };
                    ensure!(
                        transformed[i][j].derivative(axis) == expected,
                        "joint update is not isolated"
                    );
                }
            }
            gammas.push(a.format(&gamma));
        }
        let h = transformed[..retained]
            .iter()
            .map(|r| r[..retained].to_vec())
            .collect::<Vec<_>>();
        Some(json!({"basis":basis,"gammas":gammas,
            "H":h.iter().map(|r| r.iter().map(|p| a.format(p)).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "H_determinant":a.format(&det(&h)),"exact_congruence":true,
            "scope":"Existence of a negative-F positive joint fiber is equivalent to H positive definite; all finite switches and the positive-F complement remain obligations."}))
    } else {
        None
    };
    Ok(
        json!({"schema":1,"kind":"research_identity_replay", "variables":a.names,
        "F":a.format(f), "U":a.format(&u),"constant_gram_determinant":a.format(&rank),
        "signed_determinant_verified":true,"strict_gram_rank_verified":true,
        "peels":peels,"joint_peel":joint,"original_completion":false}),
    )
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(!args.output.exists(), "output must be fresh");
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    if !args.worker {
        let sidecar = args.output.with_extension("process.json");
        ensure!(!sidecar.exists(), "process sidecar must be fresh");
        let mut command = Command::new(std::env::current_exe()?);
        command
            .arg(&args.input)
            .arg(&args.routing)
            .arg("--output")
            .arg(&args.output)
            .arg("--worker");
        let result = symgcad::benchmark::run_bounded(
            &mut command,
            &Limits {
                wall_time_secs: 60.,
                memory_mib: 2048,
                workers: 1,
                max_cells: None,
            },
        )?;
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(sidecar)?
            .write_all(&serde_json::to_vec_pretty(&result)?)?;
        ensure!(
            result.status == "completed"
                && result.exit_code == Some(0)
                && !result.output_truncated
                && args.output.exists(),
            "bounded replay failed"
        );
        return Ok(());
    }
    let start = Instant::now();
    ensure!(
        fs::metadata(&args.input)?.len() <= 1024 * 1024
            && fs::metadata(&args.routing)?.len() <= 65536,
        "input file cap"
    );
    let original = fs::read(&args.input)?;
    let metadata = fs::read(&args.routing)?;
    let mut report = replay(
        &Problem::from_toml(std::str::from_utf8(&original)?)?,
        &serde_json::from_slice(&metadata)?,
    )?;
    report["input_sha256"] = json!(format!("{:x}", Sha256::digest(&original)));
    report["routing_sha256"] = json!(format!("{:x}", Sha256::digest(&metadata)));
    report["elapsed_seconds"] = json!(start.elapsed().as_secs_f64());
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args.output)?
        .write_all(&serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bubble() -> Result<(Problem, Routing)> {
        Ok((
            Problem::from_toml(
                "variables=['a','b']\nconstraints=['a>0','b>0']\nsplit=['a^2-4*a*b']\n",
            )?,
            Routing {
                s: "5".into(),
                vectors: vec![vec![1, 0], vec![1, 1]],
                squared_masses: vec!["1".into(), "0".into()],
                peel_axes: vec![1],
                joint_basis: None,
            },
        ))
    }
    #[test]
    fn bubble_rank_one_replay() -> Result<()> {
        let (p, r) = bubble()?;
        let v = replay(&p, &r)?;
        assert_eq!(v["peels"].as_array().unwrap().len(), 1);
        Ok(())
    }
    #[test]
    fn rejects_wrong_original_sign() -> Result<()> {
        let (mut p, r) = bubble()?;
        p.split[0] = "-a^2+4*a*b".into();
        assert!(replay(&p, &r).is_err());
        Ok(())
    }
    #[test]
    fn rejects_massive_peel() -> Result<()> {
        let (p, mut r) = bubble()?;
        r.peel_axes = vec![0];
        assert!(replay(&p, &r).is_err());
        Ok(())
    }
    #[test]
    fn rejects_missing_strict_coordinate() -> Result<()> {
        let (mut p, r) = bubble()?;
        p.constraints.pop();
        assert!(replay(&p, &r).is_err());
        Ok(())
    }
    #[test]
    fn rejects_rank_deficiency_and_unphysical_mass() -> Result<()> {
        let (p, mut r) = bubble()?;
        r.vectors = vec![vec![0, 0], vec![0, 1]];
        assert!(replay(&p, &r).is_err());
        let (_, mut r) = bubble()?;
        r.squared_masses[0] = "-1".into();
        assert!(replay(&p, &r).is_err());
        Ok(())
    }
    #[test]
    fn joint_basis_replays_and_rejects_wrong_direction() -> Result<()> {
        let (p, mut r) = bubble()?;
        r.joint_basis = Some(vec![
            vec!["-1".into(), "1".into()],
            vec!["1".into(), "1".into()],
        ]);
        let a = symgcad::algebra::Algebra::new(&p.variables)?;
        let report = replay(&p, &r)?;
        assert_eq!(
            a.parse(report["joint_peel"]["gammas"][0].as_str().unwrap())?,
            a.parse("20")?
        );
        r.joint_basis.as_mut().unwrap()[0][0] = "1".into();
        assert!(replay(&p, &r).is_err());
        Ok(())
    }
}
