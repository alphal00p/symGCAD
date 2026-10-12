//! Indexed implementation of the historical greedy compatibility merge.
//!
//! Stable original cell IDs retain vector-removal order. The smallest live
//! (i,j,axis) candidate is exactly the pair selected by the former nested scan;
//! only adjacency discovery changes. Raw cells remain the certified output.
use crate::output::{Cell, RootBound, RootIndexDomain};
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Boundary {
    polynomial: usize,
    index: usize,
    domain: u8,
}

fn boundary(value: &Option<RootBound>) -> Option<Boundary> {
    value.as_ref().map(|root| Boundary {
        polynomial: root.polynomial,
        index: root.index,
        domain: match root.index_domain {
            RootIndexDomain::Real => 0,
            RootIndexDomain::Positive => 1,
            RootIndexDomain::RealDescending => 2,
            // Never compared in practice: `merge` refuses cells that carry
            // an opt-in projective bound.
            RootIndexDomain::PositiveOrInfinity => 3,
        },
    })
}

fn same_boundary(first: &Option<RootBound>, second: &Option<RootBound>) -> bool {
    boundary(first) == boundary(second)
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Face {
    axis: usize,
    signs: Vec<i8>,
    other_axes: Vec<(Option<Boundary>, Option<Boundary>)>,
    seam: Boundary,
}

fn face(cell: &Cell, axis: usize, seam: Boundary) -> Face {
    Face {
        axis,
        signs: cell.signs.clone(),
        other_axes: cell
            .axes
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != axis)
            .map(|(_, axis)| (boundary(&axis.lower), boundary(&axis.upper)))
            .collect(),
        seam,
    }
}

type Faces = HashMap<Face, BTreeSet<usize>>;
type Candidates = BTreeSet<(usize, usize, usize)>;

#[derive(Default)]
struct Index {
    lower: Faces,
    upper: Faces,
    candidates: Candidates,
}

impl Index {
    fn insert(&mut self, id: usize, cell: &Cell) {
        for (axis, bounds) in cell.axes.iter().enumerate() {
            if let Some(seam) = boundary(&bounds.lower) {
                let key = face(cell, axis, seam);
                if let Some(others) = self.upper.get(&key) {
                    for &other in others {
                        if other != id {
                            self.candidates.insert((id.min(other), id.max(other), axis));
                        }
                    }
                }
                self.lower.entry(key).or_default().insert(id);
            }
            if let Some(seam) = boundary(&bounds.upper) {
                let key = face(cell, axis, seam);
                if let Some(others) = self.lower.get(&key) {
                    for &other in others {
                        if other != id {
                            self.candidates.insert((id.min(other), id.max(other), axis));
                        }
                    }
                }
                self.upper.entry(key).or_default().insert(id);
            }
        }
    }

    fn remove(&mut self, id: usize, cell: &Cell) {
        for (axis, bounds) in cell.axes.iter().enumerate() {
            for (side, seam) in [
                (&mut self.lower, boundary(&bounds.lower)),
                (&mut self.upper, boundary(&bounds.upper)),
            ] {
                if let Some(seam) = seam {
                    let key = face(cell, axis, seam);
                    if let Some(ids) = side.get_mut(&key) {
                        ids.remove(&id);
                        if ids.is_empty() {
                            side.remove(&key);
                        }
                    }
                }
            }
        }
        // Old candidates are checked lazily. Removing them by cell ID would
        // require another index and cannot affect which live candidate wins.
    }
}

fn direction(first: &Cell, second: &Cell, axis: usize) -> Option<bool> {
    if first.axes.len() != second.axes.len()
        || axis >= first.axes.len()
        || first.signs != second.signs
        || first
            .axes
            .iter()
            .zip(&second.axes)
            .enumerate()
            .any(|(index, (a, b))| {
                index != axis
                    && (!same_boundary(&a.lower, &b.lower) || !same_boundary(&a.upper, &b.upper))
            })
    {
        return None;
    }
    let a = &first.axes[axis];
    let b = &second.axes[axis];
    if a.upper.is_some() && same_boundary(&a.upper, &b.lower) {
        Some(true)
    } else if b.upper.is_some() && same_boundary(&b.upper, &a.lower) {
        Some(false)
    } else {
        None
    }
}

