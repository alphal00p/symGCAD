//! Stable output and the exhaustive lifting certificate.
use crate::{Problem, projection::ProjectionStep};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    CompleteGeneric,
    Incomplete,
    InvalidInput,
    ComputationFailure,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootIndexDomain {
    #[default]
    Real,
    Positive,
    /// Distinct real roots counted from +infinity. Index zero is the largest.
    /// This selector alone does not assert that its root is a continuous
    /// section; the enclosing coverage certificate must prove that property.
    RealDescending,
}
impl RootIndexDomain {
    fn is_real(&self) -> bool {
        *self == Self::Real
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootBound {
    pub polynomial: usize,
    /// Zero-based index among distinct roots in index_domain on the fiber.
    pub index: usize,
    /// Positive indices count strictly positive roots; zero is never included.
    /// RealDescending reverses real-root order and includes zero.
    #[serde(default, skip_serializing_if = "RootIndexDomain::is_real")]
    pub index_domain: RootIndexDomain,
    pub lower: String,
    pub upper: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Axis {
    pub variable: String,
    pub sample: String,
    pub lower: Option<RootBound>,
    pub upper: Option<RootBound>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub axes: Vec<Axis>,
    /// Signs of the requested split polynomials, in input order.
    pub signs: Vec<i8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LiftTrace {
    Rejected {
        constraint: usize,
    },
    Accepted {
        cell: usize,
    },
    Stack {
        roots: Vec<RootBound>,
        sectors: Vec<SectorTrace>,
    },
    Unfinished {
        reason: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SectorTrace {
    pub sample: String,
    pub child: LiftTrace,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Statistics {
    pub ordering_seconds: f64,
    pub search_seconds: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ordering_search: Option<serde_json::Value>,
    pub projection_seconds: f64,
    pub lifting_seconds: f64,
    pub merging_seconds: f64,
    pub total_seconds: f64,
    pub visited_stacks: usize,
    pub rejected_branches: usize,
    pub raw_cells: usize,
    pub projection_steps: Vec<ProjectionStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimized_projection: Option<crate::optimized_projection::OptimizedProjectionReport>,
    /// Last diagnostic snapshot from a worker that did not return a complete
    /// result file. This is neither local coverage evidence nor phase timing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interrupted_local_progress: Option<crate::local_projection::LocalReport>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolveResult {
    pub schema_version: u32,
    pub status: Status,
    pub problem: Problem,
    pub order: Vec<String>,
    pub normalized_constraints: Vec<String>,
    pub split_polynomials: Vec<String>,
    pub polynomials: Vec<String>,
    pub projection_levels: Vec<Vec<usize>>,
    pub exceptional_polynomials: Vec<usize>,
    pub cells: Vec<Cell>,
    pub trace: Option<LiftTrace>,
    pub statistics: Statistics,
    pub diagnostic: Option<String>,
    /// Compatibility-only output. Raw cells and excluded seams remain above.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merged_cells: Option<Vec<Cell>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homogeneous: Option<Box<crate::homogeneity::HomogeneousEvidence>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monotone: Option<Box<crate::monotone::MonotoneEvidence>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<Box<crate::local_projection::LocalEvidence>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sign_filter: Option<Box<crate::sign_filter::SignFilterEvidence>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linear_negative: Option<Box<crate::linear_negative::LinearNegativeEvidence>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bilinear_positive: Option<Box<crate::bilinear_positive::BilinearPositiveEvidence>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quadratic_ladder: Option<Box<crate::quadratic_ladder::QuadraticLadderEvidence>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bilinear_general: Option<Box<crate::bilinear_general::BilinearGeneralEvidence>>,
}

impl SolveResult {
    pub fn failure(problem: Problem, status: Status, reason: impl Into<String>) -> Self {
        Self {
            schema_version: 1,
            order: problem.effective_order(),
            problem,
            status,
            normalized_constraints: vec![],
            split_polynomials: vec![],
            polynomials: vec![],
            projection_levels: vec![],
            exceptional_polynomials: vec![],
            cells: vec![],
            trace: None,
            statistics: Statistics::default(),
            diagnostic: Some(reason.into()),
            merged_cells: None,
            homogeneous: None,
            monotone: None,
            local: None,
            sign_filter: None,
            linear_negative: None,
            bilinear_positive: None,
            quadratic_ladder: None,
            bilinear_general: None,
        }
    }
}

#[cfg(test)]
mod root_selector_serialization_tests {
    use super::*;

    #[test]
    fn descending_root_selector_is_explicit_and_historical_defaults_survive() {
        let old = r#"{"polynomial":0,"index":0,"lower":"0","upper":"0"}"#;
        let mut bound: RootBound = serde_json::from_str(old).unwrap();
        assert_eq!(bound.index_domain, RootIndexDomain::Real);
        bound.index_domain = RootIndexDomain::RealDescending;
        let encoded = serde_json::to_value(&bound).unwrap();
        assert_eq!(encoded["index_domain"], "real_descending");
        assert_eq!(serde_json::from_value::<RootBound>(encoded).unwrap(), bound);

        let failed = SolveResult::failure(Problem::default(), Status::Incomplete, "old result");
        let encoded = serde_json::to_value(&failed).unwrap();
        assert!(encoded.get("linear_negative").is_none());
        assert!(encoded.get("bilinear_positive").is_none());
        assert!(
            serde_json::from_value::<SolveResult>(encoded.clone())
                .unwrap()
                .bilinear_positive
                .is_none()
        );
        assert!(
            serde_json::from_value::<SolveResult>(encoded)
                .unwrap()
                .linear_negative
                .is_none()
        );
    }
}
