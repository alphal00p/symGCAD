//! Exact crossing/permutation identities and direct original-problem retries.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf, time::Instant};
use symgcad::{
    Problem,
    algebra::{Algebra, Poly},
    campaign::CampaignConfig,
    problem::normalize,
};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "artifacts/completion-aggregate-296.json")]
    audit: PathBuf,
    #[arg(long, default_value = "finiteC_6_470_")]
    target: String,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 30.0)]
    seconds: f64,
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn primitive(p: Poly) -> Poly {
    if p.is_zero() {
        p
    } else {
        let content = p.content().abs();
        p.div_coeff(&content)
    }
}
fn rename(p: &Poly, zero: &Poly, map: &[usize]) -> Poly {
    let mut q = zero.clone();
    for (coefficient, monomial) in p.coefficients.iter().zip(p.exponents_iter()) {
        let mut exponents = vec![0; map.len()];
        for (from, &to) in map.iter().enumerate() {
            exponents[to] = monomial[from];
        }
        q.append_monomial(coefficient.clone(), &exponents);
    }
    q
}
fn chart(p: &Problem, algebra: &Algebra) -> Result<(Vec<String>, Vec<Poly>, Poly)> {
    let xs: Vec<_> = p
        .variables
        .iter()
        .filter(|v| v.starts_with('x'))
        .cloned()
        .collect();
    ensure!(
        p.variables.len() == xs.len() + 2,
        "expected two outer parameters"
    );
    let normalized = normalize(p, &p.variables)?;
    ensure!(
        normalized.split.len() == 1,
        "expected one full split polynomial"
    );
    let map = p
        .variables
        .iter()
        .map(|v| {
            if v == "s" {
                Ok(0)
            } else if v == "t" {
                Ok(1)
            } else {
                xs.iter()
                    .position(|x| x == v)
                    .map(|i| i + 2)
                    .context("unknown coordinate")
            }
        })
        .collect::<Result<Vec<_>>>()?;
    let zero = algebra.parse("0")?;
    Ok((
        xs,
        normalized
            .positives
            .iter()
            .map(|p| primitive(rename(p, &zero, &map)))
            .collect(),
        primitive(rename(&normalized.split[0], &zero, &map)),
    ))
}
fn signature(p: &Poly, axis: usize) -> Vec<String> {
    let mut rows = p
        .coefficients
        .iter()
        .zip(p.exponents_iter())
        .map(|(c, e)| {
            let mut others = e
                .iter()
                .enumerate()
                .filter(|(i, _)| *i >= 2 && *i != axis)
                .map(|(_, e)| *e)
                .collect::<Vec<_>>();
            others.sort_unstable();
            format!("{c}|{}|{}|{}|{others:?}", e[0], e[1], e[axis])
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}
fn permutations(
    allowed: &[Vec<usize>],
    at: usize,
    map: &mut Vec<usize>,
    out: &mut Vec<Vec<usize>>,
) {
    if at == allowed.len() {
        out.push(map.clone());
        return;
    }
    for &candidate in &allowed[at] {
        if !map.contains(&candidate) {
            map.push(candidate);
            permutations(allowed, at + 1, map, out);
            map.pop();
        }
    }
}
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        !args.output.exists(),
        "use a fresh identity output directory"
    );
    let audit_bytes = fs::read(&args.audit)?;
    let audit: Value = serde_json::from_slice(&audit_bytes)?;
    let groups = audit["groups"].as_array().context("missing groups")?;
    let target = groups
        .iter()
        .find(|g| {
            g["scope"] == "symbolic_core"
                && g["identifier"]
                    .as_str()
                    .is_some_and(|n| n.starts_with(&args.target))
        })
        .context("target not found")?;
    let target_path = PathBuf::from(target["original_input"].as_str().context("missing input")?);
    let target_bytes = fs::read(&target_path)?;
    ensure!(
        digest(&target_bytes) == target["original_input_sha256"],
        "target input changed"
    );
    let original = Problem::from_toml(std::str::from_utf8(&target_bytes)?)?;
    let count = original
        .variables
        .iter()
        .filter(|v| v.starts_with('x'))
        .count();
    ensure!(
        count <= 8,
        "permutation search is limited to eight coordinates"
    );
    let mut names = vec!["s".into(), "t".into()];
    names.extend((0..count).map(|i| format!("y{i}")));
    let algebra = Algebra::new(&names)?;
    let zero = algebra.parse("0")?;
    let reflected_t = algebra.parse("5/9-s-t")?;
    let (target_xs, target_constraints, target_f) = chart(&original, &algebra)?;
    let mut target_domain = target_constraints
        .iter()
        .map(|p| algebra.format(p))
        .collect::<Vec<_>>();
    target_domain.sort();
    let target_signatures = (0..count)
        .map(|i| signature(&target_f, i + 2))
        .collect::<Vec<_>>();
    fs::create_dir_all(&args.output)?;
    let output = args.output.canonicalize()?;
    let started = Instant::now();
    let mut matches = Vec::new();
    let mut checked = Vec::new();
    let mut cases = Vec::new();
    for donor in groups {
        if started.elapsed().as_secs_f64() > args.seconds {
            break;
        }
        if donor["scope"] != "symbolic_core"
            || donor["complete_verified"] != true
            || !donor["identifier"]
                .as_str()
                .is_some_and(|n| n.starts_with("finite"))
        {
            continue;
        }
        let donor_path = PathBuf::from(donor["original_input"].as_str().context("donor path")?);
        let donor_bytes = fs::read(&donor_path)?;
        ensure!(
            digest(&donor_bytes) == donor["original_input_sha256"],
            "donor changed"
        );
        let problem = Problem::from_toml(std::str::from_utf8(&donor_bytes)?)?;
        if problem.variables.len() != original.variables.len()
            || !problem.variables.contains(&"s".into())
            || !problem.variables.contains(&"t".into())
        {
            continue;
        }
        let (xs, constraints, f) = chart(&problem, &algebra)?;
        if xs.len() != count {
            continue;
        }
        for reflection in [false, true] {
            let transformed = primitive(if reflection {
                f.replace_with_poly(1, &reflected_t)
            } else {
                f.clone()
            });
            if transformed.nterms() != target_f.nterms() {
                continue;
            }
            let allowed = (0..count)
                .map(|i| {
                    let sig = signature(&transformed, i + 2);
                    (0..count)
                        .filter(|&j| target_signatures[j] == sig)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let mut candidates = Vec::new();
            permutations(&allowed, 0, &mut vec![], &mut candidates);
            checked.push(json!({"donor":donor["identifier"],"reflection":reflection,"candidate_permutations":candidates.len()}));
            for permutation in candidates {
                let mut map = vec![0, 1];
                map.extend(permutation.iter().map(|v| v + 2));
                if rename(&transformed, &zero, &map) != target_f {
                    continue;
                }
                let mut domain = constraints
                    .iter()
                    .map(|p| {
                        let p = if reflection {
                            p.replace_with_poly(1, &reflected_t)
                        } else {
                            p.clone()
                        };
                        algebra.format(&primitive(rename(&p, &zero, &map)))
                    })
                    .collect::<Vec<_>>();
                domain.sort();
                if domain != target_domain {
                    continue;
                }
                let renaming: BTreeMap<_, _> = xs
                    .iter()
                    .enumerate()
                    .map(|(i, x)| (x.clone(), target_xs[permutation[i]].clone()))
                    .chain([("s".into(), "s".into()), ("t".into(), "t".into())])
                    .collect();
                let attempt = donor["attempts"]
                    .as_array()
                    .context("attempts missing")?
                    .iter()
                    .find(|a| a["verified_matching_full_problem"] == true)
                    .context("verified donor lacks attempt")?;
                let source_order: Vec<String> =
                    serde_json::from_value(attempt["proof_order"].clone())?;
                let order = source_order
                    .iter()
                    .map(|v| renaming[v].clone())
                    .collect::<Vec<_>>();
                let mut retry = original.clone();
                let donor_input = PathBuf::from(
                    attempt["input_file"]
                        .as_str()
                        .context("donor input missing")?,
                );
                let donor_input_bytes = fs::read(&donor_input)?;
                ensure!(
                    digest(&donor_input_bytes) == attempt["input_sha256"],
                    "verified donor input changed"
                );
                retry.solver = Problem::from_toml(std::str::from_utf8(&donor_input_bytes)?)?.solver;
                retry.solver.order = order.clone();
                retry.solver.ordering = "given".into();
                retry.solver.homogeneous_anchor = retry
                    .solver
                    .homogeneous_anchor
                    .map(|v| renaming[&v].clone());
                retry.solver.homogeneous_block = retry
                    .solver
                    .homogeneous_block
                    .iter()
                    .map(|v| renaming[v].clone())
                    .collect();
                retry.solver.checkpoint_dir = None;
                ensure!(
                    original.solver.order.is_empty() || retry.solver.order == original.solver.order,
                    "original fixed order would change"
                );
                retry.limits.workers = 1;
                retry.limits.memory_mib = 8192;
                retry.limits.wall_time_secs = 120.0;
                retry.validate()?;
                let path = output.join(format!("retry-{:03}.toml", matches.len()));
                fs::write(&path, toml::to_string_pretty(&retry)?)?;
                let triangular = !reflection
                    || order.iter().position(|v| v == "s") < order.iter().position(|v| v == "t");
                matches.push(json!({"donor":donor["identifier"],"donor_input":donor_path,"donor_input_sha256":digest(&donor_bytes),"source_result":attempt["result_file"],"source_result_sha256":attempt["actual_result_sha256"],"feynman_renaming":renaming,"t_reflection":reflection,"t_map":if reflection{"5/9-s-t"}else{"t"},"normalized_split_identity":true,"normalized_domain_identity":true,"source_order":source_order,"retry_order":order,"affine_map_triangular_in_order":triangular,"retry_input":path}));
                if cases.len() < 2 {
                    cases.push(path);
                }
            }
        }
    }
    let report = json!({"target":target["identifier"],"target_input":target_path,"target_input_sha256":digest(&target_bytes),"audit":args.audit,"audit_sha256":digest(&audit_bytes),"matches":matches,"checked":checked,"seconds":started.elapsed().as_secs_f64(),"complete_search":started.elapsed().as_secs_f64()<=args.seconds,"original_completions_claimed":0});
    fs::write(
        output.join("identity.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    if !cases.is_empty() {
        let config = CampaignConfig {
            cases,
            output_dir: output.join("results"),
            repeats: 1,
            solve_timeout_secs: 120.0,
            verify_timeout_secs: 240.0,
            campaign_time_secs: 3600.0,
            memory_mib: 8192,
            workers: 1,
            resume: false,
            cache_policy: "cold".into(),
            ..Default::default()
        };
        fs::write(
            output.join("campaign.toml"),
            toml::to_string_pretty(&config)?,
        )?;
    }
    println!(
        "{}",
        json!({"matches":report["matches"].as_array().unwrap().len(),"seconds":report["seconds"],"output":output})
    );
    Ok(())
}
