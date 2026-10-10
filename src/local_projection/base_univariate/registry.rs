//! Registry allocation accounting and exact source lookup. Neither authorizes
//! a support product: callers still perform the independent product replay.
use super::*;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct RegistryStorage {
    pub source_terms: u64,
    pub block_terms: u64,
    pub source_exponent_entries: u64,
    pub block_exponent_entries: u64,
    pub source_text_bytes: u64,
    pub block_text_bytes_bound: u64,
}

impl RegistryStorage {
    pub(in crate::local_projection) fn add_source(&mut self, source: &Poly, text_bytes: usize) {
        self.source_terms = self.source_terms.saturating_add(source.nterms() as u64);
        self.source_exponent_entries = self
            .source_exponent_entries
            .saturating_add(source.exponents.len() as u64);
        self.source_text_bytes = self.source_text_bytes.saturating_add(text_bytes as u64);
    }

    pub(in crate::local_projection) fn add_block(&mut self, block: &Poly, name_bytes: usize) {
        self.block_terms = self.block_terms.saturating_add(block.nterms() as u64);
        self.block_exponent_entries = self
            .block_exponent_entries
            .saturating_add(block.exponents.len() as u64);
        // All admitted blocks are integral. 8 < 10 implies ceil(bits/3)
        // bounds decimal digits. Reserve a sign even for positive integers,
        // parentheses, separator, axis and up to four exponent digits. No
        // coefficient formatting/allocation is needed for this bound.
        let bytes =
            block
                .coefficients
                .iter()
                .zip(block.exponents_iter())
                .fold(0u64, |sum, (c, e)| {
                    let digits = c.numerator_ref().significant_bits().div_ceil(3).max(1);
                    let axis = if e[0] == 0 {
                        0
                    } else {
                        name_bytes as u64 + 1 + if e[0] > 1 { 5 } else { 0 }
                    };
                    sum.saturating_add(digits.saturating_add(4).saturating_add(axis))
                });
        self.block_text_bytes_bound = self.block_text_bytes_bound.saturating_add(bytes);
    }

    pub(in crate::local_projection) fn decline(&self) -> Option<BaseUnivariateDecline> {
        if self.source_terms.saturating_add(self.block_terms) > MAX_REGISTRY_TERMS {
            Some(BaseUnivariateDecline::RegistryTerms)
        } else if self
            .source_exponent_entries
            .saturating_add(self.block_exponent_entries)
            > MAX_REGISTRY_EXPONENTS
        {
            Some(BaseUnivariateDecline::RegistryExponents)
        } else if self
            .source_text_bytes
            .saturating_add(self.block_text_bytes_bound)
            > MAX_REGISTRY_TEXT
        {
            Some(BaseUnivariateDecline::RegistryText)
        } else {
            None
        }
    }

    pub(in crate::local_projection) fn check(&self) -> Result<()> {
        let reason = match self.decline() {
            Some(BaseUnivariateDecline::RegistryTerms) => "base support aggregate term cap",
            Some(BaseUnivariateDecline::RegistryExponents) => {
                "base support aggregate exponent-entry cap"
            }
            Some(BaseUnivariateDecline::RegistryText) => "base support aggregate text cap",
            None => return Ok(()),
            _ => unreachable!("registry allocation decline"),
        };
        bounded(false, reason)
    }
}

/// One polynomial payload shared between the canonical-key map and the shape
/// index. Shape buckets contain only indices, not another polynomial clone.
#[derive(Default)]
pub(in crate::local_projection) struct SourceIndex {
    exact: HashMap<Arc<Poly>, usize>,
    entries: Vec<(Arc<Poly>, Vec<usize>)>,
    shapes: HashMap<u64, Vec<usize>>,
}

fn shape(p: &Poly) -> u64 {
    let mut hash = DefaultHasher::new();
    p.nvars().hash(&mut hash);
    p.exponents.hash(&mut hash);
    hash.finish()
}

impl SourceIndex {
    pub(in crate::local_projection) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(in crate::local_projection) fn get(&self, source: &Poly) -> Option<&Vec<usize>> {
        self.exact.get(source).map(|i| &self.entries[*i].1)
    }

    /// Callers reject duplicate canonical sources before insertion. Retaining
    /// the first immutable mapping also makes accidental duplicate insertion
    /// harmless to the index; it cannot replace already checked IDs.
    pub(in crate::local_projection) fn insert(&mut self, source: Poly, ids: Vec<usize>) {
        if self.exact.contains_key(&source) {
            return;
        }
        let fingerprint = shape(&source);
        let index = self.entries.len();
        let source = Arc::new(source);
        self.exact.insert(source.clone(), index);
        self.entries.push((source, ids));
        self.shapes.entry(fingerprint).or_default().push(index);
    }

