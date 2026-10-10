//! Research-only exact A/Ax446 kinematic partition identities; no GCAD coverage claim.
use anyhow::{Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use symbolica::prelude::*;
use symgcad::{
    Problem,
    algebra::{Algebra, Poly},
    problem::normalize,
    projection::{discriminant, exact_resultant},
};

const A_INPUT: &str =
    "artifacts/corpus/symbolic/0093_finiteA_7_446_11_0__0_5_1_1_1_1_0_1_1_symbolic.toml";
const AX_INPUT: &str =
    "artifacts/corpus/symbolic/0152_finiteAx12_7_446_11_0__0_5_1_1_1_1_0_1_1_symbolic.toml";
#[derive(Parser)]
struct Args {
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 9)]
    cpu: usize,
    #[arg(long, default_value_t = 120.0)]
    seconds: f64,
    #[arg(long)]
    factor_resultant: bool,
    #[arg(long, hide = true)]
    worker: bool,
}
fn positive_coefficients(p: &Poly) -> bool {
    !p.is_zero() && p.coefficients.iter().all(|c| *c > Rational::zero())
}
fn save(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("partial.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}
struct Identities {
    algebra: Algebra,
    report: Value,
    pair_resultant: Poly,
}
fn identities() -> Result<Identities> {
    let names = ["s", "u", "x1", "x2", "x3", "x4", "x5", "x7", "x8", "delta"]
        .map(String::from)
        .to_vec();
    let alg = Algebra::new(&names)?;
    let p = |s: &str| alg.parse(s);
    let a = p("x1+x2+x3")?;
    let b = p("x4+x5+x7")?;
    let l = p("x1+x3")?;
    let r = p("x5+x7")?;
    let x = p("x1+x5")?;
    let y = p("x3+x7")?;
    let xy = &x * &y;
    let c0 = p("(x1+x5+x3+x7)^2-u*(x1+x5)*(x3+x7)")?;
    let w = p("2*(x1+x5)+(31/18)*(x3+x7)")?;
    let grouped = p(
        "(x1+x2+x3+x4+x5+x7)*((x1+x2+x3)*(x4+x5+x7)+(x1+x2+x3+x4+x5+x7)*x8)-s*x2*x4*x8-u*((x4+x5+x7)*x1*x3+(x1+x2+x3)*x5*x7+x8*(x1+x5)*(x3+x7))-(5/18)*((x4+x5+x7)*x2*x3+(x1+x2+x3)*x4*x7+x8*(x2+x4)*(x3+x7))",
    )?;
    let slope = grouped.derivative(8);
    let endpoint = grouped.replace(8, &Rational::zero());
    ensure!(
        grouped == &slope * &p("x8")? + endpoint.clone(),
        "F linear identity"
    );
    let left = p("(x1-x3)^2+(4-u)*x1*x3+x2^2+2*x1*x2+(31/18)*x2*x3")?;
    let right = p("(x5-x7)^2+(4-u)*x5*x7+x4^2+2*x4*x5+(31/18)*x4*x7")?;
    ensure!(
        endpoint == &b * &left + &a * &right,
        "B sum of squares identity"
    );
    let linear = &w - &p("(s-2)*x4")?;
    let constant = &p("x4^2")? + &(&w * &p("x4")?) + c0.clone();
    ensure!(
        slope == p("x2^2")? + &linear * &p("x2")? + constant.clone(),
        "A quadratic identity"
    );
    let d = discriminant(&slope, 3)?;
    ensure!(
        d == p("s*(s-4)*x4^2")? - &p("2*s*x4")? * &w + &w * &w - &p("4")? * &c0,
        "D identity"
    );
    let dd = discriminant(&d, 5)?;
    ensure!(
        dd == &p("16*s")? * &(&w * &w + &p("s-4")? * &c0),
        "second D identity"
    );
    let d_mid = &p("s*(s-4)")? * &w.pow(2) - &p("2*s*(s-2)")? * &w.pow(2)
        + &p("(s-2)^2")? * &(&w * &w - &p("4")? * &c0);
    ensure!(
        d_mid == -(&p("4")? * &(&p("s-1")? * &w.pow(2) + &p("(s-2)^2")? * &c0)),
        "cleared middle D identity"
    );

    // C0 >= 0 implies B > 0 even on the complementary u>4 half-plane.
    let b0 = endpoint
        .replace(3, &Rational::zero())
        .replace(5, &Rational::zero());
    let cross = p("x1*x7-x3*x5")?;
    ensure!(
        &(&l + &r) * &b0 == &(&l * &r) * &c0 + &p("u")? * &cross.pow(2),
        "constant B/C0 square identity"
    );
    let k2 = endpoint
        .derivative(3)
        .replace(3, &Rational::zero())
        .replace(5, &Rational::zero());
    let k4 = endpoint
        .derivative(5)
        .replace(3, &Rational::zero())
        .replace(5, &Rational::zero());
    let positive2 = &xy * &k2 - &c0 * &p("x5*x7")?;
    let positive4 = &xy * &k4 - &c0 * &p("x1*x3")?;
    ensure!(
        positive_coefficients(&positive2) && positive_coefficients(&positive4),
        "C0 implication residual coefficients"
    );
    let b_remainder = &endpoint - &b0 - &p("x2")? * &k2 - &p("x4")? * &k4;
    ensure!(positive_coefficients(&b_remainder), "higher B coefficients");

    // Every u>4 has B signs of both kinds on the entire original orthant.
    let delta = p("delta")?;
    let den = p("10*(1+delta)")?;
    let mut neg = endpoint.replace_with_poly(1, &p("delta+4")?);
    for axis in [2, 4, 6, 7] {
        neg = neg.replace_with_poly(axis, &den);
    }
    for axis in [3, 5] {
        neg = neg.replace_with_poly(axis, &delta);
    }
    let neg_expected = p("-2*delta*(20+21*delta)*(100*delta^2+(1456/9)*delta+565/9)")?;
    ensure!(neg == neg_expected, "all-u negative endpoint family");
    let mut pos = endpoint.replace_with_poly(2, &p("2*u+1")?);
    for axis in [3, 4, 5, 6, 7] {
        pos = pos.replace(axis, &Rational::one());
    }
    ensure!(
        pos == p("4*u^2+(427/9)*u+157/3")?,
        "all-u positive endpoint family"
    );
    let mut cone_pos = endpoint.clone();
    for axis in [2, 4, 6, 7] {
        cone_pos = cone_pos.replace(axis, &Rational::one());
    }
    for axis in [3, 5] {
        cone_pos = cone_pos.replace_with_poly(axis, &p("u+1")?);
    }
    ensure!(
        cone_pos == p("2*(u+3)*((u+3)^2-u-(5/18)*(u+1))")?,
        "C0-negative B-positive family"
    );
    ensure!(
        positive_coefficients(&cone_pos.replace_with_poly(1, &p("4+delta")?)),
        "cone positive coefficients"
    );

    // One cancellation reduces the A,B resultant to a quadratic norm.
    let rem = &endpoint - &(&b * &slope);
    ensure!(rem.degree(3) <= 1, "B-bA not linear");
    let j = rem.derivative(3);
    let k = rem.replace(3, &Rational::zero());
    let j_expected = p("-(x4+x5+x7)^2-u*x5*x7+(5/18)*(x5+x7)*x7+s*(x4+x5+x7)*x4")?;
    let k_expected = p(
        "-(x4+x5+x7)^2*(x1+x3+x4+x5+x7)+u*((x4+x5+x7)*(x1*x7+x3*x5)+(x4+x5+x7-x1-x3)*x5*x7)+(5/18)*((x4+x5+x7)*x4*x3+(x4+x5+x7-x1-x3)*x4*x7)",
    )?;
    ensure!(
        j == j_expected && k == k_expected,
        "linear remainder formulas"
    );
    let pair_resultant = &k * &k - &linear * &j * &k + &constant * &j.pow(2);
    ensure!(
        exact_resultant(&slope, &endpoint, 3)? == pair_resultant,
        "quadratic norm resultant identity"
    );
    let endpoint_disc = discriminant(&endpoint, 3)?;
    let pair_s_disc = discriminant(&pair_resultant, 0)?;
    ensure!(
        pair_s_disc == &p("x4^2")? * &k.pow(2) * &endpoint_disc,
        "resultant discriminant ladder identity"
    );
    let mut originals = Vec::new();
    for (path, crossed) in [(A_INPUT, true), (AX_INPUT, false)] {
        let bytes = fs::read(path)?;
        let problem: Problem = toml::from_str(std::str::from_utf8(&bytes)?)?;
        let n = normalize(&problem, &problem.variables)?;
        let mut text = n.algebra.format(&n.split[0]);
        // Token-level variable substitution is performed through a temporary t map.
        let mut original_names = names.clone();
        original_names.push("t".into());
        let oa = Algebra::new(&original_names)?;
        let op = oa
            .parse(&text)?
            .replace_with_poly(10, &oa.parse(if crossed { "5/9-s-u" } else { "u" })?);
        text = oa.format(&op);
        ensure!(
            alg.parse(&text)? == &p("18")? * &grouped,
            "original normalized F mismatch for {path}"
        );
        originals.push(json!({"path":path,"sha256":format!("{:x}",Sha256::digest(bytes)),"exact_normalized_scale":18,"inverse_original_t":if crossed {"5/9-s-u"}else{"u"},"constraints":problem.constraints,"variable_groups":problem.variable_groups}));
    }
    let mut report = json!({"schema_version":1,"research_only":true,"coverage_claim":false,"originals":originals,"variables":names,"mass":"5/18","identities_verified":true,"F":alg.format(&grouped),"A":alg.format(&slope),"B":alg.format(&endpoint),"W":alg.format(&w),"C0":alg.format(&c0),"A_linear":alg.format(&linear),"A_constant":alg.format(&constant),"D":alg.format(&d),"D_discriminant":alg.format(&dd),"D_at_W_over_sminus2_times_sminus2_squared":alg.format(&d_mid),"B_C0_positive_residual_x2":alg.format(&positive2),"B_C0_positive_residual_x4":alg.format(&positive4),"B_higher_positive_terms":alg.format(&b_remainder),"B_negative_for_every_u_above4_scaled":alg.format(&neg),"B_positive_for_every_u_above4":alg.format(&pos),"B_positive_inside_C0_negative":alg.format(&cone_pos),"B_minus_bA_linear":alg.format(&j),"B_minus_bA_constant":alg.format(&k),"resultant_A_B":alg.format(&pair_resultant),"resultant_terms":pair_resultant.nterms(),"candidate_order":["s","t","x1","x3","x5","x7","x4","x2","x8"],"subdomain_generic_cell_count":5,"subdomain_condition":"u<4, or u>4 and C0>0; C0 base sectors need separate cylindrical decomposition","complement":"u>4,C0<0: B takes both signs for every u; no kinematic-only uniform endpoint rule"});
    report["B_discriminant_x2"] = json!(alg.format(&endpoint_disc));
    report["B_discriminant_terms"] = json!(endpoint_disc.nterms());
    report["resultant_discriminant_s_identity"] = json!("disc_s Res_x2(A,B) = x4^2*K^2*disc_x2(B)");
    Ok(Identities {
        algebra: alg,
        report,
        pair_resultant,
    })
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.seconds > 0.0 && args.seconds <= 120.0 && args.seconds.is_finite(),
        "invalid wall budget"
    );
    ensure!(!args.output.exists(), "fresh output required");
    if !args.worker {
        let mut command = Command::new("taskset");
        command
            .args(["-c", &args.cpu.to_string()])
            .arg(std::env::current_exe()?)
            .arg("--worker")
            .arg("--output")
            .arg(&args.output)
            .arg("--seconds")
            .arg(args.seconds.to_string());
        if args.factor_resultant {
            command.arg("--factor-resultant");
        }
        let process = symgcad::benchmark::run_bounded(
            &mut command,
            &symgcad::Limits {
                wall_time_secs: args.seconds,
                memory_mib: 2048,
                workers: 1,
                max_cells: None,
            },
        )?;
        save(
            &args.output.with_extension("process.json"),
            &serde_json::to_value(process)?,
        )?;
        return Ok(());
    }
    let started = Instant::now();
    let mut data = identities()?;
    data.report["identity_seconds"] = json!(started.elapsed().as_secs_f64());
    save(&args.output, &data.report)?;
    if args.factor_resultant {
        let start = Instant::now();
        let factors = symgcad::algebra::factors_checked(&[data.pair_resultant.clone()])?;
        data.report["resultant_factors"] = json!(
            factors
                .iter()
                .map(|f| data.algebra.format(f))
                .collect::<Vec<_>>()
        );
        data.report["resultant_factor_seconds"] = json!(start.elapsed().as_secs_f64());
        save(&args.output, &data.report)?;
    }
    println!("exact identities verified; research only, no original GCAD coverage claim");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    use symgcad::algebra::rational;
    #[test]
    fn exact_originals_and_partition_identities() -> Result<()> {
        let d = identities()?;
        ensure!(
            d.report["originals"].as_array().context("originals")?.len() == 2,
            "both originals"
        );
        Ok(())
    }
    #[test]
    fn complementary_endpoint_signs_are_strict_at_small_and_large_u() -> Result<()> {
        let d = identities()?;
        let neg = d.algebra.parse(
            d.report["B_negative_for_every_u_above4_scaled"]
                .as_str()
                .unwrap(),
        )?;
        let pos = d
            .algebra
            .parse(d.report["B_positive_for_every_u_above4"].as_str().unwrap())?;
        for v in ["1/1000", "1", "1000"] {
            let delta = rational(v)?;
            ensure!(
                neg.replace(9, &delta).get_constant().is_negative(),
                "negative endpoint"
            );
            ensure!(
                pos.replace(1, &(&delta + &Rational::from(4)))
                    .get_constant()
                    > Rational::zero(),
                "positive endpoint"
            );
        }
        Ok(())
    }
}
