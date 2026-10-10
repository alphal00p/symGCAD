//! Exact support blocks for generated guards in Q[x_0]. No irreducibility
//! assertion is made here, and no ordinary factor cache receives these blocks.
use super::*;
use symbolica::{domains::InternalOrdering, poly::factor::Factorize};

pub(super) const MAX_SOURCES: usize = 128;
pub(super) const MAX_RETAINED_BITS: u64 = 128_000_000;
const MAX_DEGREE: u32 = 4096;
const MAX_TERMS: usize = 4096;
const MAX_AXES: usize = 64;
const MAX_HEIGHT: u64 = 20_000;
const MAX_BLOCKS: usize = 64;
const MAX_TEXT: usize = 32 * 1024 * 1024;

pub(super) fn map_admissible(algebra: &Algebra) -> bool {
    // Together with degree/term/coefficient caps this bounds formatted source
    // text before allocation; inactive names do not occur in a source string.
    !algebra.names.is_empty() && algebra.names.len() <= MAX_AXES && algebra.names[0].len() <= 1024
}

fn bounded(condition: bool, reason: &str) -> Result<()> {
    if !condition {
        return Err(crate::resultant_interpolation::Stopped(reason.into()).into());
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaseUnivariateBasis {
    pub schema_version: u32,
    pub sources: Vec<BaseUnivariateSource>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaseUnivariateSource {
    pub polynomial: String,
    pub blocks: Vec<BaseUnivariateBlock>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaseUnivariateBlock {
    pub polynomial: usize,
    pub multiplicity: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BaseUnivariateReport {
    pub sources: usize,
    pub blocks: usize,
    pub hits: usize,
    pub static_declines: usize,
    pub decomposition_seconds: f64,
    pub reconstruction_seconds: f64,
    pub root_union_seconds: f64,
    /// Logical coefficient costs, charged once per source record (including
    /// shared blocks again). These are not allocator or process-memory counts.
    pub retained_source_bits: u64,
    pub retained_block_bits: u64,
    pub retained_total_bits: u64,
    pub peak_source_bits: u64,
    pub peak_source_height_bits: u64,
    pub peak_source_terms: usize,
    /// Measured only from the source string already needed for evidence.
    pub peak_source_text_bytes: usize,
    pub peak_proposed_block_bits: u64,
    pub peak_proposed_total_bits: u64,
    pub declines: BaseUnivariateDeclines,
    pub post_entry_resource_stops: usize,
    pub post_entry_failures: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_attempt: Option<BaseUnivariateAttempt>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BaseUnivariateDeclines {
    pub map_or_support: usize,
    pub degree: usize,
    pub terms: usize,
    pub coefficient_height: usize,
    pub coefficient_storage: usize,
    pub denominator_storage: usize,
    pub source_count: usize,
    pub registry_storage: usize,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BaseUnivariateDecline {
    MapOrSupport,
    Degree,
    Terms,
    CoefficientHeight,
    CoefficientStorage,
    DenominatorStorage,
    SourceCount,
    RegistryStorage,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BaseUnivariateAttempt {
    /// Raw caller shape; source costs below describe its canonical form only.
    pub raw_degree: Option<u32>,
    pub raw_terms: usize,
    pub source_bits: Option<u64>,
    pub proposed_block_bits: Option<u64>,
    pub proposed_total_bits: Option<u64>,
    pub retained_before_bits: u64,
    /// A fixed internal phase label, never an expression or native error dump.
    pub phase: String,
    pub decline: Option<BaseUnivariateDecline>,
}

impl BaseUnivariateReport {
    pub(super) fn begin(&mut self, p: &Poly, retained: u64) {
        self.last_attempt = Some(BaseUnivariateAttempt {
            raw_degree: (p.nvars() > 0 && p.nterms() <= MAX_TERMS).then(|| p.degree(0)),
            raw_terms: p.nterms(),
            retained_before_bits: retained,
            phase: "preflight".into(),
            ..Default::default()
        });
    }

    pub(super) fn source(&mut self, p: &Poly, bits: u64) {
        self.peak_source_bits = self.peak_source_bits.max(bits);
        self.peak_source_terms = self.peak_source_terms.max(p.nterms());
        self.peak_source_height_bits = self.peak_source_height_bits.max(
            p.coefficients
                .iter()
                .map(|c| {
                    c.numerator_ref()
                        .significant_bits()
                        .max(c.denominator_ref().significant_bits())
                })
                .max()
                .unwrap_or(0),
        );
        if let Some(attempt) = &mut self.last_attempt {
            attempt.source_bits = Some(bits);
            let total = attempt.retained_before_bits.saturating_add(bits);
            attempt.proposed_total_bits = Some(total);
            self.peak_proposed_total_bits = self.peak_proposed_total_bits.max(total);
        }
    }

    pub(super) fn proposal(&mut self, bits: u64) {
        self.peak_proposed_block_bits = self.peak_proposed_block_bits.max(bits);
        if let Some(attempt) = &mut self.last_attempt {
            attempt.proposed_block_bits = Some(bits);
            if let Some(source) = attempt.source_bits {
                let total = attempt
                    .retained_before_bits
                    .saturating_add(source)
                    .saturating_add(bits);
                attempt.proposed_total_bits = Some(total);
                self.peak_proposed_total_bits = self.peak_proposed_total_bits.max(total);
            }
        }
    }

    pub(super) fn phase(&mut self, phase: &str) {
        if let Some(attempt) = &mut self.last_attempt {
            attempt.phase = phase.into();
        }
    }

    pub(super) fn decline(&mut self, reason: BaseUnivariateDecline) {
        self.static_declines += 1;
        let count = match reason {
            BaseUnivariateDecline::MapOrSupport => &mut self.declines.map_or_support,
            BaseUnivariateDecline::Degree => &mut self.declines.degree,
            BaseUnivariateDecline::Terms => &mut self.declines.terms,
            BaseUnivariateDecline::CoefficientHeight => &mut self.declines.coefficient_height,
            BaseUnivariateDecline::CoefficientStorage => &mut self.declines.coefficient_storage,
            BaseUnivariateDecline::DenominatorStorage => &mut self.declines.denominator_storage,
            BaseUnivariateDecline::SourceCount => &mut self.declines.source_count,
            BaseUnivariateDecline::RegistryStorage => &mut self.declines.registry_storage,
        };
        *count += 1;
        if let Some(attempt) = &mut self.last_attempt {
            attempt.decline = Some(reason);
            attempt.phase = "ordinary_fallback".into();
        }
    }
}

/// Diagnostic classification of an existing static decline. This does not
/// decide admission; source_cost remains the sole polynomial-size gate.
pub(super) fn source_decline(p: &Poly) -> BaseUnivariateDecline {
    use BaseUnivariateDecline::*;
    if p.nvars() == 0 || p.degree(0) < 2 {
        return Degree;
    }
    if p.nvars() > MAX_AXES {
        return MapOrSupport;
    }
    if p.nterms() > MAX_TERMS {
        return Terms;
    }
    if p.is_constant() || highest(p) != Some(0) {
        return MapOrSupport;
    }
    if p.degree(0) > MAX_DEGREE {
        return Degree;
    }
    let mut bits = 0u64;
    let mut denominators = 0u64;
    for c in &p.coefficients {
        let n = c.numerator_ref().significant_bits();
        let d = c.denominator_ref().significant_bits();
        if n > MAX_HEIGHT || d > MAX_HEIGHT {
            return CoefficientHeight;
        }
        let Some(next) = bits.checked_add(n).and_then(|x| x.checked_add(d)) else {
            return CoefficientStorage;
        };
        bits = next;
        let Some(next) = denominators.checked_add(d) else {
            return DenominatorStorage;
        };
        denominators = next;
    }
    if bits > 64_000_000 {
        CoefficientStorage
    } else if denominators > 65_536 {
        DenominatorStorage
    } else {
        MapOrSupport
    }
}

/// Static admission is checked before primitive normalization or native work.
pub(super) fn cost(p: &Poly) -> Option<u64> {
    if p.nvars() == 0
        || p.nvars() > MAX_AXES
        || p.nterms() > MAX_TERMS
        || p.is_constant()
        || highest(p) != Some(0)
        || p.degree(0) > MAX_DEGREE
    {
        return None;
    }
    let mut bits = 0u64;
    let mut denominator_bits = 0u64;
    for c in &p.coefficients {
        let numerator = c.numerator_ref().significant_bits();
        let denominator = c.denominator_ref().significant_bits();
        if numerator > MAX_HEIGHT || denominator > MAX_HEIGHT {
            return None;
        }
        bits = bits.checked_add(numerator.checked_add(denominator)?)?;
        denominator_bits = denominator_bits.checked_add(denominator)?;
    }
    (bits <= 64_000_000 && denominator_bits <= 65_536).then_some(bits)
}

/// Degree-one sources already have a cheap ordinary factor identity. Preserve
/// the finite source quota for guards needing a nontrivial support basis.
pub(super) fn source_cost(p: &Poly) -> Option<u64> {
    if p.nvars() == 0 || p.degree(0) < 2 {
        None
    } else {
        cost(p)
    }
}

/// Replays a canonical product with degree/height bounds checked before powers.
/// The factors need not be square-free, coprime, distinct or irreducible.
pub(super) fn check_product(
    source: &Poly,
    blocks: &[(Poly, u32)],
    mut budget: impl FnMut() -> Result<()>,
) -> Result<()> {
    ensure!(
        source_cost(source).is_some() && canonical(source) == *source,
        "invalid base support source"
    );
    ensure!(!blocks.is_empty(), "empty base support product");
    bounded(blocks.len() <= MAX_BLOCKS, "base support block count cap")?;
    let mut degree = 0u64;
    let mut bound = 0u64;
    for (p, multiplicity) in blocks {
        budget()?;
        ensure!(*multiplicity > 0, "invalid base support multiplicity");
        bounded(*multiplicity <= MAX_DEGREE, "base support multiplicity cap")?;
        ensure!(
            p.variables() == source.variables() && highest(p) == Some(0),
            "base support block map or support changed"
        );
        bounded(cost(p).is_some(), "base support block size cap")?;
        ensure!(canonical(p) == *p, "noncanonical base support block");
        degree = degree
            .checked_add(u64::from(p.degree(0)) * u64::from(*multiplicity))
            .ok_or_else(|| anyhow!("base support degree overflow"))?;
        let height = p
            .coefficients
            .iter()
            .map(|c| c.numerator_ref().significant_bits())
            .max()
            .unwrap_or(0);
        let log_terms = p.nterms().max(1).next_power_of_two().trailing_zeros() as u64;
        bound = bound
            .checked_add((height + log_terms) * u64::from(*multiplicity))
            .ok_or_else(|| anyhow!("base support height overflow"))?;
        ensure!(
            degree <= u64::from(source.degree(0)),
            "base support product degree changed"
        );
        bounded(bound <= 65_536, "base support product height cap")?;
    }
    ensure!(
        degree == u64::from(source.degree(0)),
        "base support product degree changed"
    );
    bounded(
        (degree + 1) * bound <= 256_000_000,
        "base support product work cap",
    )?;
    let mut product = source.one();
    for (p, multiplicity) in blocks {
        budget()?;
        product = &product * &p.pow(*multiplicity as usize);
        budget()?;
    }
    ensure!(
        product == *source,
        "base support product does not reconstruct its canonical source"
    );
    Ok(())
}

#[cfg(test)]
pub(super) fn discover(
    source: &Poly,
    report: &mut BaseUnivariateReport,
    budget: impl FnMut() -> Result<()>,
) -> Result<Vec<(Poly, u32)>> {
    discover_observed(source, report, budget, |_, _, _| {})
}

/// The observer receives only bounded metadata and cannot affect algebra or
/// failure routing. In particular it does not supply a budget or a factor.
pub(super) fn discover_observed(
    source: &Poly,
    report: &mut BaseUnivariateReport,
    mut budget: impl FnMut() -> Result<()>,
    mut observer: impl FnMut(&BaseUnivariateReport, Option<u64>, &'static str),
) -> Result<Vec<(Poly, u32)>> {
    budget()?;
    let started = Instant::now();
    let proposed = source.square_free_factorization();
    report.decomposition_seconds += started.elapsed().as_secs_f64();
    observer(report, None, "proposal_check");
    budget()?;
    let started = Instant::now();
    bounded(
        proposed.len() <= MAX_BLOCKS + 2,
        "native base support block count cap",
    )?;
    let mut blocks: HashMap<Poly, u32> = HashMap::new();
    for (mut p, m) in proposed {
        budget()?;
        ensure!(!p.is_zero() && m > 0, "invalid native base support factor");
        bounded(
            m <= MAX_DEGREE as usize,
            "native base support multiplicity cap",
        )?;
        if p.is_constant() {
            p = source.constant(p.get_constant());
        }
        ensure!(
            p.variables() == source.variables(),
            "native base support map changed"
        );
        if p.is_constant() {
            continue;
        }
        ensure!(
            highest(&p) == Some(0),
            "native base support factor support changed"
        );
        bounded(cost(&p).is_some(), "native base support factor cap")?;
        let p = canonical(&p);
        let old = blocks.entry(p).or_default();
        *old = old
            .checked_add(m as u32)
            .ok_or_else(|| anyhow!("base support multiplicity overflow"))?;
    }
    let mut blocks: Vec<_> = blocks.into_iter().collect();
    blocks.sort_by(|a, b| a.0.internal_cmp(&b.0));
    let block_bits = blocks
        .iter()
        .try_fold(0u64, |sum, (p, _)| sum.checked_add(cost(p)?));
    observer(report, block_bits, "reconstruction");
    check_product(source, &blocks, &mut budget)?;
    report.reconstruction_seconds += started.elapsed().as_secs_f64();
    Ok(blocks)
}

/// Strict sparse integer grammar prevents an evidence string from expanding
/// huge expressions before support/degree/height caps can be checked.
fn parse_source(algebra: &Algebra, text: &str) -> Result<Poly> {
    ensure!(
        text.len() <= MAX_TEXT && !text.is_empty() && map_admissible(algebra),
        "base source text/map cap"
    );
    let axis = format!("*{}", algebra.names[0]);
    let mut p = algebra.parse("0")?;
    let mut previous = None;
    for (i, term) in text.split('+').enumerate() {
        ensure!(i < MAX_TERMS, "base source term cap");
        let (coefficient, suffix) = term
            .strip_prefix('(')
            .and_then(|s| s.split_once(')'))
            .ok_or_else(|| anyhow!("invalid base source term"))?;
        let digits = coefficient.strip_prefix('-').unwrap_or(coefficient);
        ensure!(
            !digits.is_empty()
                && digits.len() <= 6021
                && digits.bytes().all(|b| b.is_ascii_digit()),
            "invalid base source integer"
        );
        let exponent = if suffix.is_empty() {
            0
        } else if suffix == axis {
            1
        } else {
            suffix
                .strip_prefix(&format!("{axis}^"))
                .ok_or_else(|| anyhow!("base source is not in its first axis"))?
                .parse::<u32>()?
        };
        ensure!(
            exponent <= MAX_DEGREE && previous.is_none_or(|old| old < exponent),
            "base source exponent cap/order"
        );
        previous = Some(exponent);
        let c = crate::algebra::rational(coefficient)?;
        ensure!(
            !c.is_zero() && c.numerator_ref().significant_bits() <= MAX_HEIGHT,
            "base source coefficient cap"
        );
        let mut exponents = vec![0; algebra.names.len()];
        exponents[0] = exponent;
        p.append_monomial(c, &exponents);
    }
    ensure!(
        source_cost(&p).is_some() && canonical(&p) == p && algebra.format(&p) == text,
        "noncanonical base source"
    );
    Ok(p)
}

#[derive(Default)]
pub(super) struct Validated {
    pub sources: HashMap<Poly, Vec<usize>>,
    pub typed: BTreeSet<usize>,
}

impl Validated {
    /// Match only an already checked canonical product. A freshly recomputed
    /// guard may carry a rational unit larger than the discovery admission cap.
    /// Do not normalize it or apply that cap again merely to identify its zeros.
    /// Validation bounds this scan to 128 sources of at most 4096 terms each;
    /// map/support comparisons precede arithmetic on the existing raw values.
    pub(super) fn proportional_ids(&self, raw: &Poly) -> Option<&[usize]> {
        if raw.is_constant() || raw.nterms() > MAX_TERMS {
            return None;
        }
        for (source, ids) in &self.sources {
            if raw.variables() != source.variables()
                || raw.nterms() != source.nterms()
                || !raw.exponents_iter().eq(source.exponents_iter())
            {
                continue;
            }
            let scale = &raw.coefficients[0] / &source.coefficients[0];
            if !scale.is_zero()
                && raw
                    .coefficients
                    .iter()
                    .zip(&source.coefficients)
                    .all(|(actual, expected)| actual == &(expected * &scale))
            {
                return Some(ids);
            }
        }
        None
    }
}

pub(super) fn validate(
    algebra: &Algebra,
    polynomials: &[Poly],
    evidence: Option<&BaseUnivariateBasis>,
    enabled: bool,
) -> Result<Validated> {
    let mut result = Validated::default();
    let Some(evidence) = evidence else {
        return Ok(result);
    };
    ensure!(enabled, "base support evidence requires enabled option");
    ensure!(
        evidence.schema_version == 1
            && !evidence.sources.is_empty()
            && evidence.sources.len() <= MAX_SOURCES,
        "invalid base support schema/source count"
    );
    let mut retained_bits = 0u64;
    for record in &evidence.sources {
        ensure!(
            !record.blocks.is_empty() && record.blocks.len() <= MAX_BLOCKS,
            "base support block count cap"
        );
        let source = parse_source(algebra, &record.polynomial)?;
        retained_bits += cost(&source).unwrap();
        ensure!(
            retained_bits <= MAX_RETAINED_BITS,
            "base support retained source cap"
        );
        ensure!(
            !result.sources.contains_key(&source),
            "duplicate base support source"
        );
        let mut previous = None;
        let mut blocks = Vec::new();
        let mut block_bits = 0u64;
        for block in &record.blocks {
            ensure!(
                previous.is_none_or(|id| id < block.polynomial),
                "base support IDs are not unique and sorted"
            );
            previous = Some(block.polynomial);
            let p = polynomials
                .get(block.polynomial)
                .ok_or_else(|| anyhow!("invalid base support block ID"))?;
            block_bits += cost(p).ok_or_else(|| anyhow!("invalid base support block"))?;
            ensure!(block_bits <= 64_000_000, "base support block storage cap");
            blocks.push((p.clone(), block.multiplicity));
        }
        retained_bits += block_bits;
        ensure!(
            retained_bits <= MAX_RETAINED_BITS,
            "base support retained source/block cap"
        );
        check_product(&source, &blocks, || Ok(()))?;
        let ids: Vec<_> = record.blocks.iter().map(|b| b.polynomial).collect();
        result.typed.extend(&ids);
        result.sources.insert(source, ids);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbolica::domains::integer::Integer;

    #[test]
    fn diagnostic_decline_reasons_follow_existing_size_gates() {
        let a = Algebra::new(&["x".into(), "z".into()]).unwrap();
        for (text, reason) in [
            ("x", BaseUnivariateDecline::Degree),
            ("x^4097+1", BaseUnivariateDecline::Degree),
            ("x^2+z", BaseUnivariateDecline::MapOrSupport),
        ] {
            let p = a.parse(text).unwrap();
            assert!(source_cost(&p).is_none());
            assert_eq!(source_decline(&p), reason);
        }
        let huge = Integer::from(2).pow(20_000);
        let p = a.parse("x^2").unwrap().mul_coeff(Rational::from(huge)) + a.parse("1").unwrap();
        assert!(source_cost(&p).is_none());
        assert_eq!(source_decline(&p), BaseUnivariateDecline::CoefficientHeight);
        let mut p = a.parse("0").unwrap();
        for exponent in 0..=4096 {
            p.append_monomial(Rational::from(1), &[exponent, 0]);
        }
        assert!(source_cost(&p).is_none());
        assert_eq!(source_decline(&p), BaseUnivariateDecline::Terms);
        let c = Rational::from((Integer::from(1), Integer::from(2).pow(14_000)));
        let mut p = a.parse("0").unwrap();
        for exponent in 2..7 {
            p.append_monomial(c.clone(), &[exponent, 0]);
        }
        assert!(source_cost(&p).is_none());
        assert_eq!(
            source_decline(&p),
            BaseUnivariateDecline::DenominatorStorage
        );
    }

    #[test]
    fn observer_sees_native_return_before_a_late_resource_stop() {
        let a = Algebra::new(&["x".into()]).unwrap();
        let p = a.parse("(x-1)^2*(x+2)").unwrap();
        let mut calls = 0;
        let mut events = Vec::new();
        let mut report = BaseUnivariateReport::default();
        let stopped = discover_observed(
            &p,
            &mut report,
            || {
                calls += 1;
                if calls == 2 {
                    Err(crate::resultant_interpolation::Stopped("after native".into()).into())
                } else {
                    Ok(())
                }
            },
            |report, bits, phase| {
                events.push((phase, bits, report.decomposition_seconds));
            },
        );
        assert!(
            stopped
                .unwrap_err()
                .downcast_ref::<crate::resultant_interpolation::Stopped>()
                .is_some()
        );
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "proposal_check");
        assert_eq!(events[0].1, None);
        let mut events = Vec::new();
        let blocks = discover_observed(
            &p,
            &mut BaseUnivariateReport::default(),
            || Ok(()),
            |_, bits, phase| events.push((phase, bits)),
        )
        .unwrap();
        assert_eq!(
            events.iter().map(|x| x.0).collect::<Vec<_>>(),
            ["proposal_check", "reconstruction"]
        );
        assert_eq!(
            events[1].1,
            Some(blocks.iter().map(|(p, _)| cost(p).unwrap()).sum())
        );
        check_product(&p, &blocks, || Ok(())).unwrap();
    }

    fn source_evidence(algebra: &Algebra, source: &Poly) -> BaseUnivariateBasis {
        BaseUnivariateBasis {
            schema_version: 1,
            sources: vec![BaseUnivariateSource {
                polynomial: algebra.format(source),
                blocks: vec![BaseUnivariateBlock {
                    polynomial: 0,
                    multiplicity: 1,
                }],
            }],
        }
    }

    #[test]
    fn validated_lookup_requires_exact_full_map_support_and_proportionality() {
        let a = Algebra::new(&["s".into(), "z".into()]).unwrap();
        let source = a.parse("s^2-1").unwrap();
        let evidence = source_evidence(&a, &source);
        let validated = validate(&a, std::slice::from_ref(&source), Some(&evidence), true).unwrap();
        let huge = Integer::from(2).pow(20_001);
        let raw = source
            .clone()
            .mul_coeff(Rational::from((huge, Integer::from(3))));
        assert!(source_cost(&raw).is_none());
        assert_eq!(validated.proportional_ids(&raw), Some([0].as_slice()));
        // Same monomials and almost the same large coefficients are not enough.
        let nearby = &raw + &a.parse("s^2").unwrap();
        assert_eq!(nearby.nterms(), raw.nterms());
        assert!(validated.proportional_ids(&nearby).is_none());
        for text in ["0", "1", "s^2-2", "z^2-1", "s^2+s-1"] {
            assert!(
                validated
                    .proportional_ids(&a.parse(text).unwrap())
                    .is_none()
            );
        }
        let other = Algebra::new(&["s".into()]).unwrap();
        assert!(
            validated
                .proportional_ids(&other.parse("s^2-1").unwrap())
                .is_none()
        );
    }

    #[test]
    fn checker_raw_guard_lookup_accepts_large_signed_rational_units_only() {
        let problem = Problem {
            variables: vec!["s".into(), "z".into()],
            ..Default::default()
        };
        let normalized = normalize(&problem, &problem.variables).unwrap();
        let a = &normalized.algebra;
        let source = a.parse("s^2-1").unwrap();
        let ordinary = a.parse("s-5").unwrap();
        // Deliberately retain a composite source, with no s-1 or s+1 IDs:
        // a fallback to irreducible factoring cannot accidentally pass this test.
        let polynomials = vec![source.clone(), ordinary.clone()];
        let evidence = source_evidence(a, &source);
        let mut checker = Checker {
            normalized: &normalized,
            polynomials: &polynomials,
            ids: polynomials
                .iter()
                .cloned()
                .enumerate()
                .map(|(i, p)| (p, i))
                .collect(),
            input_factors: &[],
            guard_certificates: &[],
            guard_domains: Vec::new(),
            protected: BTreeSet::new(),
            domain_pruning: false,
            positive_ray: false,
            positive_pair_separation: false,
            positive_pair_cancellations: 0,
            positive_pair_witnesses: false,
            strict_bezout: false,
            bezout_registry: std::cell::RefCell::new(strict_bezout::Registry::default()),
            stable_sector_samples: false,
            descartes_one: false,
            descartes_witnesses: false,
            descartes_intervals: false,
            base_point_projection: true,
            adjacent_pairs: false,
            binary_discriminants: false,
            base_basis: validate(a, &polynomials, Some(&evidence), true).unwrap(),
        };
        checker.check_registry().unwrap();
        let huge = Integer::from(2).pow(20_001);
        for unit in [
            Rational::from((huge.clone(), Integer::from(3))),
            Rational::from((-huge.clone(), Integer::from(3))),
            Rational::from((Integer::from(3), huge.clone())),
            Rational::from((Integer::from(-3), huge)),
        ] {
            let raw = source.clone().mul_coeff(unit);
            assert!(source_cost(&raw).is_none());
            assert_eq!(checker.guard_basis(&[raw]).unwrap(), BTreeSet::from([0]));
        }
        assert!(checker.guard_basis(&[a.parse("s^2-2").unwrap()]).is_err());
        assert_eq!(
            checker
                .guard_basis(std::slice::from_ref(&ordinary))
                .unwrap(),
            BTreeSet::from([1])
        );
        checker.base_basis = Validated::default();
        assert_eq!(
            checker.guard_basis(&[ordinary]).unwrap(),
            BTreeSet::from([1])
        );
    }

    #[test]
    fn composite_repeated_shared_blocks_replay_without_irreducibility() {
        let a = Algebra::new(&["x".into(), "z".into()]).unwrap();
        let p = canonical(&a.parse("-3/5*(x-1)^3*(x+2)^2*(x^2-2)").unwrap());
        let blocks = vec![
            (a.parse("(x-1)*(x+2)").unwrap(), 2),
            (a.parse("(x-1)*(x^2-2)").unwrap(), 1),
        ];
        check_product(&p, &blocks, || Ok(())).unwrap();
        let mut bad = blocks.clone();
        bad[0].1 = 1;
        assert!(check_product(&p, &bad, || Ok(())).is_err());
        let blocks = discover(&p, &mut BaseUnivariateReport::default(), || Ok(())).unwrap();
        check_product(&p, &blocks, || Ok(())).unwrap();
    }
    #[test]
    fn caps_maps_sparse_grammar_and_stop_are_explicit() {
        let a = Algebra::new(&["x".into(), "z".into()]).unwrap();
        for text in ["0", "1", "z+1", "x^4097+1"] {
            assert!(cost(&a.parse(text).unwrap()).is_none());
        }
        let p = a.parse("x^2-3*x+2").unwrap();
        assert_eq!(parse_source(&a, &a.format(&p)).unwrap(), p);
        for text in [
            "(1)*(x+1)^4096",
            "(1)*z",
            "(1)*x^4097",
            "(1)*x+(1)*x",
            "(-1)*x",
        ] {
            assert!(parse_source(&a, text).is_err());
        }
        assert!(
            discover(&p, &mut BaseUnivariateReport::default(), || Err(anyhow!(
                "stop"
            )))
            .is_err()
        );
        let other = Algebra::new(&["x".into()])
            .unwrap()
            .parse("x^2-3*x+2")
            .unwrap();
        assert!(check_product(&p, &[(other, 1)], || Ok(())).is_err());
    }
    #[test]
    fn entered_stage_failures_and_late_budget_stops_never_select_fallback() {
        assert!(matches!(
            base_stage_stop(anyhow!("native reconstruction failure")),
            Stop::Failed(_)
        ));
        let stopped = crate::resultant_interpolation::Stopped("budget".into());
        assert!(matches!(base_stage_stop(stopped.into()), Stop::Resource(_)));
        let a = Algebra::new(&["x".into()]).unwrap();
        let p = a.parse("x^2-3*x+2").unwrap();
        let mut calls = 0;
        let mut report = BaseUnivariateReport::default();
        let result = discover(&p, &mut report, || {
            calls += 1;
            if calls == 2 {
                Err(crate::resultant_interpolation::Stopped("after native".into()).into())
            } else {
                Ok(())
            }
        });
        assert!(matches!(
            base_stage_stop(result.unwrap_err()),
            Stop::Resource(_)
        ));
        let mut calls = 0;
        assert!(
            check_product(&p, &[(p.clone(), 1)], || {
                calls += 1;
                if calls == 3 {
                    Err(anyhow!("after product"))
                } else {
                    Ok(())
                }
            })
            .is_err()
        );
    }
}
