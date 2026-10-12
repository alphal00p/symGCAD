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
    /// Projective section of the OPT-IN contract extension
    /// `solver.projective_sections` (disabled by default): the index-th
    /// distinct strictly positive root of the boundary polynomial on the
    /// fiber, or +infinity where fewer than index+1 such roots exist.
    ///
    /// Such a bound may only be an UPPER bound of a cell axis. A cell carrying
    /// one is a connected open cylindrical cell in the compactified
    /// coordinate u=x/(1+x): its upper section u=X/(1+X) is continuous with
    /// values in (0,1], and u=1 (x=+infinity) is reached where the selected
    /// root does not exist. Only `bilinear_general` projective plans certify
    /// these bounds, and only when the problem enables the option; every
    /// other verifier rejects them. When the root does not exist at the cell
    /// sample, the sample-specific isolation `lower`/`upper` strings are both
    /// [`PROJECTIVE_INFINITY`].
    PositiveOrInfinity,
}
impl RootIndexDomain {
    fn is_real(&self) -> bool {
        *self == Self::Real
    }
    /// True for the opt-in projective selector (see
    /// [`RootIndexDomain::PositiveOrInfinity`]).
    pub fn is_projective(&self) -> bool {
        *self == Self::PositiveOrInfinity
    }
}

/// Isolation text of a [`RootIndexDomain::PositiveOrInfinity`] bound whose
/// selected root does not exist at the cell sample (the section is +infinity
/// there). It is deliberately not a rational number, so a consumer that does
/// not know projective sections fails instead of reading a finite value.
pub const PROJECTIVE_INFINITY: &str = "+inf";

impl Cell {
    /// Whether any axis bound uses the opt-in projective selector.
    pub fn has_projective_bound(&self) -> bool {
        self.axes.iter().any(|axis| {
            [&axis.lower, &axis.upper]
                .into_iter()
                .flatten()
                .any(|b| b.index_domain.is_projective())
        })
    }
}

/// Whether any raw or merged cell of a result uses a projective bound.
pub fn has_projective_bounds(result: &SolveResult) -> bool {
    result.cells.iter().any(Cell::has_projective_bound)
        || result
            .merged_cells
            .iter()
            .flatten()
            .any(Cell::has_projective_bound)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootBound {
    pub polynomial: usize,
    /// Zero-based index among distinct roots in index_domain on the fiber.
    pub index: usize,
    /// Positive indices count strictly positive roots; zero is never included.
    /// RealDescending reverses real-root order and includes zero.
    /// PositiveOrInfinity counts like Positive and selects +infinity where
    /// the root does not exist (opt-in projective sections only).
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
    /// Heuristic candidate scores of `ordering = "kinematic_portfolio"`.
    /// Descriptive only: the certificate is the ordinary fixed-order solve.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ordering_portfolio: Option<crate::kinematic_portfolio::PortfolioReport>,
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
    /// Planning, per-piece and assembly wall times of a bilinear_general
    /// result, kept distinct because pieces may run as separate processes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bilinear_general_pieces: Option<crate::bilinear_general::PieceTimings>,
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

        // The opt-in projective selector has its own explicit name, and its
        // non-rational infinity isolation text survives a round trip.
        for (lower, upper) in [("3/2", "3/2"), (PROJECTIVE_INFINITY, PROJECTIVE_INFINITY)] {
            let projective = RootBound {
                polynomial: 4,
                index: 0,
                index_domain: RootIndexDomain::PositiveOrInfinity,
                lower: lower.into(),
                upper: upper.into(),
            };
            let encoded = serde_json::to_value(&projective).unwrap();
            assert_eq!(encoded["index_domain"], "positive_or_infinity");
            assert_eq!(
                serde_json::from_value::<RootBound>(encoded).unwrap(),
                projective
            );
            let cell = Cell {
                axes: vec![Axis {
                    variable: "x".into(),
                    sample: "1".into(),
                    lower: None,
                    upper: Some(projective),
                }],
                signs: vec![],
            };
            assert!(cell.has_projective_bound());
        }
        assert!(crate::algebra::rational(PROJECTIVE_INFINITY).is_err());

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