    pub(in crate::local_projection) fn proportional_ids(&self, raw: &Poly) -> Option<&[usize]> {
        if raw.is_constant() || raw.nterms() > MAX_TERMS || raw.nvars() > MAX_AXES {
            return None;
        }
        if let Some(ids) = self.get(raw) {
            return Some(ids);
        }
        for &index in self.shapes.get(&shape(raw))? {
            let (source, ids) = &self.entries[index];
            if raw.variables() != source.variables() || raw.exponents != source.exponents {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_bound_covers_exact_sparse_spelling_without_formatting_for_admission() {
        let a = Algebra::new(&["long_axis".into(), "unused".into()]).unwrap();
        for text in [
            "long_axis^4096-123456789*long_axis+1",
            "long_axis^2-1",
            "123*long_axis+456",
        ] {
            let p = canonical(&a.parse(text).unwrap());
            let mut storage = RegistryStorage::default();
            storage.add_block(&p, a.names[0].len());
            assert!(storage.block_text_bytes_bound >= a.format(&p).len() as u64);
            assert_eq!(storage.block_exponent_entries, 2 * p.nterms() as u64);
        }
    }

    #[test]
    fn aggregate_caps_charge_repeated_records_and_distinguish_each_limit() {
        let a = Algebra::new(&["x".into()]).unwrap();
        let p = a.parse("x^2+1").unwrap();
        let mut storage = RegistryStorage::default();
        for _ in 0..2 {
            storage.add_source(&p, a.format(&p).len());
            storage.add_block(&p, 1);
        }
        assert_eq!(storage.source_terms, 4);
        assert_eq!(storage.block_terms, 4);
        for (value, expected) in [
            (
                RegistryStorage {
                    source_terms: MAX_REGISTRY_TERMS + 1,
                    ..Default::default()
                },
                BaseUnivariateDecline::RegistryTerms,
            ),
            (
                RegistryStorage {
                    block_exponent_entries: MAX_REGISTRY_EXPONENTS + 1,
                    ..Default::default()
                },
                BaseUnivariateDecline::RegistryExponents,
            ),
            (
                RegistryStorage {
                    source_text_bytes: MAX_REGISTRY_TEXT + 1,
                    ..Default::default()
                },
                BaseUnivariateDecline::RegistryText,
            ),
        ] {
            assert_eq!(value.decline(), Some(expected));
            assert!(
                value
                    .check()
                    .unwrap_err()
                    .downcast_ref::<crate::resultant_interpolation::Stopped>()
                    .is_some()
            );
        }
    }

    #[test]
    fn cumulative_full_map_exponents_exhaust_before_coefficient_bits() {
        let names = (0..64).map(|i| format!("x{i}")).collect::<Vec<_>>();
        let a = Algebra::new(&names).unwrap();
        let mut p = a.parse("1").unwrap();
        for e in 1..4096 {
            let mut powers = vec![0; 64];
            powers[0] = e;
            p.append_monomial(Rational::one(), &powers);
        }
        assert!(source_cost(&p).is_some() && cost(&p).is_some());
        let mut storage = RegistryStorage::default();
        for i in 1..=64 {
            // Distinct primitive sources: these are not duplicate requests
            // that the real registry would correctly avoid charging.
            p.coefficients[0] = Rational::from(i);
            let text = source_text(&a, &p).unwrap().unwrap();
            storage.add_source(&p, text.len());
            storage.add_block(&p, 2);
        }
        assert_eq!(
            storage.source_exponent_entries + storage.block_exponent_entries,
            MAX_REGISTRY_EXPONENTS
        );
        storage.check().unwrap();
        p.coefficients[0] = Rational::from(65);
        let text = source_text(&a, &p).unwrap().unwrap();
        storage.add_source(&p, text.len());
        storage.add_block(&p, 2);
        assert_eq!(
            storage.decline(),
            Some(BaseUnivariateDecline::RegistryExponents)
        );
        assert!(130 * source_cost(&p).unwrap() < MAX_RETAINED_BITS);
        assert!(storage.source_terms + storage.block_terms < MAX_REGISTRY_TERMS);
    }

    #[test]
    fn canonical_and_proportional_lookup_preserve_immutable_ids_and_exact_support() {
        let a = Algebra::new(&["x".into(), "unused".into()]).unwrap();
        let mut index = SourceIndex::default();
        for degree in 2..202 {
            index.insert(
                a.parse(&format!("x^{degree}+1")).unwrap(),
                vec![degree as usize],
            );
        }
        let q = a.parse("x^100+1").unwrap();
        index.insert(q.clone(), vec![999]);
        assert_eq!(index.get(&q).unwrap(), &[100]);
        assert_eq!(
            index.proportional_ids(&q.mul_coeff(Rational::from((-3, 7)))),
            Some([100].as_slice())
        );
        let p = a.parse("x^100+2").unwrap();
        assert!(index.proportional_ids(&p).is_none());
        // Deliberate shape-fingerprint collision still requires exact support.
        let absent = a.parse("x^203+1").unwrap();
        index.shapes.insert(shape(&absent), vec![0]);
        assert!(index.proportional_ids(&absent).is_none());
        let other = Algebra::new(&["x".into()]).unwrap();
        assert!(
            index
                .proportional_ids(&other.parse("x^100+1").unwrap())
                .is_none()
        );
        assert!(index.proportional_ids(&a.parse("0").unwrap()).is_none());
    }
}
