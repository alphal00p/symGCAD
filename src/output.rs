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
        }
    }
}