/// Compatibility only: seams remain recoverable from raw cells and the trace.
/// Preserve the historical smallest-pair/first-axis policy, samples, bound
/// evidence and output order while indexing potentially adjacent cells.
///
/// Cells with an opt-in projective bound (`RootIndexDomain::PositiveOrInfinity`)
/// are refused: they are kept unchanged and never merged with any other cell.
/// Problem validation also refuses `merge = "aggressive"` together with
/// `projective_sections`.
pub fn merge(raw: &[Cell]) -> Vec<Cell> {
    let mut cells = raw.iter().cloned().map(Some).collect::<Vec<_>>();
    let mut index = Index::default();
    for (id, cell) in raw.iter().enumerate() {
        if !cell.has_projective_bound() {
            index.insert(id, cell);
        }
    }
    while let Some((i, j, axis)) = index.candidates.pop_first() {
        let (Some(first), Some(second)) = (&cells[i], &cells[j]) else {
            continue;
        };
        let Some(forward) = direction(first, second, axis) else {
            continue;
        };
        index.remove(i, first);
        index.remove(j, second);
        let second = cells[j].take().unwrap();
        let first = cells[i].as_mut().unwrap();
        if forward {
            first.axes[axis].upper = second.axes[axis].upper.clone();
        } else {
            first.axes[axis].lower = second.axes[axis].lower.clone();
        }
        index.insert(i, first);
    }
    cells.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::Axis;
    use rand::{SeedableRng, rngs::StdRng, seq::SliceRandom};

    // Retain the previous algorithm as an independent compatibility oracle.
    fn scanned_merge(raw: &[Cell]) -> Vec<Cell> {
        let mut cells = raw.to_vec();
        loop {
            let mut found = None;
            'pairs: for i in 0..cells.len() {
                for j in i + 1..cells.len() {
                    if cells[i].signs != cells[j].signs {
                        continue;
                    }
                    for k in 0..cells[i].axes.len() {
                        if (0..cells[i].axes.len()).any(|axis| {
                            axis != k
                                && (!same_boundary(
                                    &cells[i].axes[axis].lower,
                                    &cells[j].axes[axis].lower,
                                ) || !same_boundary(
                                    &cells[i].axes[axis].upper,
                                    &cells[j].axes[axis].upper,
                                ))
                        }) {
                            continue;
                        }
                        let a = &cells[i].axes[k];
                        let b = &cells[j].axes[k];
                        if a.upper.is_some() && same_boundary(&a.upper, &b.lower) {
                            found = Some((i, j, k, true));
                            break 'pairs;
                        }
                        if b.upper.is_some() && same_boundary(&b.upper, &a.lower) {
                            found = Some((i, j, k, false));
                            break 'pairs;
                        }
                    }
                }
            }
            let Some((i, j, k, forward)) = found else {
                break;
            };
            if forward {
                cells[i].axes[k].upper = cells[j].axes[k].upper.clone();
            } else {
                cells[i].axes[k].lower = cells[j].axes[k].lower.clone();
            }
            cells.remove(j);
        }
        cells
    }

    fn bound(axis: usize, value: usize) -> Option<RootBound> {
        Some(RootBound {
            polynomial: axis,
            index: value,
            index_domain: RootIndexDomain::Real,
            lower: value.to_string(),
            upper: value.to_string(),
        })
    }

    fn grid(side: usize, dimensions: usize) -> Vec<Cell> {
        (0..side.pow(dimensions as u32))
            .map(|mut code| Cell {
                axes: (0..dimensions)
                    .map(|axis| {
                        let position = code % side;
                        code /= side;
                        Axis {
                            variable: format!("x{axis}"),
                            sample: format!("{}/2", 2 * position + 1),
                            lower: bound(axis, position),
                            upper: bound(axis, position + 1),
                        }
                    })
                    .collect(),
                signs: vec![1],
            })
            .collect()
    }

    #[test]
    fn exhaustive_punctured_cubes_preserve_greedy_choices_and_evidence() {
        let original = grid(2, 3);
        let mut rng = StdRng::seed_from_u64(0);
        for mask in 0..256 {
            let mut cells = original
                .iter()
                .enumerate()
                .filter(|(id, _)| mask & (1 << id) != 0)
                .map(|(id, cell)| {
                    let mut cell = cell.clone();
                    // Include different split signs; neither algorithm may
                    // merge through their boundary.
                    cell.signs.push(if id % 3 == 0 { -1 } else { 1 });
                    cell
                })
                .collect::<Vec<_>>();
            for _ in 0..4 {
                cells.shuffle(&mut rng);
                assert_eq!(merge(&cells), scanned_merge(&cells), "mask={mask}");
            }
        }
    }

    #[test]
    fn changing_multiple_axes_retains_the_same_lexicographic_result() {
        let mut rng = StdRng::seed_from_u64(11);
        for dimensions in 1..=4 {
            let mut cells = grid(3, dimensions);
            for _ in 0..8 {
                cells.shuffle(&mut rng);
                assert_eq!(merge(&cells), scanned_merge(&cells));
            }
        }
    }

    #[test]
    fn infinite_endpoints_root_selectors_and_fiber_intervals_keep_their_meaning() {
        let mut cells = grid(3, 1);
        cells[0].axes[0].lower = None;
        cells[2].axes[0].upper = None;
        // Sample-specific isolation intervals do not define root identity.
        cells[1].axes[0].lower.as_mut().unwrap().lower = "1/2".into();
        assert_eq!(merge(&cells), scanned_merge(&cells));
        assert_eq!(merge(&cells).len(), 1);
        for domain in [RootIndexDomain::Positive, RootIndexDomain::RealDescending] {
            let mut incompatible = cells.clone();
            incompatible[1].axes[0].lower.as_mut().unwrap().index_domain = domain;
            assert_eq!(merge(&incompatible), scanned_merge(&incompatible));
            assert_eq!(merge(&incompatible).len(), 2);
        }
        assert!(merge(&[]).is_empty());
        // Opt-in projective sections are refused: cells carrying one keep
        // their exact raw form, even where a seam would otherwise match.
        for side in [0, 1, 2] {
            let mut projective = cells.clone();
            let selector = projective[side].axes[0]
                .upper
                .get_or_insert_with(|| bound(0, 3).unwrap());
            selector.index_domain = RootIndexDomain::PositiveOrInfinity;
            let merged = merge(&projective);
            assert!(merged.contains(&projective[side]));
            // The middle cell separates the outer two; otherwise the other
            // two cells still merge with each other.
            assert_eq!(merged.len(), if side == 1 { 3 } else { 2 });
        }
        let zero_dimensional = vec![Cell {
            axes: vec![],
            signs: vec![1],
        }];
        assert_eq!(merge(&zero_dimensional), zero_dimensional);
    }

    #[test]
    fn a_large_grid_merges_without_a_quadratic_pair_scan() {
        let cells = grid(16, 3);
        let merged = merge(&cells);
        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0].axes.iter().map(|a| &a.sample).collect::<Vec<_>>(),
            cells[0].axes.iter().map(|a| &a.sample).collect::<Vec<_>>()
        );
        for axis in 0..3 {
            assert_eq!(merged[0].axes[axis].lower, bound(axis, 0));
            assert_eq!(merged[0].axes[axis].upper, bound(axis, 16));
        }
    }
}
