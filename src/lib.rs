//! Exact generic cylindrical algebraic decomposition.
pub mod config;
pub mod algebra;
pub mod projection;
pub mod problem;
pub mod roots;
pub mod ordering;
pub mod output;
pub mod solver;
pub mod verify;
pub mod corpus;
pub mod benchmark;
pub mod search;
pub mod domain;
pub mod optimized_projection;
pub use config::{Limits, Problem, SolverOptions};
