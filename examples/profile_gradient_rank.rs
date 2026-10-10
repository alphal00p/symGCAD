//! Research-only exact obstruction to reducing active linear coordinates.
//! Full column rank of evaluated gradients certifies that the gradient
//! polynomials have no nonzero constant rational linear relation. Lower rank
//! is only inconclusive; sampled rank never authorizes a substitution.
use anyhow::{Result, ensure};
use clap::Parser;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, time::Instant};
use symbolica::{prelude::*, tensors::matrix::Matrix};
use symgcad::{
    algebra::{Algebra, Poly},
    optimized_projection::OperationCapture,
};

#[derive(Parser)]
struct Args {
    capture: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

fn rank_evidence(p: &Poly) -> Result<serde_json::Value> {
    let active = (0..p.nvars())
        .filter(|&i| p.degree(i) > 0)
        .collect::<Vec<_>>();
    ensure!(
        !active.is_empty() && active.len() <= 8,
        "need1..8 active variables"
    );
    ensure!(
        p.nvars() <= 32 && p.nterms() <= 100_000,
        "research operand cap"
    );
    ensure!(
        active.iter().all(|&i| p.degree(i) <= 128),
        "research degree cap"
    );
    let gradients = active.iter().map(|&i| p.derivative(i)).collect::<Vec<_>>();
    let mut seed = 19u64;
    let mut points = Vec::new();
    let mut entries = Vec::new();
    for _ in 0..16 {
        let mut point = vec![Rational::zero(); p.nvars()];
        for &axis in &active {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            point[axis] = Rational::from(1 + (seed >> 32) % 11);
        }
        entries.extend(gradients.iter().map(|g| g.replace_all(&point)));
        points.push(point);
    }
    let matrix = Matrix::from_linear(entries.clone(), 16, active.len() as u32, Q)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let rank = matrix.rank();
    let mut proportional = Vec::new();
    for i in 0..active.len() {
        for j in i + 1..active.len() {
            let ratio = gradients[i].lcoeff() / gradients[j].lcoeff();
            if gradients[j].clone().mul_coeff(ratio.clone()) == gradients[i] {
                proportional
                    .push(json!({"first":active[i],"second":active[j],"ratio":ratio.to_string()}));
            }
        }
    }
    Ok(
        json!({"active_variables":active,"sample_rows":16,"exact_gradient_rank":rank,
        "constant_linear_reduction_impossible":rank==active.len(),
        "lower_rank_is_only_inconclusive":true,"proportional_gradient_pairs":proportional,
        "points":points.iter().map(|row|row.iter().map(ToString::to_string).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "gradient_matrix":entries.chunks(active.len()).map(|row|row.iter().map(ToString::to_string).collect::<Vec<_>>()).collect::<Vec<_>>() }),
    )
}

fn main() -> Result<()> {
    let args = Args::parse();
    let bytes = fs::read(&args.capture)?;
    let capture: OperationCapture = serde_json::from_slice(&bytes)?;
    ensure!(!capture.operands_omitted_for_size, "capture omits operands");
    let algebra = Algebra::new(&capture.variable_names)?;
    let start = Instant::now();
    let records = capture
        .operands
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("capture lacks operands"))?
        .iter()
        .map(|expression| {
            let p = algebra.parse(expression)?;
            Ok(json!({"terms":p.nterms(),"evidence":rank_evidence(&p)?}))
        })
        .collect::<Result<Vec<_>>>()?;
    fs::write(
        &args.output,
        serde_json::to_vec_pretty(&json!({"capture":args.capture,
        "capture_sha256":format!("{:x}",Sha256::digest(&bytes)),"variables":capture.variable_names,
        "elapsed_seconds":start.elapsed().as_secs_f64(),"records":records,
        "claim":"Exact sampled full rank rules out reducing the number of linear coordinates; it does not rule out invertible shears that improve sparsity."}))?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_rank_is_exact_and_sum_dependency_is_not_overclaimed() {
        let a = Algebra::new(&["s".into(), "t".into(), "x".into()]).unwrap();
        let v = rank_evidence(&a.parse("s^2+t^2+x^2").unwrap()).unwrap();
        assert_eq!(v["exact_gradient_rank"], 3);
        assert_eq!(v["constant_linear_reduction_impossible"], true);
        let v = rank_evidence(&a.parse("(s+t+x)^2").unwrap()).unwrap();
        assert_eq!(v["exact_gradient_rank"], 1);
        assert_eq!(v["constant_linear_reduction_impossible"], false);
        assert_eq!(
            v["proportional_gradient_pairs"].as_array().unwrap().len(),
            3
        );
    }
    #[test]
    fn rational_ratio_and_nonpairwise_dependency() {
        let a = Algebra::new(&["s".into(), "t".into(), "x".into()]).unwrap();
        let v = rank_evidence(&a.parse("2*s-3*t+x^2").unwrap()).unwrap();
        assert_eq!(v["exact_gradient_rank"], 2);
        assert_eq!(v["proportional_gradient_pairs"][0]["ratio"], "-2/3");
        let v = rank_evidence(&a.parse("(s-t)^2+(t-x)^2").unwrap()).unwrap();
        assert_eq!(v["exact_gradient_rank"], 2);
        assert!(
            v["proportional_gradient_pairs"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}
