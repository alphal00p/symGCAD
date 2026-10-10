//! Bounded research measurement of exact linear-factor cache overhead.
//! Retains source-record hashes; no solver completion or timing-parity claim.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf, process::Command, time::Instant};
use symgcad::{
    algebra::{Algebra, Poly, canonical},
    optimized_projection::{OptimizedProjectionOptions, ProjectionCache},
};

#[derive(Parser)]
struct Args {
    directory: PathBuf,
    #[arg(long)]
    output: PathBuf,
    /// Replay exactly the source list in a previous report.
    #[arg(long)]
    manifest: Option<PathBuf>,
    #[arg(long, default_value_t = 1000)]
    cases: usize,
    #[arg(long, default_value_t = 10000)]
    scan: usize,
    #[arg(long, default_value_t = 3)]
    repeats: usize,
    #[arg(long, default_value_t = 60.0)]
    seconds: f64,
    #[arg(long, hide = true)]
    worker: bool,
}

#[derive(Deserialize)]
struct Operation {
    operation: String,
    variable_names: Vec<String>,
    input_hash: String,
    output: Vec<String>,
    multiplicities: Option<Vec<usize>>,
    output_hash: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct Source {
    path: PathBuf,
    sha256: String,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn eligible(record: &Operation) -> Result<Option<(Algebra, Poly)>> {
    if record.operation != "factor"
        || record.output.len() != 1
        || record.multiplicities.as_deref() != Some(&[1])
    {
        return Ok(None);
    }
    ensure!(
        hash(&serde_json::to_vec(&record.output)?) == record.input_hash
            && hash(&serde_json::to_vec(&(
                &record.output,
                &record.multiplicities
            ))?) == record.output_hash,
        "source factor/input hashes do not reconstruct the same polynomial"
    );
    let algebra = Algebra::new(&record.variable_names)?;
    let p = algebra.parse(&record.output[0])?;
    // Independently check Gauss's lemma with Symbolica's exact GCD: a linear
    // polynomial with monomial leading coefficient is irreducible if that
    // coefficient is coprime with the polynomial. The producer instead uses
    // exponent minima, so the selection does not copy its implementation.
    if p.is_constant()
        || p.nterms().saturating_mul(p.nvars()) > 256
        || !(0..p.nvars()).any(|i| {
            if p.degree(i) != 1 {
                return false;
            }
            let leading = p.univariate_lcoeff(i);
            leading.nterms() == 1 && p.gcd(&leading).is_constant()
        })
    {
        return Ok(None);
    }
    ensure!(canonical(&p) == p, "source factor was not canonical");
    Ok(Some((algebra, p)))
}

fn work(args: &Args) -> Result<()> {
    let previous: Option<serde_json::Value> = args
        .manifest
        .as_ref()
        .map(|p| -> Result<_> { Ok(serde_json::from_slice(&fs::read(p)?)?) })
        .transpose()?;
    let listed: Vec<Source> = if let Some(report) = &previous {
        serde_json::from_value(report["sources"].clone())?
    } else {
        // Enumeration is bounded. Its order is not assumed portable: the exact
        // selected list and byte hashes are retained and reused for comparisons.
        fs::read_dir(&args.directory)?
            .take(args.scan)
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .map(|e| Source {
                path: e.path(),
                sha256: String::new(),
            })
            .collect()
    };
    ensure!(listed.len() <= 10000, "manifest exceeds scan cap");
    let mut sources = Vec::new();
    let mut inputs = Vec::new();
    let mut seen = BTreeSet::new();
    let mut algebra: Option<Algebra> = None;
    let mut scanned = 0;
    for source in listed {
        if inputs.len() == args.cases {
            break;
        }
        scanned += 1;
        if fs::metadata(&source.path)?.len() > 65536 {
            continue;
        }
        let bytes = fs::read(&source.path)?;
        let sha256 = hash(&bytes);
        ensure!(
            source.sha256.is_empty() || source.sha256 == sha256,
            "source changed"
        );
        let record: Operation = serde_json::from_slice(&bytes)?;
        if let Some((a, p)) = eligible(&record)? {
            if algebra.as_ref().is_some_and(|old| old.names != a.names) {
                continue;
            }
            if seen.insert(record.input_hash) {
                if algebra.is_none() {
                    algebra = Some(a);
                }
                inputs.push(p);
                sources.push(Source {
                    path: source.path,
                    sha256,
                });
            }
        }
    }
    ensure!(!inputs.is_empty(), "no eligible linear factors");
    if let Some(report) = &previous {
        ensure!(
            sources.len() == report["sources"].as_array().unwrap().len(),
            "comparison did not replay every source"
        );
    }
    let algebra = algebra.unwrap();
    let mut timings = Vec::new();
    let caches = args.output.with_extension("caches");
    ensure!(!caches.exists(), "refusing to reuse prior caches");
    fs::create_dir_all(&caches)?;
    for repetition in 0..args.repeats {
        for persistent in [false, true] {
            let directory = caches.join(format!("{repetition}-{persistent}"));
            let options = OptimizedProjectionOptions {
                checkpoint_dir: persistent.then(|| directory.clone()),
                ..Default::default()
            };
            let mut cache = ProjectionCache::new(&algebra, &options);
            let start = Instant::now();
            for p in &inputs {
                ensure!(
                    cache.factor_basis(&[p.clone()])? == [p.clone()],
                    "factor changed"
                );
            }
            let cold_seconds = start.elapsed().as_secs_f64();
            let cold_metrics = cache.report.cache.clone();
            let start = Instant::now();
            for p in &inputs {
                ensure!(
                    cache.factor_basis(&[p.clone()])? == [p.clone()],
                    "warm factor changed"
                );
            }
            let warm_seconds = start.elapsed().as_secs_f64();
            drop(cache);
            let mut replay = ProjectionCache::new(&algebra, &options);
            let start = Instant::now();
            for p in &inputs {
                ensure!(
                    replay.factor_basis(&[p.clone()])? == [p.clone()],
                    "replay factor changed"
                );
            }
            let replay_seconds = start.elapsed().as_secs_f64();
            timings.push(json!({"repetition":repetition,"persistent":persistent,
                "cold_seconds":cold_seconds,"cold_metrics":cold_metrics,
                "warm_seconds":warm_seconds,
                "replay_seconds":replay_seconds,"replay_metrics":replay.report.cache}));
        }
    }
    let report = json!({"schema_version":1,"kind":"linear_factor_cache_diagnostic",
        "scanned":scanned,"cases":inputs.len(),"sources":sources,"timings":timings,
        "manifest_sha256":args.manifest.as_ref().map(|p|fs::read(p).map(|b|hash(&b))).transpose()?,
        "coverage_change":0,"note":"Exact factor identity and cache overhead only; not whole-solver timings. Cold namespace and fresh cache object per measurement; replay uses prior persistent files."});
    fs::write(&args.output, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        (1..=10000).contains(&args.cases) && (1..=10000).contains(&args.scan),
        "invalid scan limit"
    );
    ensure!((1..=3).contains(&args.repeats), "invalid repetition count");
    ensure!(
        args.seconds.is_finite() && args.seconds > 0.0 && args.seconds <= 60.0,
        "invalid time limit"
    );
    if args.worker {
        return work(&args);
    }
    let sidecar = args.output.with_extension("process.json");
    ensure!(
        !args.output.exists() && !sidecar.exists(),
        "refusing to overwrite evidence"
    );
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    let executable = std::env::current_exe()?;
    let mut command = Command::new(&executable);
    command
        .args(std::env::args_os().skip(1))
        .arg("--worker")
        .env("RAYON_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1");
    let limits = symgcad::Limits {
        wall_time_secs: args.seconds,
        memory_mib: 2048,
        workers: 1,
        max_cells: None,
    };
    let process = symgcad::benchmark::run_bounded(&mut command, &limits)?;
    let report = json!({"schema_version":1,"binary_sha256":hash(&fs::read(executable)?),
        "limits":limits,"process":process});
    fs::write(sidecar, serde_json::to_vec_pretty(&report)?)?;
    ensure!(
        process.status == "completed" && process.exit_code == Some(0) && !process.output_truncated,
        "bounded diagnostic did not complete"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(expression: &str) -> Operation {
        let a = Algebra::new(&["s".into(), "x".into()]).unwrap();
        let output = vec![a.format(&canonical(&a.parse(expression).unwrap()))];
        let multiplicities = Some(vec![1]);
        Operation {
            operation: "factor".into(),
            variable_names: a.names,
            input_hash: hash(&serde_json::to_vec(&output).unwrap()),
            output_hash: hash(&serde_json::to_vec(&(&output, &multiplicities)).unwrap()),
            output,
            multiplicities,
        }
    }
    #[test]
    fn selection_checks_exact_linear_identity_and_hashes() {
        assert!(eligible(&record("3*s-2*x-7")).unwrap().is_some());
        assert!(eligible(&record("s^2+x")).unwrap().is_some());
        assert!(eligible(&record("s*x+1")).unwrap().is_some());
        for p in ["s*x+x", "s^2+x^2", "3"] {
            assert!(eligible(&record(p)).unwrap().is_none());
        }
        let mut r = record("s-x");
        r.input_hash = "tampered".into();
        assert!(eligible(&r).is_err());
    }
}
