//! Research-only finite orthant atlas; no original cell certificate is emitted.
use anyhow::{Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fs, path::PathBuf, time::Instant};
use symbolica::prelude::*;
use symgcad::{
    Problem,
    algebra::{Algebra, Poly},
    problem::normalize,
};

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
}
fn positive_coordinate(p: &Poly, variable: usize) -> bool {
    p.nterms() == 1
        && p.coefficients[0] > Rational::zero()
        && p.exponents_iter()
            .next()
            .unwrap()
            .iter()
            .enumerate()
            .all(|(i, &e)| e == u32::from(i == variable))
}
fn coefficient_sign(p: &Poly) -> i8 {
    if p.is_zero() {
        0
    } else if p.coefficients.iter().all(|c| c > &Rational::zero()) {
        1
    } else if p.coefficients.iter().all(|c| c < &Rational::zero()) {
        -1
    } else {
        0
    }
}
/// Bit one selects x=1+y; bit zero selects x=1/(1+y).
fn chart(a: &Algebra, p: &Poly, upper: usize, inverse: bool) -> Result<Poly> {
    let n = a.names.len();
    ensure!(
        n <= 10 && (0..n).all(|i| p.degree(i) <= 1),
        "requires at most ten multiaffine coordinates"
    );
    let axes = a
        .names
        .iter()
        .map(|v| a.parse(v))
        .collect::<Result<Vec<_>>>()?;
    let one = a.parse("1")?;
    let mut result = p.zero();
    for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
        let mut term = one.clone().mul_coeff(c.clone());
        for i in 0..n {
            let degree = p.degree(i);
            let factor = if upper & (1 << i) != 0 {
                if inverse {
                    (&axes[i] - &one).pow(e[i] as usize)
                } else {
                    (&axes[i] + &one).pow(e[i] as usize)
                }
            } else if inverse {
                &(&one - &axes[i]).pow(e[i] as usize) * &axes[i].pow((degree - e[i]) as usize)
            } else {
                (&one + &axes[i]).pow((degree - e[i]) as usize)
            };
            term = &term * &factor;
        }
        result = &result + &term;
    }
    Ok(result)
}
fn best_peels(p: &Poly, removed: usize, memo: &mut HashMap<usize, Vec<usize>>) -> Vec<usize> {
    if let Some(v) = memo.get(&removed) {
        return v.clone();
    }
    let mut best = vec![];
    for i in 0..p.nvars() {
        if removed & (1 << i) != 0 || p.degree(i) != 1 || coefficient_sign(&p.derivative(i)) != 1 {
            continue;
        }
        let endpoint = p.replace(i, &Rational::zero());
        let mut candidate = vec![i];
        candidate.extend(best_peels(&endpoint, removed | (1 << i), memo));
        if candidate.len() > best.len() {
            best = candidate;
        }
    }
    memo.insert(removed, best.clone());
    best
}
fn main() -> Result<()> {
    let args = Args::parse();
    let start = Instant::now();
    ensure!(!args.output.exists(), "use a fresh output directory");
    let bytes = fs::read(&args.input)?;
    let source = Problem::from_toml(std::str::from_utf8(&bytes)?)?;
    ensure!(
        source.split.is_empty() && source.variables.len() <= 10,
        "requires one unsplit sign constraint"
    );
    ensure!(
        source.solver.order.is_empty(),
        "research ranking must not override an explicit order"
    );
    let groups = if source.variable_groups.is_empty() {
        vec![source.variables.clone()]
    } else {
        source.variable_groups.clone()
    };
    ensure!(
        groups.len() == 1,
        "research ranking requires one variable group"
    );
    let normal = normalize(&source, &source.variables)?;
    let a = &normal.algebra;
    let n = source.variables.len();
    for i in 0..n {
        ensure!(
            normal.positives.iter().any(|p| positive_coordinate(p, i)),
            "every axis needs an original positive bound"
        );
    }
    let targets = normal
        .positives
        .iter()
        .filter(|p| !(0..n).any(|i| positive_coordinate(p, i)))
        .collect::<Vec<_>>();
    ensure!(
        targets.len() == 1,
        "requires exactly one non-coordinate constraint"
    );
    let p = targets[0];
    ensure!(
        (0..n).all(|i| p.degree(i) == 1),
        "target must use every axis and be multiaffine"
    );
    fs::create_dir_all(&args.output)?;
    let mut rows = Vec::<Value>::new();
    for mask in 0..(1usize << n) {
        let q = chart(a, p, mask, false)?;
        // Use original degrees, which are all one. If a transformed degree drops,
        // pad the reverse numerator explicitly by the omitted lower-axis power.
        let mut restored = chart(a, &q, mask, true)?;
        for i in 0..n {
            if mask & (1 << i) == 0 && q.degree(i) == 0 {
                restored = &restored * &a.parse(&a.names[i])?;
            }
        }
        ensure!(restored == *p, "exact atlas identity failed for mask{mask}");
        let plus = best_peels(&q, 0, &mut HashMap::new());
        let neg = q.clone().mul_coeff(Rational::from(-1));
        let minus = best_peels(&neg, 0, &mut HashMap::new());
        let (oriented, peels, orientation) = if minus.len() > plus.len() {
            (&neg, minus, -1)
        } else {
            (&q, plus, 1)
        };
        let mut order = (0..n)
            .filter(|i| !peels.contains(i))
            .map(|i| source.variables[i].clone())
            .collect::<Vec<_>>();
        order.extend(peels.iter().rev().map(|&i| source.variables[i].clone()));
        let mut problem = source.clone();
        problem.name = Some(format!(
            "{}/atlas-{mask:03}",
            source.name.as_deref().unwrap_or("unnamed")
        ));
        problem.parameters.clear();
        problem.constraints = source.variables.iter().map(|v| format!("{v}>0")).collect();
        problem.split = vec![a.format(oriented)];
        problem.solver.engine = "monotone".into();
        problem.solver.monotone_base_engine = "local".into();
        problem.solver.ordering = "given".into();
        problem.solver.order = order.clone();
        problem.solver.positivity = true;
        problem.solver.local_adjacent_pairs = true;
        problem.solver.local_batch_siblings = false;
        problem.solver.local_max_refinements = Some(100000);
        problem.solver.local_max_nodes = Some(1000000);
        problem.limits.workers = 1;
        problem.limits.memory_mib = 4096;
        problem.limits.wall_time_secs = 30.0;
        problem.validate()?;
        let path = args.output.join(format!("chart-{mask:03}.toml"));
        fs::write(&path, toml::to_string_pretty(&problem)?)?;
        rows.push(json!({"mask":mask,"problem":path,"original_domain":source.variables.iter().enumerate().map(|(i,v)|format!("{v}{}1",if mask&(1<<i)!=0 {">"} else {"<"})).collect::<Vec<_>>(),
            "terms":q.nterms(),"coefficient_sign":coefficient_sign(&q),"orientation":orientation,"target_sign":orientation,
            "peels":peels.iter().map(|&i|&source.variables[i]).collect::<Vec<_>>(),"order":order,"exact_inverse_identity":true}));
    }
    let constant = rows.iter().filter(|r| r["coefficient_sign"] != 0).count();
    let report = json!({"source":args.input,"source_sha256":format!("{:x}",Sha256::digest(bytes)),"input_terms":p.nterms(),"charts":rows,
        "sign_definite_charts":constant,"exact_identity_count":1usize<<n,"original_completion":false,"elapsed_seconds":start.elapsed().as_secs_f64(),
        "warning":"Per-chart orders may differ. Original completion requires a common cylindrical order or common refinement, exact boundary/root-ordinal transport, and retention of every x_i=1 seam.",
        "identity":"prod_{lower i} x_i * q(y_upper=x_i-1,y_lower=(1-x_i)/x_i)=p(x)","chart_bijections":["x=1+y for x>1","x=1/(1+y) for 0<x<1"]});
    fs::write(
        args.output.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!(
        "{}",
        serde_json::to_string(
            &json!({"output":args.output,"charts":1usize<<n,"sign_definite_charts":constant,"elapsed_seconds":report["elapsed_seconds"]})
        )?
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_axis_charts_have_exact_inverse() {
        let a = Algebra::new(&["x".into(), "y".into()]).unwrap();
        let p = a.parse("2*x*y-3*x+5*y-7").unwrap();
        for mask in 0..4 {
            let q = chart(&a, &p, mask, false).unwrap();
            assert_eq!(chart(&a, &q, mask, true).unwrap(), p);
        }
    }
    #[test]
    fn lower_chart_preserves_sign_and_can_create_positive_coefficients() {
        let a = Algebra::new(&["x".into(), "y".into()]).unwrap();
        let p = a.parse("1-x*y").unwrap();
        let q = chart(&a, &p, 0, false).unwrap();
        assert_eq!(q, a.parse("x+y+x*y").unwrap());
        assert_eq!(coefficient_sign(&q), 1);
        assert_eq!(best_peels(&q, 0, &mut HashMap::new()).len(), 2);
    }
    #[test]
    fn repeated_axis_degree_is_rejected() {
        let a = Algebra::new(&["x".into()]).unwrap();
        assert!(chart(&a, &a.parse("x^2-1").unwrap(), 0, false).is_err());
    }
}
