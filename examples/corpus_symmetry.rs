//! Research only: exact domain-preserving permutation/reciprocal donor search.
//! Matches are identities, never transferred original completion certificates.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf, time::Instant};
use symgcad::{
    Problem,
    algebra::{Algebra, Poly},
    problem::normalize,
};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "artifacts/completion-aggregate-296.json")]
    audit: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    target: Option<String>,
    #[arg(long)]
    reciprocal: bool,
    #[arg(long, default_value_t = 120.0)]
    seconds: f64,
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn primitive(p: Poly) -> Poly {
    if p.is_zero() {
        p
    } else {
        let c = p.content().abs();
        p.div_coeff(&c)
    }
}
fn rename(p: &Poly, zero: &Poly, map: &[usize]) -> Poly {
    let mut q = zero.clone();
    for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
        let mut out = vec![0; map.len()];
        for (i, &j) in map.iter().enumerate() {
            out[j] = e[i];
        }
        q.append_monomial(c.clone(), &out);
    }
    q
}
/// prod x_i^deg_i(p) * p(...,1/x_i,...). The multiplier is strictly
/// positive on the original domain because only explicit x_i>0 are eligible.
fn reciprocal(p: &Poly, zero: &Poly, axes: &[usize]) -> Poly {
    let mut q = zero.clone();
    let degrees: Vec<_> = axes.iter().map(|&i| p.degree(i)).collect();
    for (c, e) in p.coefficients.iter().zip(p.exponents_iter()) {
        let mut out = e.to_vec();
        for (&i, &degree) in axes.iter().zip(&degrees) {
            out[i] = degree - e[i];
        }
        q.append_monomial(c.clone(), &out);
    }
    q
}
fn coordinate(p: &Poly) -> Option<usize> {
    if p.nterms() != 1 || p.coefficients[0].is_negative() {
        return None;
    }
    let e = p.exponents_iter().next()?;
    (e.iter().map(|x| *x as usize).sum::<usize>() == 1)
        .then(|| e.iter().position(|&x| x == 1).unwrap())
}
fn axes(p: &Problem) -> (Vec<String>, Vec<String>) {
    let mut outer = p
        .variables
        .iter()
        .filter(|v| !v.starts_with('x'))
        .cloned()
        .collect::<Vec<_>>();
    outer.sort();
    let xs = p
        .variables
        .iter()
        .filter(|v| v.starts_with('x'))
        .cloned()
        .collect();
    (outer, xs)
}
struct Chart {
    names: Vec<String>,
    constraints: Vec<Poly>,
    split: Vec<Poly>,
    positive: Vec<usize>,
}
fn chart(problem: &Problem, algebra: &Algebra, outer: &[String]) -> Result<Chart> {
    let (_, xs) = axes(problem);
    let names = outer.iter().chain(&xs).cloned().collect::<Vec<_>>();
    let map = problem
        .variables
        .iter()
        .map(|v| names.iter().position(|n| n == v).unwrap())
        .collect::<Vec<_>>();
    let n = normalize(problem, &problem.variables)?;
    let zero = algebra.parse("0")?;
    let constraints = n
        .positives
        .iter()
        .map(|p| primitive(rename(p, &zero, &map)))
        .collect::<Vec<_>>();
    let split = n
        .split
        .iter()
        .map(|p| primitive(rename(p, &zero, &map)))
        .collect::<Vec<_>>();
    let positive = constraints.iter().filter_map(coordinate).collect();
    Ok(Chart {
        names,
        constraints,
        split,
        positive,
    })
}
fn distinguished(chart: &Chart) -> Option<&Poly> {
    chart
        .split
        .iter()
        .chain(&chart.constraints)
        .max_by_key(|p| p.nterms())
}
fn coefficients(p: &Poly) -> Vec<String> {
    let mut s = p
        .coefficients
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    s.sort();
    s
}
fn signature(p: &Poly, axis: usize, outer: usize) -> Vec<String> {
    let mut s = p
        .coefficients
        .iter()
        .zip(p.exponents_iter())
        .map(|(c, e)| {
            let mut other = e
                .iter()
                .enumerate()
                .filter(|(i, _)| *i >= outer && *i != axis)
                .map(|(_, e)| *e)
                .collect::<Vec<_>>();
            other.sort_unstable();
            format!("{c}|{:?}|{}|{other:?}", &e[..outer], e[axis])
        })
        .collect::<Vec<_>>();
    s.sort();
    s
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
    for &j in &allowed[at] {
        if !map.contains(&j) {
            map.push(j);
            permutations(allowed, at + 1, map, out);
            map.pop();
        }
    }
}
fn domain_strings(polynomials: &[Poly], algebra: &Algebra) -> Vec<String> {
    let mut values = polynomials
        .iter()
        .map(|p| algebra.format(&primitive(p.clone())))
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}
fn group_map(problem: &Problem, names: &BTreeMap<String, String>) -> Vec<Vec<String>> {
    problem
        .variable_groups
        .iter()
        .map(|g| {
            let mut renamed = g.iter().map(|v| names[v].clone()).collect::<Vec<_>>();
            renamed.sort();
            renamed
        })
        .collect()
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(!args.output.exists(), "use a fresh output file");
    let audit_bytes = fs::read(&args.audit)?;
    let audit: Value = serde_json::from_slice(&audit_bytes)?;
    let groups = audit["groups"].as_array().context("missing groups")?;
    let mut inputs = Vec::new();
    for g in groups {
        let path = PathBuf::from(g["original_input"].as_str().context("input path")?);
        let bytes = fs::read(&path)?;
        ensure!(
            digest(&bytes) == g["original_input_sha256"],
            "original input changed: {}",
            path.display()
        );
        let p = Problem::from_toml(std::str::from_utf8(&bytes)?)?;
        inputs.push(p);
    }
    let started = Instant::now();
    let mut reports = Vec::new();
    for (target_index, target) in groups.iter().enumerate() {
        let label = target["identifier"].as_str().context("identifier")?;
        if target["complete_verified"] == true
            || args
                .target
                .as_ref()
                .is_some_and(|filter| !label.contains(filter))
        {
            continue;
        }
        let original = &inputs[target_index];
        let (outer, xs) = axes(original);
        ensure!(xs.len() <= 8, "coordinate search cap");
        let names = outer
            .iter()
            .cloned()
            .chain((0..xs.len()).map(|i| format!("y{i}")))
            .collect::<Vec<_>>();
        let algebra = Algebra::new(&names)?;
        let zero = algebra.parse("0")?;
        let target_chart = chart(original, &algebra, &outer)?;
        let f = distinguished(&target_chart).context("empty problem")?;
        let signatures = (outer.len()..names.len())
            .map(|i| signature(f, i, outer.len()))
            .collect::<Vec<_>>();
        let domain = domain_strings(&target_chart.constraints, &algebra);
        let target_coefficients = coefficients(f);
        let mut matches = Vec::new();
        let mut tested_donors = 0;
        let mut tested_masks = 0;
        let mut tested_permutations = 0;
        let mut complete = true;
        'donors: for (donor_index, donor) in groups.iter().enumerate() {
            if donor_index == target_index {
                continue;
            }
            if started.elapsed().as_secs_f64() > args.seconds {
                complete = false;
                break;
            }
            let p = &inputs[donor_index];
            let (donor_outer, donor_xs) = axes(p);
            if donor_outer != outer
                || donor_xs.len() != xs.len()
                || p.split.len() != original.split.len()
            {
                continue;
            }
            let d = chart(p, &algebra, &outer)?;
            let df = distinguished(&d).context("empty donor")?;
            tested_donors += 1;
            let may_reflect = label.starts_with("finite")
                && donor["identifier"]
                    .as_str()
                    .is_some_and(|s| s.starts_with("finite"))
                && outer == ["s".to_string(), "t".to_string()];
            for reflection in [false, true] {
                if reflection && !may_reflect {
                    continue;
                }
                let reflect = |p: &Poly| -> Result<Poly> {
                    Ok(if reflection {
                        p.replace_with_poly(1, &algebra.parse("5/9-s-t")?)
                    } else {
                        p.clone()
                    })
                };
                let reflected = primitive(reflect(df)?);
                for orientation in [1, -1] {
                    let oriented = if orientation == 1 {
                        reflected.clone()
                    } else {
                        -reflected.clone()
                    };
                    if coefficients(&oriented) != target_coefficients {
                        continue;
                    }
                    let invertible = d
                        .positive
                        .iter()
                        .copied()
                        .filter(|&i| i >= outer.len())
                        .collect::<Vec<_>>();
                    let masks = if args.reciprocal {
                        1usize << invertible.len()
                    } else {
                        1
                    };
                    for mask in 0..masks {
                        if started.elapsed().as_secs_f64() > args.seconds {
                            complete = false;
                            break 'donors;
                        }
                        tested_masks += 1;
                        let inverted = invertible
                            .iter()
                            .enumerate()
                            .filter_map(|(bit, &i)| ((mask & (1 << bit)) != 0).then_some(i))
                            .collect::<Vec<_>>();
                        let transformed = reciprocal(&oriented, &zero, &inverted);
                        let allowed = (outer.len()..names.len())
                            .map(|i| {
                                let s = signature(&transformed, i, outer.len());
                                (0..xs.len())
                                    .filter(|&j| {
                                        signatures[j] == s
                                            && (d.positive.contains(&i)
                                                == target_chart
                                                    .positive
                                                    .contains(&(j + outer.len())))
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>();
                        let mut candidates = Vec::new();
                        permutations(&allowed, 0, &mut vec![], &mut candidates);
                        for permutation in candidates {
                            tested_permutations += 1;
                            let map = (0..outer.len())
                                .chain(permutation.iter().map(|j| j + outer.len()))
                                .collect::<Vec<_>>();
                            if rename(&transformed, &zero, &map) != *f {
                                continue;
                            }
                            let transform = |p: &Poly| -> Result<Poly> {
                                Ok(primitive(rename(
                                    &reciprocal(&reflect(p)?, &zero, &inverted),
                                    &zero,
                                    &map,
                                )))
                            };
                            let transformed_domain = d
                                .constraints
                                .iter()
                                .map(|p| {
                                    // x>0 and 1/x>0 are equivalent on the certified
                                    // positive-coordinate domain; retain this premise.
                                    if coordinate(p).is_some_and(|i| inverted.contains(&i)) {
                                        Ok(primitive(rename(p, &zero, &map)))
                                    } else {
                                        transform(p)
                                    }
                                })
                                .collect::<Result<Vec<_>>>()?;
                            if domain_strings(&transformed_domain, &algebra) != domain {
                                continue;
                            }
                            let mut split_signs = Vec::new();
                            for (dp, tp) in d.split.iter().zip(&target_chart.split) {
                                let transformed = transform(dp)?;
                                if transformed == *tp {
                                    split_signs.push(1);
                                } else if -transformed == *tp {
                                    split_signs.push(-1);
                                } else {
                                    break;
                                }
                            }
                            if split_signs.len() != d.split.len() {
                                continue;
                            }
                            let renaming = d
                                .names
                                .iter()
                                .enumerate()
                                .map(|(i, v)| (v.clone(), target_chart.names[map[i]].clone()))
                                .collect::<BTreeMap<_, _>>();
                            let identity = target_chart
                                .names
                                .iter()
                                .map(|v| (v.clone(), v.clone()))
                                .collect::<BTreeMap<_, _>>();
                            let group_compatible =
                                group_map(p, &renaming) == group_map(original, &identity);
                            let attempt = donor["attempts"].as_array().and_then(|a| {
                                a.iter()
                                    .find(|a| a["verified_matching_full_problem"] == true)
                            });
                            let source_order: Vec<String> = attempt
                                .map(|a| serde_json::from_value(a["proof_order"].clone()))
                                .transpose()?
                                .unwrap_or_else(|| p.solver.order.clone());
                            let mapped_order = source_order
                                .iter()
                                .map(|v| renaming[v].clone())
                                .collect::<Vec<_>>();
                            let fixed_order_compatible = original.solver.order.is_empty()
                                || mapped_order.is_empty()
                                || mapped_order == original.solver.order;
                            let triangular_reflection = !reflection
                                || mapped_order.is_empty()
                                || mapped_order.iter().position(|v| v == "s")
                                    < mapped_order.iter().position(|v| v == "t");
                            let renamed_inversions = inverted
                                .iter()
                                .map(|&i| target_chart.names[map[i]].clone())
                                .collect::<Vec<_>>();
                            matches.push(json!({"donor":donor["identifier"],"donor_scope":donor["scope"],"donor_complete_verified":donor["complete_verified"],"donor_input":donor["original_input"],"donor_input_sha256":donor["original_input_sha256"],"source_result":attempt.map(|a|&a["result_file"]),"source_result_sha256":attempt.map(|a|&a["actual_result_sha256"]),"renaming":renaming,"inverted_source_coordinates":inverted.iter().map(|&i|&d.names[i]).collect::<Vec<_>>(),"inverted_target_coordinates":renamed_inversions,"t_reflection":reflection,"t_map":if reflection{"5/9-s-t"}else{"t"},"split_orientations":split_signs,"strict_domain_identity":true,"positive_denominator_premises":true,"groups_compatible":group_compatible,"original_fixed_order_compatible":fixed_order_compatible,"source_order":source_order,"mapped_order":mapped_order,"reflection_triangular_in_mapped_order":triangular_reflection,"transport_eligible":group_compatible&&fixed_order_compatible&&triangular_reflection,"completion_claimed":false}));
                        }
                    }
                }
            }
        }
        reports.push(json!({"target":label,"scope":target["scope"],"target_input":target["original_input"],"target_input_sha256":target["original_input_sha256"],"matches":matches,"tested_compatible_donors":tested_donors,"tested_masks":tested_masks,"tested_permutations":tested_permutations,"complete_search":complete}));
    }
    let matches = reports
        .iter()
        .flat_map(|r| r["matches"].as_array().unwrap())
        .collect::<Vec<_>>();
    let summary = json!({"targets":reports.len(),"matches":matches.len(),"verified_donor_matches":matches.iter().filter(|m|m["donor_complete_verified"]==true).count(),"eligible_verified_donor_matches":matches.iter().filter(|m|m["donor_complete_verified"]==true&&m["transport_eligible"]==true).count(),"complete_search":reports.iter().all(|r|r["complete_search"]==true),"seconds":started.elapsed().as_secs_f64(),"original_completions_claimed":0});
    let report = json!({"schema_version":1,"audit":args.audit,"audit_sha256":digest(&audit_bytes),"reciprocal_enabled":args.reciprocal,"summary":summary,"targets":reports,"limitations":"Exact search covers positive coordinate reciprocal subsets, propagator permutations and optional ZZ t reflection, preserving all strict constraints and the ordered split list up to recorded split signs. It does not cover arbitrary scalings, translations, projective maps, parameter relabelings or positive monomial factor removal. Polynomial identities do not transfer proof trees; no completion is claimed."});
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
    println!("{}", report["summary"]);
    Ok(())
}

#[test]
fn reciprocal_identity_preserves_sign_only_with_positive_denominator() -> Result<()> {
    let a = Algebra::new(&["x".into(), "y".into()])?;
    let zero = a.parse("0")?;
    let p = a.parse("x*y+x-y-1")?;
    let inverted = reciprocal(&p, &zero, &[0]);
    assert_eq!(inverted, -p.clone());
    assert_eq!(reciprocal(&inverted, &zero, &[0]), p);
    assert_eq!(coordinate(&a.parse("3*x")?), Some(0));
    assert_eq!(coordinate(&a.parse("-x")?), None);
    assert_eq!(coordinate(&a.parse("x*y")?), None);
    Ok(())
}

#[test]
fn crossing_must_expand_before_coefficient_and_support_screening() -> Result<()> {
    let algebra = Algebra::new(&["s".into(), "t".into(), "x".into(), "y".into()])?;
    let donor = algebra.parse("s*x+t*y")?;
    let target = algebra.parse("s*x+(5/9-s-t)*y")?;
    assert_ne!(donor.nterms(), target.nterms());
    assert_ne!(
        coefficients(&primitive(donor.clone())),
        coefficients(&primitive(target.clone()))
    );
    let crossed = primitive(donor.replace_with_poly(1, &algebra.parse("5/9-s-t")?));
    assert_eq!(crossed, primitive(target));
    Ok(())
}
