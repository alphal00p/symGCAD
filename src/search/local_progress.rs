//! Optional local-CAD complexity observations for timeout promotion only.
//!
//! The counters predict effort; they never establish coverage, reject a
//! mathematical branch, or change UCT rewards and completed-winner selection.
use crate::algebra::Poly;
use serde::{Deserialize, Serialize};

pub const POLICY: &str = "local-inventory-log-v1";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalInventory {
    pub schema_version: u32,
    pub polynomial_count: u64,
    pub term_count: u64,
    pub max_total_degree: u64,
    pub max_coefficient_bits: u64,
}

impl LocalInventory {
    pub fn new() -> Self {
        Self {
            schema_version: 1,
            ..Self::default()
        }
    }

    /// Called once when a distinct polynomial enters the append-only registry.
    pub(crate) fn observe(&mut self, polynomial: &Poly) {
        self.polynomial_count = self.polynomial_count.saturating_add(1);
        self.term_count = self.term_count.saturating_add(polynomial.nterms() as u64);
        let degree = polynomial
            .exponents_iter()
            .map(|powers| {
                powers
                    .iter()
                    .fold(0u64, |sum, &power| sum.saturating_add(u64::from(power)))
            })
            .max()
            .unwrap_or(0);
        self.max_total_degree = self.max_total_degree.max(degree);
        let bits = polynomial
            .coefficients
            .iter()
            .map(|c| {
                c.numerator_ref()
                    .significant_bits()
                    .max(c.denominator_ref().significant_bits())
            })
            .max()
            .unwrap_or(0);
        self.max_coefficient_bits = self.max_coefficient_bits.max(bits);
    }
}

/// A componentwise monotone structural estimate. Equal log weights avoid
/// corpus-specific tuning. No depth, elapsed-time or coverage estimate enters
/// it: local refinement can return to an earlier level at any time.
pub fn promotion_score(inventory: &LocalInventory, refinements: usize) -> Option<f64> {
    if inventory.schema_version != 1
        || inventory.polynomial_count == 0
        || inventory.term_count < inventory.polynomial_count
        || inventory.max_total_degree == 0
        || inventory.max_coefficient_bits == 0
    {
        return None;
    }
    let score = [
        inventory.polynomial_count,
        inventory.term_count,
        inventory.max_total_degree,
        inventory.max_coefficient_bits,
        refinements as u64,
    ]
    .iter()
    .map(|&value| (value as f64).ln_1p())
    .sum::<f64>();
    score.is_finite().then_some(score)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn records_total_degree_and_rational_coefficient_height() {
        let algebra = crate::algebra::Algebra::new(&["x".into(), "y".into()]).unwrap();
        let mut inventory = LocalInventory::new();
        inventory.observe(&algebra.parse("(256/3)*x^2*y^3+x").unwrap());
        assert_eq!(inventory.polynomial_count, 1);
        assert_eq!(inventory.term_count, 2);
        assert_eq!(inventory.max_total_degree, 5);
        assert_eq!(inventory.max_coefficient_bits, 9);
        inventory.observe(&algebra.parse("x/1024+y").unwrap());
        assert_eq!(inventory.polynomial_count, 2);
        assert_eq!(inventory.term_count, 4);
        assert_eq!(inventory.max_total_degree, 5);
        assert_eq!(inventory.max_coefficient_bits, 11);
    }
}
