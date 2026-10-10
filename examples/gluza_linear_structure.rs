//! Research-only exact endpoint and positive-combination screening.
//! This emits identities, not an original CAD or a new proof rule.
use anyhow::{Result, ensure};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf, process::Command, time::Instant};
use symbolica::prelude::*;
use symgcad::{
    Problem,
    algebra::{Algebra, Poly, factors_checked},
    problem::normalize,
};

#[derive(Parser)]
struct Args {
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, hide = true)]
    worker: bool,
    /// Screen exact minors against the restored homogeneous x3 coordinate.
    #[arg(long)]
    anchor_minors: bool,
    /// Research-only exact rational search for A>0,B<0 in F=A*z+B.
    #[arg(long, default_value_t = 0)]
    corner_samples: usize,
    /// Check the source paper's one-mass self-energy determinant identities.
    #[arg(long)]
    graph_minors: bool,
    /// Check the two massless-axis identities in the eight-variable Gluza33 graph.
    #[arg(long)]
    graph33_minors: bool,
}

fn graph33_minors(a: &Algebra, p: &Poly) -> Result<Value> {
    ensure!(
        a.names == ["x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7"],
        "source graph33 requires its original eight coordinates"
    );
    let ma = a.parse("x0+x2+x5+x6")?;
    let mb = a.parse("-x0-x6")?;
    let mc = a.parse("-x2")?;
    let md = a.parse("x0+x1+x3+x6")?;
    let me = a.parse("-x3")?;
    let mf = a.parse("x2+x3+x4+x7")?;
    let adj = [
        [
            &md * &mf - &me * &me,
            &mc * &me - &mb * &mf,
            &mb * &me - &mc * &md,
        ],
        [
            &mc * &me - &mb * &mf,
            &ma * &mf - &mc * &mc,
            &mb * &mc - &ma * &me,
        ],
        [
            &mb * &me - &mc * &md,
            &mb * &mc - &ma * &me,
            &ma * &md - &mb * &mb,
        ],
    ];
    let u = &(&ma * &adj[0][0]) + &(&mb * &adj[0][1]) + (&mc * &adj[0][2]);
    let j = [a.parse("x5+x6")?, a.parse("-x6")?, a.parse("x7")?];
    let adj_j = adj
        .iter()
        .map(|row| {
            row.iter()
                .zip(&j)
                .fold(p.zero(), |acc, (coefficient, component)| {
                    &acc + &(coefficient * component)
                })
        })
        .collect::<Vec<_>>();
    let jt_adj_j = j
        .iter()
        .zip(&adj_j)
        .fold(p.zero(), |acc, (component, value)| {
            &acc + &(component * value)
        });
    let v = &a.parse("x5+x6+x7")? * &u - jt_adj_j;
    let mass = a.parse("x0+x1+x2+x4+x6+x7")?;
    ensure!(
        &mass * &u - v.clone().mul_coeff(Rational::from(10)) == *p,
        "graph33 momentum identity does not match exact fixture"
    );
    ensure!(
        sign(&u) == 1,
        "graph33 U is not strictly coefficient-positive"
    );
    let mut direct = Vec::new();
    for (axis, loop_vector, external) in [(3, [0, 1, -1], 0), (5, [1, 0, 0], 1)] {
        let mut h = u.clone().mul_coeff(Rational::from(external));
        for (component, weight) in adj_j.iter().zip(loop_vector) {
            h = &h - &component.clone().mul_coeff(Rational::from(weight));
        }
        let delta = &p.derivative(axis) * &u - p * &u.derivative(axis);
        ensure!(
            &delta + &(&h * &h).mul_coeff(Rational::from(10)) == p.zero(),
            "graph33 massless Rayleigh identity failed"
        );
        direct.push(json!({"axis":a.names[axis],"H":a.format(&h),"H_terms":h.nterms(),"identity":"F_i*U-F*U_i = -10*H_i^2","identity_verified":true}));
    }
    let mut current = p.clone();
    let mut guide = u.clone();
    let mut chain = Vec::new();
    for axis in [3, 5] {
        ensure!(
            current.degree(axis) == 1 && guide.degree(axis) == 1,
            "graph33 chain is not linear"
        );
        let leading = current.derivative(axis);
        let endpoint = current.replace(axis, &Rational::zero());
        let c = guide.derivative(axis);
        let d = guide.replace(axis, &Rational::zero());
        ensure!(
            sign(&c) == 1 && sign(&d) == 1,
            "graph33 C/D positivity failed"
        );
        let comparison = &endpoint * &c - &leading * &d;
        let factors = comparison.factor();
        let rebuilt = factors
            .iter()
            .fold(comparison.one(), |acc, (factor, multiplicity)| {
                &acc * &factor.pow(*multiplicity)
            });
        ensure!(
            rebuilt == comparison,
            "graph33 comparison reconstruction failed"
        );
        let known_sign = factors
            .iter()
            .filter(|(_, multiplicity)| multiplicity % 2 == 1)
            .fold(1_i8, |acc, (factor, _)| acc * sign(factor));
        ensure!(
            comparison.is_zero() || known_sign == 1,
            "graph33 comparison nonnegativity failed"
        );
        chain.push(json!({"axis":a.names[axis],"polynomial":a.format(&current),"guide":a.format(&guide),"leading":a.format(&leading),"endpoint":a.format(&endpoint),"C":a.format(&c),"D":a.format(&d),"comparison_BC_minus_AD":a.format(&comparison),"comparison_factors":factors.iter().map(|(q,m)|json!({"polynomial":a.format(q),"multiplicity":m,"coefficient_sign":sign(q)})).collect::<Vec<_>>(),"comparison_nonnegative":true,"C_D_strictly_positive":true,"identity_verified":true}));
        current = leading;
        guide = c;
    }
    let group_a = a.parse("x0+x2+x7")?;
    let group_b = a.parse("x1+x4+x6")?;
    let cone = &(&group_a * &group_a) + &(&group_b * &group_b)
        - (&group_a * &group_b).mul_coeff(Rational::from(8));
    ensure!(current == cone, "graph33 cone residual identity failed");
    ensure!(guide == mass, "graph33 residual guide identity failed");
    Ok(
        json!({"source":"https://arxiv.org/html/2201.02576v2#A1.SS3","paper_propagator_to_fixture":["x5","x0","x6","x1","x2","x3","x4","x7"],"U":a.format(&u),"V":a.format(&v),"mass_sum":a.format(&mass),"F_equals_mass_U_minus_10V_verified":true,"direct_minors":direct,"negative_chain":chain,"residual":a.format(&current),"residual_guide":a.format(&guide),"residual_terms":current.nterms(),"residual_identity":"F_x3x5 = A^2-8*A*B+B^2; A=x0+x2+x7, B=x1+x4+x6","residual_identity_verified":true,"candidate_original_order":["x6","x1","x4","x0","x2","x7","x5","x3"],"scope":"Exact original graph and two-step comparison identities only; no reduced base decomposition or original coverage certificate."}),
    )
}

