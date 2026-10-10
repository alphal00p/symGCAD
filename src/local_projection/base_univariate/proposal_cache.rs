//! Untrusted restart proposals for the separately checked base support basis.
//! No polynomial ID, root interval, domain fact or factorization claim is saved.
use super::*;
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const SCHEMA: u32 = 1;
const KIND: &str = "symgcad-base-support-proposal-v1";
const MAX_BYTES: usize = 32 * 1024 * 1024;
// ceil(20000 * log10(2)); exact bit limits remain mandatory after parsing.
const MAX_DIGITS: usize = 6021;
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    schema_version: u32,
    kind: String,
    authority: String,
    backend_version: String,
    coefficient_domain: String,
    variable_names: Vec<String>,
    axis: usize,
    source_sha256: String,
    policy_sha256: String,
}

pub(super) struct Request {
    binding: Binding,
    path: PathBuf,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Term {
    coefficient: String,
    exponent: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Block {
    multiplicity: u32,
    terms: Vec<Term>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    binding: Binding,
    blocks: Vec<Block>,
    blocks_sha256: String,
}

pub(super) enum Load {
    Missing,
    Rejected,
    // A checksum and structural preflight do not establish this product.
    Proposed(Vec<(Poly, u32)>),
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn payload_digest(value: &impl Serialize) -> Result<String> {
    struct HashWriter {
        hash: Sha256,
        bytes: usize,
    }
    impl Write for HashWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.bytes.saturating_add(bytes.len()) > MAX_BYTES {
                return Err(std::io::Error::other("base proposal payload cap"));
            }
            self.bytes += bytes.len();
            self.hash.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = HashWriter {
        hash: Sha256::new(),
        bytes: 0,
    };
    serde_json::to_writer(&mut writer, value)?;
    Ok(format!("{:x}", writer.hash.finalize()))
}

fn policy_digest() -> Result<String> {
    Ok(digest(&serde_json::to_vec(&(
        MAX_SOURCES,
        MAX_RETAINED_BITS,
        MAX_DEGREE,
        MAX_TERMS,
        MAX_AXES,
        MAX_BLOCK_HEIGHT,
        MAX_SOURCE_HEIGHT,
        MAX_BLOCK_BITS,
        MAX_SOURCE_BITS,
        MAX_BLOCKS,
        MAX_TEXT,
        256_000_000u64,
        MAX_BYTES,
    ))?))
}

pub(super) fn request(
    directory: &Path,
    algebra: &Algebra,
    source_text: &str,
) -> Result<Option<Request>> {
    if !map_admissible(algebra)
        || algebra.names.iter().any(|name| name.len() > 1024)
        || source_text.len() > MAX_TEXT
    {
        return Ok(None);
    }
    let binding = Binding {
        schema_version: SCHEMA,
        kind: KIND.into(),
        authority: "unverified_product_proposal".into(),
        backend_version: symbolica::license::LicenseManager::get_version().into(),
        coefficient_domain: "Q".into(),
        variable_names: algebra.names.clone(),
        axis: 0,
        source_sha256: digest(source_text.as_bytes()),
        policy_sha256: policy_digest()?,
    };
    let key = digest(&serde_json::to_vec(&binding)?);
    Ok(Some(Request {
        binding,
        path: directory
            .join("base-support-v1")
            .join(format!("{key}.json")),
    }))
}

fn read(path: &Path, budget: &mut impl FnMut() -> Result<()>) -> Result<Vec<u8>> {
    use std::os::unix::fs::OpenOptionsExt;
    // Do not follow a substituted symlink or block opening a FIFO.
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= MAX_BYTES as u64,
        "base proposal file cap/type"
    );
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    let mut chunk = [0; 65536];
    loop {
        budget()?;
        let n = file.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        ensure!(
            bytes.len().saturating_add(n) <= MAX_BYTES,
            "base proposal grew beyond cap"
        );
        bytes.extend_from_slice(&chunk[..n]);
    }
    Ok(bytes)
}

fn parse(
    request: &Request,
    algebra: &Algebra,
    source: &Poly,
    bytes: &[u8],
) -> Result<Vec<(Poly, u32)>> {
    ensure!(bytes.len() <= MAX_BYTES, "base proposal byte cap");
    let record: Record = serde_json::from_slice(bytes)?;
    ensure!(
        record.binding == request.binding,
        "base proposal request mismatch"
    );
    ensure!(
        !record.blocks.is_empty() && record.blocks.len() <= MAX_BLOCKS,
        "base proposal block count"
    );
    ensure!(
        payload_digest(&record.blocks)? == record.blocks_sha256,
        "base proposal checksum"
    );
    let mut blocks = Vec::new();
    let mut bits = 0u64;
    for block in record.blocks {
        ensure!(
            block.multiplicity > 0 && block.multiplicity <= MAX_DEGREE,
            "base proposal multiplicity"
        );
        ensure!(
            !block.terms.is_empty() && block.terms.len() <= MAX_TERMS,
            "base proposal term count"
        );
        let mut polynomial = source.zero();
        let mut previous = None;
        for term in block.terms {
            let digits = term
                .coefficient
                .strip_prefix('-')
                .unwrap_or(&term.coefficient);
            ensure!(
                !digits.is_empty()
                    && digits.len() <= MAX_DIGITS
                    && digits.bytes().all(|byte| byte.is_ascii_digit()),
                "base proposal integer grammar"
            );
            ensure!(
                term.exponent <= MAX_DEGREE && previous.is_none_or(|old| old < term.exponent),
                "base proposal exponent order/cap"
            );
            previous = Some(term.exponent);
            let coefficient = crate::algebra::rational(&term.coefficient)?;
            ensure!(
                !coefficient.is_zero()
                    && coefficient.numerator_ref().significant_bits() <= MAX_BLOCK_HEIGHT
                    && coefficient.to_string() == term.coefficient,
                "base proposal coefficient cap/canonical spelling"
            );
            bits = bits.saturating_add(coefficient.numerator_ref().significant_bits() + 1);
            ensure!(
                bits <= MAX_BLOCK_BITS,
                "base proposal collective coefficient cap"
            );
            let mut exponents = vec![0; algebra.names.len()];
            exponents[0] = term.exponent;
            polynomial.append_monomial(coefficient, &exponents);
        }
        ensure!(
            cost(&polynomial).is_some() && canonical(&polynomial) == polynomial,
            "base proposal noncanonical block"
        );
        ensure!(
            !blocks.iter().any(|(old, _)| *old == polynomial),
            "duplicate base proposal block"
        );
        blocks.push((polynomial, block.multiplicity));
    }
    blocks.sort_by(|a, b| a.0.internal_cmp(&b.0));
    // All caps precede powers. This does not check the product identity yet.
    product_preflight(source, &blocks, || Ok(()))?;
    Ok(blocks)
}

pub(super) fn load(
    request: &Request,
    algebra: &Algebra,
    source: &Poly,
    budget: &mut impl FnMut() -> Result<()>,
) -> Result<Load> {
    let bytes = match read(&request.path, budget) {
        Ok(bytes) => bytes,
        Err(error)
            if error
                .downcast_ref::<crate::resultant_interpolation::Stopped>()
                .is_some() =>
        {
            return Err(error);
        }
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
        {
            return Ok(Load::Missing);
        }
        Err(_) => return Ok(Load::Rejected),
    };
    budget()?;
    // Structural cache failures are misses; no algebra backend has been entered.
    let result = parse(request, algebra, source, &bytes);
    budget()?;
    Ok(match result {
        Ok(blocks) => Load::Proposed(blocks),
        Err(_) => Load::Rejected,
    })
}

struct Limited(Vec<u8>);
impl Write for Limited {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.0.len().saturating_add(bytes.len()) > MAX_BYTES {
            return Err(std::io::Error::other("base proposal serialization cap"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(super) fn save(
    request: &Request,
    source: &Poly,
    blocks: &[(Poly, u32)],
    budget: &mut impl FnMut() -> Result<()>,
) -> Result<()> {
    // Permit saving an unverified native proposal, but only after its complete
    // bounded shape preflight. Every subsequent load must replay the product.
    product_preflight(source, blocks, &mut *budget)?;
    let mut saved = Vec::new();
    for (polynomial, multiplicity) in blocks {
        budget()?;
        saved.push(Block {
            multiplicity: *multiplicity,
            terms: polynomial
                .coefficients
                .iter()
                .zip(polynomial.exponents_iter())
                .map(|(c, powers)| Term {
                    coefficient: c.to_string(),
                    exponent: powers[0],
                })
                .collect(),
        });
    }
    let record = Record {
        binding: request.binding.clone(),
        blocks_sha256: payload_digest(&saved)?,
        blocks: saved,
    };
    let mut bytes = Limited(Vec::new());
    serde_json::to_writer(&mut bytes, &record)?;
    budget()?;
    let directory = request
        .path
        .parent()
        .ok_or_else(|| anyhow!("base proposal parent missing"))?;
    std::fs::create_dir_all(directory)?;
    let temporary = request.path.with_extension(format!(
        "{}-{}.tmp",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    // If create_new fails, this path belongs to an earlier writer: do not
    // delete it during cleanup of a file we never created.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| -> Result<()> {
        for chunk in bytes.0.chunks(65536) {
            budget()?;
            file.write_all(chunk)?;
        }
        file.sync_all()?;
        budget()?;
        std::fs::rename(&temporary, &request.path)?;
        File::open(directory)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "symgcad-base-proposals-{}-{}",
                std::process::id(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn context(directory: &Directory, source: &str) -> (Algebra, Poly, Request) {
        let algebra = Algebra::new(&["x".into(), "unused".into()]).unwrap();
        let source = canonical(&algebra.parse(source).unwrap());
        let text = source_text(&algebra, &source).unwrap().unwrap();
        let request = request(&directory.0, &algebra, &text).unwrap().unwrap();
        (algebra, source, request)
    }
    fn rehash(value: &mut serde_json::Value) {
        value["blocks_sha256"] = digest(&serde_json::to_vec(&value["blocks"]).unwrap()).into();
    }

    #[test]
    fn restart_proposal_needs_exact_replay_and_preserves_signed_units() {
        let directory = Directory::new();
        let (algebra, source, request) = context(&directory, "(x-1)^2*(x+2)");
        let blocks = vec![
            (algebra.parse("x-1").unwrap(), 2),
            (algebra.parse("x+2").unwrap(), 1),
        ];
        save(&request, &source, &blocks, &mut || Ok(())).unwrap();
        let Load::Proposed(loaded) = load(&request, &algebra, &source, &mut || Ok(())).unwrap()
        else {
            panic!()
        };
        check_product(&source, &loaded, || Ok(())).unwrap();
        for scale in [Rational::from((-7, 3)), Rational::from((5, 11))] {
            let raw = source.clone().mul_coeff(scale.clone());
            assert_eq!(canonical(&raw), source);
            assert_eq!(source.clone().mul_coeff(scale), raw);
        }
        assert!(source_cost(&source.zero()).is_none());
        // Structure/checksums alone deliberately do not authorize a product.
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&request.path).unwrap()).unwrap();
        value["blocks"][0]["terms"][0]["coefficient"] = "-3".into();
        rehash(&mut value);
        let proposed = parse(
            &request,
            &algebra,
            &source,
            &serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
        assert!(check_product(&source, &proposed, || Ok(())).is_err());
    }

    #[test]
    fn composite_repeated_and_shared_blocks_are_product_proposals_not_sfq_authority() {
        let directory = Directory::new();
        let (algebra, source, request) = context(&directory, "(x-1)^3*(x+2)^2");
        let blocks = vec![
            (algebra.parse("(x-1)^2*(x+2)").unwrap(), 1),
            (algebra.parse("(x-1)*(x+2)").unwrap(), 1),
        ];
        save(&request, &source, &blocks, &mut || Ok(())).unwrap();
        let Load::Proposed(proposed) = load(&request, &algebra, &source, &mut || Ok(())).unwrap()
        else {
            panic!()
        };
        check_product(&source, &proposed, || Ok(())).unwrap();
    }

    #[test]
    fn rehashed_wrong_binding_and_malformed_blocks_are_rejected_before_powers() {
        let directory = Directory::new();
        let (algebra, source, request) = context(&directory, "x^2-1");
        save(&request, &source, &[(source.clone(), 1)], &mut || Ok(())).unwrap();
        let valid: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&request.path).unwrap()).unwrap();
        for choice in 0..11 {
            let mut value = valid.clone();
            match choice {
                0 => value["binding"]["schema_version"] = 2.into(),
                1 => value["binding"]["axis"] = 1.into(),
                2 => value["binding"]["variable_names"][1] = "other".into(),
                3 => value["binding"]["source_sha256"] = "wrong".into(),
                4 => value["blocks"][0]["multiplicity"] = 0.into(),
                5 => value["blocks"][0]["terms"][0]["coefficient"] = "0".into(),
                6 => value["blocks"][0]["terms"][0]["coefficient"] = "1/2".into(),
                7 => {
                    value["blocks"][0]["terms"][0]["coefficient"] =
                        "9".repeat(MAX_DIGITS + 1).into()
                }
                8 => value["blocks"][0]["terms"][1]["exponent"] = (MAX_DEGREE + 1).into(),
                9 => {
                    let duplicate = value["blocks"][0].clone();
                    value["blocks"].as_array_mut().unwrap().push(duplicate);
                }
                10 => value["binding"]["authority"] = "checked".into(),
                _ => unreachable!(),
            }
            rehash(&mut value);
            assert!(
                parse(
                    &request,
                    &algebra,
                    &source,
                    &serde_json::to_vec(&value).unwrap()
                )
                .is_err(),
                "mutation {choice}"
            );
        }
    }

    #[test]
    fn capped_regular_reads_and_interruption_do_not_start_native_fallback() {
        let directory = Directory::new();
        let (algebra, source, request) = context(&directory, "x^2-1");
        save(&request, &source, &[(source.clone(), 1)], &mut || Ok(())).unwrap();
        let error = load(&request, &algebra, &source, &mut || {
            Err(crate::resultant_interpolation::Stopped("read cancelled".into()).into())
        })
        .err()
        .unwrap();
        assert!(
            error
                .downcast_ref::<crate::resultant_interpolation::Stopped>()
                .is_some()
        );
        let target = request.path.with_extension("saved");
        std::fs::rename(&request.path, &target).unwrap();
        std::os::unix::fs::symlink(&target, &request.path).unwrap();
        assert!(matches!(
            load(&request, &algebra, &source, &mut || Ok(())).unwrap(),
            Load::Rejected
        ));
        std::fs::remove_file(&request.path).unwrap();
        let file = File::create(&request.path).unwrap();
        file.set_len(MAX_BYTES as u64 + 1).unwrap();
        assert!(matches!(
            load(&request, &algebra, &source, &mut || Ok(())).unwrap(),
            Load::Rejected
        ));
    }

    #[test]
    fn interrupted_native_replay_saves_only_a_proposal_and_restart_rechecks_it() {
        use std::cell::Cell;
        let directory = Directory::new();
        let (algebra, source, request) = context(&directory, "(x-1)^2*(x+2)");
        let text = source_text(&algebra, &source).unwrap().unwrap();
        let replay = Cell::new(false);
        let mut stopped_report = BaseUnivariateReport::default();
        let stopped = discover_cached_observed(
            &source,
            Some((&algebra, &text, &directory.0)),
            &mut stopped_report,
            || {
                if replay.get() {
                    Err(crate::resultant_interpolation::Stopped("replay cancelled".into()).into())
                } else {
                    Ok(())
                }
            },
            |_, _, phase| {
                if phase == "reconstruction" {
                    replay.set(true);
                }
            },
        );
        assert!(matches!(
            base_stage_stop(stopped.unwrap_err()),
            Stop::Resource(_)
        ));
        assert_eq!(stopped_report.proposal_cache.writes, 1);
        assert!(request.path.is_file());

        let mut restarted = BaseUnivariateReport::default();
        let blocks = discover_cached_observed(
            &source,
            Some((&algebra, &text, &directory.0)),
            &mut restarted,
            || Ok(()),
            |_, _, _| {},
        )
        .unwrap();
        assert_eq!(restarted.proposal_cache.hits, 1);
        assert_eq!(restarted.decomposition_seconds, 0.0);
        check_product(&source, &blocks, || Ok(())).unwrap();

        let replay = Cell::new(false);
        let mut interrupted = BaseUnivariateReport::default();
        let stopped = discover_cached_observed(
            &source,
            Some((&algebra, &text, &directory.0)),
            &mut interrupted,
            || {
                if replay.get() {
                    Err(
                        crate::resultant_interpolation::Stopped("cached replay cancelled".into())
                            .into(),
                    )
                } else {
                    Ok(())
                }
            },
            |_, _, phase| {
                if phase == "cached_reconstruction" {
                    replay.set(true);
                }
            },
        );
        assert!(matches!(
            base_stage_stop(stopped.unwrap_err()),
            Stop::Resource(_)
        ));
        assert_eq!(interrupted.decomposition_seconds, 0.0);
        assert_eq!(interrupted.proposal_cache.misses, 0);
        assert_eq!(interrupted.proposal_cache.writes, 0);
    }

    #[test]
    fn unwritable_cache_is_best_effort_and_wrong_product_is_a_checked_miss() {
        let directory = Directory::new();
        let (algebra, source, request) = context(&directory, "(x-1)^2*(x+2)");
        let text = source_text(&algebra, &source).unwrap().unwrap();
        // A regular file where a cache directory is expected cannot be written.
        std::fs::write(directory.0.join("base-support-v1"), b"not a directory").unwrap();
        let mut report = BaseUnivariateReport::default();
        let blocks = discover_cached_observed(
            &source,
            Some((&algebra, &text, &directory.0)),
            &mut report,
            || Ok(()),
            |_, _, _| {},
        )
        .unwrap();
        check_product(&source, &blocks, || Ok(())).unwrap();
        assert_eq!(report.proposal_cache.write_failures, 1);
        std::fs::remove_file(directory.0.join("base-support-v1")).unwrap();
        save(&request, &source, &blocks, &mut || Ok(())).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&request.path).unwrap()).unwrap();
        value["blocks"][0]["terms"][0]["coefficient"] = "-3".into();
        rehash(&mut value);
        std::fs::write(&request.path, serde_json::to_vec(&value).unwrap()).unwrap();
        let mut report = BaseUnivariateReport::default();
        let blocks = discover_cached_observed(
            &source,
            Some((&algebra, &text, &directory.0)),
            &mut report,
            || Ok(()),
            |_, _, _| {},
        )
        .unwrap();
        assert_eq!(report.proposal_cache.rejected, 1);
        assert_eq!(report.proposal_cache.hits, 0);
        assert_eq!(report.proposal_cache.writes, 1);
        check_product(&source, &blocks, || Ok(())).unwrap();
    }
}