fn graph_minors(a: &Algebra, p: &Poly) -> Result<Value> {
    ensure!(
        a.names == ["x0", "x1", "x2", "x4", "x5", "x6", "x7"],
        "source graph requires the exact seven original coordinates"
    );
    let ma = a.parse("x0+x2+x5+x6")?;
    let mb = a.parse("-x0-x6")?;
    let mc = a.parse("-x2")?;
    let md = a.parse("x0+x1+1+x6")?;
    let me = a.parse("-1")?;
    let mf = a.parse("x2+1+x4+x7")?;
    let adj = [
        [
            &md * &mf - &me * &me,
            &mc * &me - &mb * &mf,
            &mb * &me - &mc * &md,
        ],
        [
            &mc * &me - &mb * &mf,
            &ma * &mf - &mc * &mc,
            &mb * &mc - &ma * &me,
        ],
        [
            &mb * &me - &mc * &md,
            &mb * &mc - &ma * &me,
            &ma * &md - &mb * &mb,
        ],
    ];
    let u = &(&ma * &adj[0][0]) + &(&mb * &adj[0][1]) + (&mc * &adj[0][2]);
    ensure!(
        sign(&u) == 1,
        "loop determinant is not coefficient-positive"
    );
    let j = [a.parse("x5+x6")?, a.parse("-x6")?, a.parse("x7")?];
    let adj_j = adj
        .iter()
        .map(|row| {
            row.iter()
                .zip(&j)
                .fold(p.zero(), |acc, (coefficient, component)| {
                    &acc + &(coefficient * component)
                })
        })
        .collect::<Vec<_>>();
    let jt_adj_j = j
        .iter()
        .zip(&adj_j)
        .fold(p.zero(), |acc, (component, value)| {
            &acc + &(component * value)
        });
    let v = &a.parse("x5+x6+x7")? * &u - jt_adj_j;
    ensure!(
        &u - &v == *p,
        "source momentum determinant does not reconstruct exact fixture F"
    );
    let loop_vectors = [
        [1, -1, 0],
        [0, 1, 0],
        [1, 0, -1],
        [0, 0, 1],
        [1, 0, 0],
        [1, -1, 0],
        [0, 0, 1],
    ];
    let external = [0, 0, 0, 0, 1, 1, 1];
    let mut minors = Vec::new();
    for axis in 0..p.nvars() {
        let mut h = u.clone().mul_coeff(Rational::from(external[axis]));
        for (component, weight) in adj_j.iter().zip(loop_vectors[axis]) {
            h = &h - &component.clone().mul_coeff(Rational::from(weight));
        }
        let delta = &p.derivative(axis) * &u - p * &u.derivative(axis);
        ensure!(
            &delta + &(&h * &h) == p.zero(),
            "Rayleigh square identity failed"
        );
        ensure!(
            h.degree(axis) == 0,
            "square numerator retained eliminated coordinate"
        );
        let u_leading = u.derivative(axis);
        let u_endpoint = u.replace(axis, &Rational::zero());
        ensure!(
            sign(&u_leading) == 1 && sign(&u_endpoint) == 1,
            "strict positive U coefficients missing"
        );
        let factors = h.factor();
        let rebuilt = factors.iter().fold(h.one(), |acc, (factor, multiplicity)| {
            &acc * &factor.pow(*multiplicity)
        });
        ensure!(rebuilt == h, "square numerator factor identity failed");
        minors.push(json!({"axis":a.names[axis],"square_numerator":a.format(&h),"square_numerator_terms":h.nterms(),"square_numerator_coefficient_sign":sign(&h),"factors":factors.iter().map(|(q,m)|json!({"polynomial":a.format(q),"multiplicity":m})).collect::<Vec<_>>(),"U_leading":a.format(&u_leading),"U_endpoint":a.format(&u_endpoint),"both_U_coefficients_strict_positive":true,"identity":"F_i*U-F*U_i = -H_i^2","identity_verified":true,"forbidden_corner":"F=A*x_i+B cannot have A>=0,B<0 or A>0,B<=0 on positive base"}));
    }
    let mut chain = Vec::new();
    let mut current = p.clone();
    let mut guide = u.clone();
    for name in ["x0", "x4", "x1", "x5"] {
        let axis = a.names.iter().position(|v| v == name).unwrap();
        ensure!(
            current.degree(axis) == 1 && guide.degree(axis) <= 1,
            "chain lost linearity"
        );
        let leading = current.derivative(axis);
        let endpoint = current.replace(axis, &Rational::zero());
        let c = guide.derivative(axis);
        let d = guide.replace(axis, &Rational::zero());
        let comparison = &endpoint * &c - &leading * &d;
        let factors = comparison.factor();
        let rebuilt = factors
            .iter()
            .fold(comparison.one(), |acc, (factor, multiplicity)| {
                &acc * &factor.pow(*multiplicity)
            });
        ensure!(
            rebuilt == comparison,
            "chain comparison factor reconstruction failed"
        );
        let mut known_sign = 1i8;
        for (factor, multiplicity) in &factors {
            if multiplicity % 2 == 1 {
                known_sign *= sign(factor);
            }
        }
        let comparison_nonnegative = comparison.is_zero() || known_sign == 1;
        let comparison_proved = sign(&c) == 1 && sign(&d) == 1 && comparison_nonnegative;
        let leading_negative = sign(&leading) == -1;
        chain.push(json!({"axis":name,"polynomial":a.format(&current),"guide":a.format(&guide),"leading":a.format(&leading),"endpoint":a.format(&endpoint),"C":a.format(&c),"D":a.format(&d),"comparison_BC_minus_AD":a.format(&comparison),"comparison_factors":factors.iter().map(|(q,m)|json!({"polynomial":a.format(q),"multiplicity":m,"coefficient_sign":sign(q)})).collect::<Vec<_>>(),"comparison_nonnegative":comparison_nonnegative,"comparison_proved":comparison_proved,"leading_strictly_negative":leading_negative,"generic_negative_fiber":"A<0 and z>largest_real_root(z*(A*z+B))","identity_verified":true}));
        ensure!(
            comparison_proved || leading_negative,
            "candidate chain step lacks exact sign proof"
        );
        current = leading;
        guide = c;
    }
    ensure!(
        sign(&current) == -1,
        "candidate chain endpoint is not strictly negative"
    );
    Ok(
        json!({"source":"https://arxiv.org/html/2201.02576v2#S3.E4","U":a.format(&u),"V":a.format(&v),"F_equals_U_minus_V_verified":true,"U_strict_coefficient_positive":true,"minors":minors,"negative_chain":chain,"negative_chain_endpoint":a.format(&current),"candidate_original_order":["x2","x6","x7","x5","x1","x4","x0"],"scope":"Exact graph and hereditary comparison identities only. This report does not construct original-coordinate CAD cells or validate a largest-root output certificate."}),
    )
}

fn corner_search(a: &Algebra, p: &Poly, count: usize) -> Result<Value> {
    ensure!(count <= 65_536, "corner sample cap exceeded");
    let derivatives = (0..p.nvars())
        .map(|axis| p.derivative(axis))
        .collect::<Vec<_>>();
    let mut witnesses = BTreeMap::<String, Value>::new();
    let mut seed = 0_u64;
    let mut tested = 0;
    for sample in 0..count {
        let mut point = Vec::new();
        for _ in 0..p.nvars() {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let value = if sample % 2 == 0 {
                let exponent = ((seed >> 16) % 25) as i32 - 12;
                let mantissa = ((seed >> 40) % 9 + 1) as i64;
                if exponent >= 0 {
                    Rational::new(mantissa << exponent, 8)
                } else {
                    Rational::new(mantissa, 8_i64 << (-exponent))
                }
            } else {
                Rational::new(((seed >> 32) % 2048 + 1) as i64, 512)
            };
            point.push(value);
        }
        let value = p.replace_all(&point);
        tested += 1;
        for (axis, derivative) in derivatives.iter().enumerate() {
            if witnesses.contains_key(&a.names[axis]) {
                continue;
            }
            let leading = derivative.replace_all(&point);
            let endpoint = &value - &(&leading * &point[axis]);
            if leading <= Rational::zero() || endpoint >= Rational::zero() {
                continue;
            }
            let root = -endpoint.clone() / leading.clone();
            let mut root_point = point.clone();
            root_point[axis] = root.clone();
            ensure!(
                p.replace_all(&root_point).is_zero(),
                "exact crossing reconstruction failed"
            );
            ensure!(
                derivative.replace_all(&root_point) > Rational::zero(),
                "crossing slope changed"
            );
            witnesses.insert(a.names[axis].clone(), json!({"sample_number":sample,"base_point":point.iter().map(ToString::to_string).collect::<Vec<_>>(),"leading":leading.to_string(),"endpoint":endpoint.to_string(),"positive_root":root.to_string(),"root_point":root_point.iter().map(ToString::to_string).collect::<Vec<_>>(),"root_and_positive_slope_verified":true}));
        }
        if witnesses.len() == p.nvars() {
            break;
        }
    }
    Ok(
        json!({"sample_count":tested,"seed":0,"axes_with_increasing_positive_zero":witnesses,"every_axis_counterexample":witnesses.len()==p.nvars(),"scope":"A found rational A>0,B<0 witness disproves globally decreasing zero crossings for that axis. Failure to find one is not a certificate."}),
    )
}
fn sign(p: &Poly) -> i8 {
    if !p.is_zero() && p.coefficients.iter().all(|c| c > &Rational::zero()) {
        1
    } else if !p.is_zero() && p.coefficients.iter().all(|c| c < &Rational::zero()) {
        -1
    } else {
        0
    }
}
fn positive_coordinate(p: &Poly, axis: usize) -> bool {
    p.nterms() == 1
        && p.coefficients[0] > Rational::zero()
        && p.exponents_iter().next().is_some_and(|e| {
            e.iter()
                .enumerate()
                .all(|(j, &d)| d == u32::from(j == axis))
        })
}
fn homogeneous_piece(p: &Poly, degree: u32) -> Poly {
    let mut result = p.zero();
    for (c, m) in p.coefficients.iter().zip(p.exponents_iter()) {
        if m.iter().sum::<u32>() == degree {
            result.append_monomial(c.clone(), m);
        }
    }
    result
}
fn screen(a: &Algebra, p: &Poly, axis: usize) -> Result<Value> {
    ensure!(p.degree(axis) == 1, "requires a linear axis");
    let leading = p.derivative(axis);
    let endpoint = p.replace(axis, &Rational::zero());
    ensure!(
        &(&a.parse(&a.names[axis])? * &leading) + &endpoint == *p,
        "linear identity failed"
    );
    let axes = (0..p.nvars()).filter(|&v| v != axis).collect::<Vec<_>>();
    let mut witnesses = Vec::new();
    let mut closest: Option<(usize, Value)> = None;
    let positives = a
        .names
        .iter()
        .map(|v| a.parse(v))
        .collect::<Result<Vec<_>>>()?;
    let domain = symgcad::domain::derive_domain(a, &positives, &[])?;
    let options = symgcad::domain::CertificateOptions::default();
    let mut constant_certificates = Vec::new();
    for weight in [
        Rational::new(1, 4),
        Rational::new(1, 2),
        Rational::one(),
        Rational::from(2),
        Rational::from(4),
    ] {
        for endpoint_sign in [-1i8, 1] {
            let residual = leading.clone().mul_coeff(weight.clone())
                + endpoint.clone().mul_coeff(Rational::from(endpoint_sign));
            if let Some(certificate) =
                symgcad::domain::certify_nonzero(a, &residual, &domain, &options)?
            {
                symgcad::domain::verify_certificate(a, &residual, &positives, &[], &certificate)?;
                constant_certificates.push(json!({"multiplier":weight.to_string(),"endpoint_sign":endpoint_sign,"residual":a.format(&residual),"certificate":certificate,"independently_replayed":true}));
            }
        }
    }
    // Exact rational counterexamples establish which sign corners really occur.
    // If all four occur, no globally positive h can make either h*A+B or h*A-B
    // strictly one-signed on the whole original positive base.
    let mut corners = BTreeMap::<String, Value>::new();
    let values = [Rational::new(1, 8), Rational::one(), Rational::from(8)];
    for mut code in 0..3usize.pow(axes.len() as u32) {
        let mut point = vec![Rational::one(); p.nvars()];
        for &variable in &axes {
            point[variable] = values[code % 3].clone();
            code /= 3;
        }
        let av = leading.replace_all(&point);
        let bv = endpoint.replace_all(&point);
        if av.is_zero() || bv.is_zero() {
            continue;
        }
        let key = format!(
            "{},{}",
            if av > Rational::zero() { 1 } else { -1 },
            if bv > Rational::zero() { 1 } else { -1 }
        );
        corners.entry(key).or_insert_with(||json!({"point":point.iter().map(ToString::to_string).collect::<Vec<_>>(),"leading_value":av.to_string(),"endpoint_value":bv.to_string()}));
        if corners.len() == 4 {
            break;
        }
    }
    for mask in 1..(1usize << (axes.len() + 1)) {
        let mut h = if mask & (1 << axes.len()) != 0 {
            p.one()
        } else {
            p.zero()
        };
        for (i, &v) in axes.iter().enumerate() {
            if mask & (1 << i) != 0 {
                let mut e = vec![0; p.nvars()];
                e[v] = 1;
                h.append_monomial(Rational::one(), &e);
            }
        }
        ensure!(
            sign(&h) == 1 && h.degree(axis) == 0,
            "multiplier domain proof failed"
        );
        for endpoint_sign in [-1i8, 1] {
            let residual =
                &h * &leading + endpoint.clone().mul_coeff(Rational::from(endpoint_sign));
            let residual_sign = sign(&residual);
            let pos = residual
                .coefficients
                .iter()
                .filter(|c| **c > Rational::zero())
                .count();
            let neg = residual
                .coefficients
                .iter()
                .filter(|c| **c < Rational::zero())
                .count();
            let candidate = json!({"multiplier":a.format(&h),"endpoint_sign":endpoint_sign,"residual":a.format(&residual),"residual_sign":residual_sign,"positive_terms":pos,"negative_terms":neg,"exact_identity":true});
            if residual_sign != 0 {
                witnesses.push(candidate.clone());
            }
            let score = pos.min(neg);
            if closest
                .as_ref()
                .is_none_or(|(previous, _)| score < *previous)
            {
                closest = Some((score, candidate));
            }
        }
    }
    let factors = |p: &Poly| -> Result<Vec<Value>> {
        Ok(factors_checked(std::slice::from_ref(p))?
            .into_iter()
            .map(|q| json!({"polynomial":a.format(&q)}))
            .collect())
    };
    Ok(
        json!({"axis":a.names[axis],"leading":a.format(&leading),"endpoint":a.format(&endpoint),"leading_terms":leading.nterms(),"endpoint_terms":endpoint.nterms(),"leading_factors":factors(&leading)?,"endpoint_factors":factors(&endpoint)?,"witnesses":witnesses,"constant_certificates":constant_certificates,"joint_sign_witnesses":corners,"all_four_joint_signs_occur":corners.len()==4,"closest":closest.map(|(_,v)|v),"candidate_count":2*((1usize<<(axes.len()+1))-1)}),
    )
}
fn main() -> Result<()> {
    let args = Args::parse();
    let started = Instant::now();
    ensure!(!args.output.exists(), "output must be fresh");
    if !args.worker {
        let mut command = Command::new(std::env::current_exe()?);
        command
            .arg(&args.input)
            .arg("--output")
            .arg(&args.output)
            .arg("--worker");
        if args.anchor_minors {
            command.arg("--anchor-minors");
        }
        if args.corner_samples != 0 {
            command
                .arg("--corner-samples")
                .arg(args.corner_samples.to_string());
        }
        if args.graph_minors {
            command.arg("--graph-minors");
        }
        if args.graph33_minors {
            command.arg("--graph33-minors");
        }
        let process = symgcad::benchmark::run_bounded(
            &mut command,
            &symgcad::Limits {
                wall_time_secs: 60.0,
                memory_mib: 4096,
                workers: 1,
                max_cells: None,
            },
        )?;
        if let Some(parent) = args.output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            args.output.with_extension("process.json"),
            serde_json::to_vec_pretty(&process)?,
        )?;
        ensure!(
            process.status == "completed" && process.exit_code == Some(0),
            "bounded research worker did not complete: {}",
            process.status
        );
        return Ok(());
    }
    let bytes = fs::read(&args.input)?;
    let problem = Problem::from_toml(std::str::from_utf8(&bytes)?)?;
    ensure!(
        problem.variables.len() <= 8 && problem.split.is_empty(),
        "bounded unsplit problem required"
    );
    let n = normalize(&problem, &problem.variables)?;
    for axis in 0..problem.variables.len() {
        ensure!(
            n.positives.iter().any(|p| positive_coordinate(p, axis)),
            "original positive coordinate premise missing"
        );
    }
    let targets = n
        .positives
        .iter()
        .filter(|p| !(0..problem.variables.len()).any(|axis| positive_coordinate(p, axis)))
        .collect::<Vec<_>>();
    ensure!(targets.len() == 1, "exactly one target constraint required");
    // Original Gluza30 negative fixture normalizes to -F>0.
    let p = targets[0].clone().mul_coeff(Rational::from(-1));
    if args.graph33_minors {
        ensure!(
            p.nterms() <= 256 && (0..p.nvars()).all(|v| p.degree(v) <= 2),
            "bounded graph33 input required"
        );
        let report = json!({"source":args.input,"source_sha256":format!("{:x}",Sha256::digest(&bytes)),"polynomial":n.algebra.format(&p),"orientation_relative_normalized_target":-1,"graph":graph33_minors(&n.algebra,&p)?,"elapsed_seconds":started.elapsed().as_secs_f64(),"original_completion":false});
        if let Some(parent) = args.output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
        return Ok(());
    }
    ensure!(
        p.nterms() <= 256 && (0..p.nvars()).all(|v| p.degree(v) == 1),
        "bounded multiaffine target required"
    );
    if args.graph_minors {
        let report = json!({"source":args.input,"source_sha256":format!("{:x}",Sha256::digest(&bytes)),"polynomial":n.algebra.format(&p),"orientation_relative_normalized_target":-1,"graph":graph_minors(&n.algebra,&p)?,"elapsed_seconds":started.elapsed().as_secs_f64(),"original_completion":false});
        if let Some(parent) = args.output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
        return Ok(());
    }
    if args.corner_samples != 0 {
        let report = json!({"source":args.input,"source_sha256":format!("{:x}",Sha256::digest(&bytes)),"polynomial":n.algebra.format(&p),"orientation_relative_normalized_target":-1,"countersearch":corner_search(&n.algebra,&p,args.corner_samples)?,"elapsed_seconds":started.elapsed().as_secs_f64(),"original_completion":false});
        if let Some(parent) = args.output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
        return Ok(());
    }
    let parts = (0..=4)
        .map(|d| homogeneous_piece(&p, d))
        .collect::<Vec<_>>();
    let mut restored = p.zero();
    for q in &parts {
        restored = &restored + q;
    }
    ensure!(restored == p, "total-degree decomposition failed");
    let mut euler = p.zero();
    for axis in 0..p.nvars() {
        euler = &euler + &(&n.algebra.parse(&n.algebra.names[axis])? * &p.derivative(axis));
    }
    ensure!(
        &euler - &p.clone().mul_coeff(Rational::from(3)) == &parts[4] - &parts[2],
        "Euler identity failed"
    );
    let axes = (0..p.nvars())
        .map(|axis| screen(&n.algebra, &p, axis))
        .collect::<Result<Vec<_>>>()?;
    let mut report = json!({"source":args.input,"source_sha256":format!("{:x}",Sha256::digest(bytes)),"polynomial":n.algebra.format(&p),"orientation_relative_normalized_target":-1,"axes":axes,"homogeneous_parts":parts.iter().enumerate().map(|(d,q)|json!({"degree":d,"polynomial":n.algebra.format(q),"coefficient_sign":sign(q)})).collect::<Vec<_>>(),"euler_identity":"sum_i x_i*dF/dx_i - 3*F = P4-P2","euler_identity_verified":true,"no_positive_critical_zero":sign(&parts[2])==1 && sign(&parts[4])==-1,"elapsed_seconds":started.elapsed().as_secs_f64(),"original_completion":false,"scope":"Exact coefficient identities and positive-orthant coefficient-sign witnesses only; no coordinate transport or cell coverage proof."});
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
    if args.anchor_minors {
        let ft = &parts[2].clone().mul_coeff(Rational::from(2)) + &parts[3];
        let positives = n
            .algebra
            .names
            .iter()
            .map(|v| n.algebra.parse(v))
            .collect::<Result<Vec<_>>>()?;
        let domain = symgcad::domain::derive_domain(&n.algebra, &positives, &[])?;
        report["restored_anchor_derivative"] = json!(n.algebra.format(&ft));
        report["anchor_minors_complete"] = json!(false);
        report["anchor_minors"] = json!([]);
        for axis in 0..p.nvars() {
            let delta = &p.derivative(axis) * &ft - &p * &ft.derivative(axis);
            ensure!(
                delta.degree(axis) == 0,
                "linear coordinate did not cancel from anchor minor"
            );
            report["active_anchor_minor"] =
                json!({"axis":n.algebra.names[axis],"terms":delta.nterms()});
            fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
            let factors = delta.factor();
            let rebuilt = factors
                .iter()
                .fold(delta.one(), |v, (q, m)| &v * &q.pow(*m));
            ensure!(
                rebuilt == delta,
                "anchor-minor factor reconstruction failed"
            );
            let mut factors_report = Vec::new();
            let mut known_sign = 1i8;
            let mut strict = true;
            for (q, m) in factors {
                let certificate = symgcad::domain::certify_nonzero(
                    &n.algebra,
                    &q,
                    &domain,
                    &symgcad::domain::CertificateOptions::default(),
                )?;
                if let Some(c) = &certificate {
                    symgcad::domain::verify_certificate(&n.algebra, &q, &positives, &[], c)?;
                    if m % 2 == 1 {
                        known_sign *= c.sign;
                    }
                } else if m % 2 == 0 {
                    strict = false;
                } else {
                    known_sign = 0;
                    strict = false;
                }
                factors_report.push(json!({"polynomial":n.algebra.format(&q),"multiplicity":m,"certificate":certificate}));
            }
            report["anchor_minors"].as_array_mut().unwrap().push(json!({"axis":n.algebra.names[axis],"polynomial":n.algebra.format(&delta),"terms":delta.nterms(),"coefficient_sign":sign(&delta),"factors":factors_report,"factor_identity_verified":true,"certified_sign":known_sign,"strict":strict}));
            fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
        }
        report["anchor_minors_complete"] = json!(true);
        report["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
        fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linear_corner_implication_can_hold_with_changing_slope() {
        let a = Algebra::new(&["x".into(), "z".into()]).unwrap();
        let p = a.parse("(x-1)*z+x^2-2*x").unwrap();
        let row = screen(&a, &p, 1).unwrap();
        assert!(
            row["witnesses"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["endpoint_sign"] == -1
                    && v["residual_sign"] == 1
                    && a.parse(v["multiplier"].as_str().unwrap()).unwrap()
                        == a.parse("x").unwrap())
        );
        assert_eq!(sign(&p.derivative(1)), 0);
    }
    #[test]
    fn nonstrict_residual_is_not_a_certificate() {
        let a = Algebra::new(&["x".into(), "z".into()]).unwrap();
        let row = screen(&a, &a.parse("(x-1)*z+x*(x-1)").unwrap(), 1).unwrap();
        assert!(row["witnesses"].as_array().unwrap().iter().all(|v|a.parse(v["residual"].as_str().unwrap()).unwrap()!=a.parse("0").unwrap()));
    }
    #[test]
    fn total_degree_pieces_reconstruct_radial_euler_identity() {
        let a = Algebra::new(&["x".into(), "y".into()]).unwrap();
        let p = a.parse("x*y+x^2*y-x^2*y^2").unwrap();
        let e = &a.parse("x").unwrap() * &p.derivative(0)
            + &a.parse("y").unwrap() * &p.derivative(1)
            - p.clone().mul_coeff(Rational::from(3));
        assert_eq!(e, &homogeneous_piece(&p, 4) - &homogeneous_piece(&p, 2));
    }
    #[test]
    fn increasing_crossing_counterexample_is_an_exact_positive_zero() {
        let a = Algebra::new(&["x".into(), "z".into()]).unwrap();
        let p = a.parse("x*z-1").unwrap();
        let report = corner_search(&a, &p, 1).unwrap();
        assert_eq!(report["every_axis_counterexample"], true);
        for (_, row) in report["axes_with_increasing_positive_zero"]
            .as_object()
            .unwrap()
        {
            assert_eq!(row["root_and_positive_slope_verified"], true);
        }
    }
}
