//! Bounded discovery before projection, followed by conditional-proof compaction.
//!
//! A completed child carries all its ancestor obligations in `Built::required`.
//! Keeping those guards and its exact sample permits enlarging its parent sector:
//! final projection makes the guards sign-invariant on the whole connected
//! sector. Nothing here is a proof until ordinary final projection succeeds.
//! The independent checker and final tree format are deliberately unchanged.
use super::*;
use std::collections::VecDeque;

const MAX_CUTS: usize = 2048;
const MAX_SECTORS: usize = 4096;
const MAX_REQUIREMENTS: usize = 131_072;
const MAX_COMPACTIONS: usize = 32;

struct Child {
    /// Never replace this sample: the entire Built was constructed at it.
    sample: Rational,
    built: Built,
}

/// Select whole old children, in order. Each is used at most once and is moved,
/// never deeply cloned. An uncertain strict membership check declines reuse.
fn select_children(frontier: Vec<Child>, roots: &[FiberRoot]) -> Option<Vec<Child>> {
    if roots.len().checked_add(1)? > frontier.len() {
        return None;
    }
    let mut old: VecDeque<_> = frontier.into();
    let mut selected = Vec::with_capacity(roots.len() + 1);
    for i in 0..=roots.len() {
        let lower = i.checked_sub(1).and_then(|j| roots.get(j));
        let upper = roots.get(i);
        let representative = loop {
            let child = old.pop_front()?;
            if stack_samples::strictly_inside(&child.sample, lower, upper) {
                break child;
            }
        };
        selected.push(representative);
    }
    Some(selected)
}

impl Builder<'_> {
    /// Discovery uses every cut's full exact fiber. Only an original positive
    /// coordinate constraint may discard negative roots, with one explicitly
    /// rejected half-line; no as-yet-unproved root classification is borrowed.
    fn discovery_roots(
        &mut self,
        cuts: &BTreeSet<usize>,
        level: usize,
        prefix: &[Rational],
        ray: Option<usize>,
    ) -> Work<Vec<FiberRoot>> {
        self.check_budget()?;
        self.report.current_level = Some(level);
        self.report.current_prefix = prefix.iter().map(ToString::to_string).collect();
        self.report.active_phase = "deferred_root_discovery".into();
        self.publish(false)?;
        // Nullified or degree-drop fibers need their owning ancestor refined.
        // Do this cheap check before root isolation, without computing any D.
        for &id in cuts {
            self.check_budget()?;
            let p = &self.polynomials[id];
            if highest(p) != Some(level) {
                return Err(Stop::Unsupported("discovery cut has the wrong axis".into()));
            }
            let leading = p.univariate_lcoeff(level);
            if prefix_value(&leading, prefix)?.is_zero() {
                let guards = self.guard_basis(&[leading])?;
                self.guard_nonzero(&guards, prefix)?;
                return Err(Stop::Unsupported(
                    "degree-drop guard did not refine its ancestor".into(),
                ));
            }
        }
        let started = Instant::now();
        let fibers = cuts
            .iter()
            .map(|&id| {
                Ok((
                    id,
                    self.normalized.algebra.specialize_univariate(
                        &self.polynomials[id],
                        level,
                        prefix,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let typed_base = cuts.iter().any(|id| self.base_typed.contains(id));
        let mut roots = isolate_union(fibers).map_err(|e| {
            if typed_base {
                base_stage_stop(e)
            } else {
                Stop::from(e)
            }
        })?;
        if typed_base {
            self.report
                .base_univariate_basis
                .as_mut()
                .unwrap()
                .root_union_seconds += started.elapsed().as_secs_f64();
        }
        if ray.is_some() {
            // A tentative nongeneric cut may share zero with the coordinate.
            // Union ownership is irrelevant to discovery. Final projection
            // must separately discharge that cut's endpoint obligation.
            let zero = roots
                .iter()
                .position(|root| root.interval.0.is_zero() && root.interval.1.is_zero())
                .ok_or_else(|| anyhow!("discovery omitted the protected coordinate zero"))?;
            roots.drain(..zero);
            if !roots
                .iter()
                .skip(1)
                .all(|r| r.interval.0 > Rational::zero())
            {
                return Err(Stop::Unsupported(
                    "discovery roots were not separated from zero".into(),
                ));
            }
        }
        self.operations.phases.root_isolation_seconds += started.elapsed().as_secs_f64();
        self.check_budget()?;
        Ok(roots)
    }

    /// `None` is only a bounded scheduling decline. The caller then runs its
    /// ordinary project-first loop with the current cut inventory. Guards and
    /// resource stops are propagated, never hidden by a second solver budget.
    pub(super) fn visit_deferred(
        &mut self,
        prefix: &mut Vec<Rational>,
        cuts: &mut BTreeSet<usize>,
    ) -> Work<Option<Built>> {
        self.report.deferred_stacks += 1;
        let result = self.discover_and_compact(prefix, cuts)?;
        self.report.deferred_fallbacks += usize::from(result.is_none());
        Ok(result)
    }

    fn discover_and_compact(
        &mut self,
        prefix: &mut Vec<Rational>,
        cuts: &mut BTreeSet<usize>,
    ) -> Work<Option<Built>> {
        let level = prefix.len();
        // Base-point projection intentionally uses full-real roots in replay.
        let ray = (self.options.positive_ray
            && !(level == 0 && self.options.base_point_projection))
            .then(|| positive_coordinate(self.normalized, level))
            .flatten();
        let mut pool = stack_samples::StackSamples::new();
        let mut frontier = loop {
            self.check_budget()?;
            if cuts.len() > MAX_CUTS {
                return Ok(None);
            }
            let roots = self.discovery_roots(cuts, level, prefix, ray)?;
            if roots.len() >= MAX_SECTORS {
                return Ok(None);
            }
            let mut frontier = Vec::new();
            let mut additions = BTreeSet::new();
            let mut siblings = 0usize;
            let mut requirements = 0usize;
            let mut cells = 0usize;
            for i in 0..=roots.len() {
                self.check_budget()?;
                if !additions.is_empty() {
                    siblings += 1;
                    self.report.discovery_siblings += 1;
                }
                let lower = i.checked_sub(1).and_then(|j| roots.get(j));
                let upper = roots.get(i);
                let retained = pool.choose(lower, upper);
                self.report.stable_sample_hits += usize::from(retained.is_some());
                self.report.stable_sample_misses += usize::from(retained.is_none());
                let sample = retained.unwrap_or_else(|| sector_sample(lower, upper));
                prefix.push(sample.clone());
                let child = if let Some(constraint) = ray.filter(|_| i == 0) {
                    Ok(Built {
                        node: LocalNode::Rejected { constraint },
                        required: self.input_factors[constraint].iter().copied().collect(),
                        cells: 0,
                    })
                } else {
                    self.visit(prefix)
                };
                prefix.pop();
                if !(i == 0 && ray.is_some()) {
                    let (recorded, evictions) = pool.remember(&sample);
                    self.report.stable_sample_recorded += usize::from(recorded);
                    self.report.stable_sample_evictions += evictions;
                    self.report.stable_sample_peak_bytes =
                        self.report.stable_sample_peak_bytes.max(pool.bytes());
                }
                match child {
                    Err(Stop::Guard(id)) if highest(&self.polynomials[id]) == Some(level) => {
                        if cuts.contains(&id) {
                            return Err(Stop::Unsupported(
                                "nongeneric sample remained on an existing discovery cut".into(),
                            ));
                        }
                        additions.insert(id);
                    }
                    Err(stop) => {
                        if !additions.is_empty() {
                            cuts.extend(additions);
                            if self.options.reuse_learned_cuts {
                                self.learned_cuts.remember(prefix, cuts);
                            }
                            self.observe_archive(prefix, cuts);
                        }
                        return Err(stop);
                    }
                    Ok(built) => {
                        for &id in &built.required {
                            match highest(&self.polynomials[id]) {
                                Some(axis) if axis == level => {
                                    if !cuts.contains(&id) {
                                        additions.insert(id);
                                    }
                                }
                                Some(axis) if axis < level => {}
                                _ => {
                                    return Err(Stop::Unsupported(
                                        "discovery child returned a later-axis guard".into(),
                                    ));
                                }
                            }
                        }
                        if additions.is_empty() {
                            requirements = requirements.saturating_add(built.required.len());
                            if requirements > MAX_REQUIREMENTS {
                                return Ok(None);
                            }
                            cells = cells
                                .checked_add(built.cells)
                                .ok_or_else(|| Stop::Resource("cell count overflow".into()))?;
                            if self.problem.limits.max_cells.is_some_and(|max| cells > max) {
                                return Err(Stop::Resource("local projection cell limit".into()));
                            }
                            frontier.push(Child { sample, built });
                        }
                    }
                }
                if !additions.is_empty() {
                    frontier.clear();
                    if !self.options.batch_sibling_discovery
                        || additions.len() >= 8
                        || siblings >= 32
                    {
                        break;
                    }
                }
            }
            if additions.is_empty() {
                break frontier;
            }
            self.report.refinement_cuts += additions.len();
            self.report.multi_cut_refinements += usize::from(additions.len() > 1);
            self.report.max_refinement_cuts = self.report.max_refinement_cuts.max(additions.len());
            cuts.extend(additions);
            if self.options.reuse_learned_cuts {
                self.learned_cuts.remember(prefix, cuts);
            }
            self.observe_archive(prefix, cuts);
            self.report.refinements += 1;
        };
        self.report.deferred_frontiers += 1;
        for round in 0..=MAX_COMPACTIONS {
            self.check_budget()?;
            let mut live = self.input_levels[level].clone();
            for child in &frontier {
                live.extend(
                    child
                        .built
                        .required
                        .iter()
                        .copied()
                        .filter(|&id| highest(&self.polynomials[id]) == Some(level)),
                );
            }
            if !live.is_subset(cuts) {
                return Err(Stop::Unsupported(
                    "discovery frontier was not closed".into(),
                ));
            }
            if live == *cuts {
                break;
            }
            // Bounded compaction never calls new children or reseeds hints.
            // A cap declines this scheduling path, not mathematical coverage.
            if round == MAX_COMPACTIONS {
                return Ok(None);
            }
            let roots = self.discovery_roots(&live, level, prefix, ray)?;
            let previous_children = frontier.len();
            let Some(selected) = select_children(frontier, &roots) else {
                return Ok(None);
            };
            self.report.compacted_cuts += cuts.len() - live.len();
            self.report.compacted_children += previous_children - selected.len();
            self.report.compaction_rounds += 1;
            *cuts = live;
            frontier = selected;
        }
        self.check_budget()?;
        let cut_list: Vec<_> = cuts.iter().copied().collect();
        self.report.deferred_projection_calls += 1;
        let projection = self.project(&cut_list, level, prefix)?;
        // Reconstruct the FINAL certified inventory. Provisional positive-root
        // ownership/classifications never appear in exported evidence.
        let started = Instant::now();
        let roots = stack_roots(
            &self.normalized.algebra,
            &self.polynomials,
            &cut_list,
            level,
            prefix,
            projection.positive_ray.as_ref(),
            &projection.root_bearing,
        )?;
        self.operations.phases.root_isolation_seconds += started.elapsed().as_secs_f64();
        if roots.len() + 1 != frontier.len() {
            return Ok(None);
        }
        for (i, child) in frontier.iter().enumerate() {
            self.check_budget()?;
            if !stack_samples::strictly_inside(
                &child.sample,
                i.checked_sub(1).and_then(|j| roots.get(j)),
                roots.get(i),
            ) {
                return Ok(None);
            }
        }
        let mut required = self.projection_requirements(&projection);
        let mut cells = 0usize;
        let mut sectors = Vec::with_capacity(frontier.len());
        for child in frontier {
            for id in child.built.required {
                match highest(&self.polynomials[id]) {
                    Some(axis) if axis == level => {
                        if !cuts.contains(&id) {
                            return Err(Stop::Unsupported(
                                "compaction omitted a child requirement".into(),
                            ));
                        }
                    }
                    Some(axis) if axis < level => {
                        required.insert(id);
                    }
                    _ => {
                        return Err(Stop::Unsupported(
                            "compacted child has later-axis guard".into(),
                        ));
                    }
                }
            }
            cells = cells
                .checked_add(child.built.cells)
                .ok_or_else(|| Stop::Resource("cell count overflow".into()))?;
            sectors.push(LocalSector {
                sample: child.sample.to_string(),
                child: child.built.node,
            });
        }
        self.guard_nonzero(&required, prefix)?;
        self.check_budget()?;
        let root_bounds = local_bounds(&roots, projection.positive_ray.as_ref())?;
        Ok(Some(Built {
            node: LocalNode::Stack {
                cuts: cut_list,
                projection,
                roots: root_bounds,
                sectors,
                required_below: required.iter().copied().collect(),
            },
            required,
            cells,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "symgcad-deferred-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Import a real durable cut hint at the exact positive a sample. Hints have
    /// no proof authority, so this exercises the public solver and its unchanged
    /// independent checker, rather than fabricating a conditional Built.
    fn hinted(live: bool, deferred: bool) -> LocalSolveResult {
        let directory = Directory::new();
        let mut p = Problem {
            variables: ["a", "z", "w"].map(String::from).to_vec(),
            constraints: ["a>0", "z>0", "w>0"].map(String::from).to_vec(),
            split: vec![if live { "w^2-z^8+a" } else { "w-z" }.into()],
            ..Default::default()
        };
        p.limits.wall_time_secs = 20.0;
        p.solver.local_hint_archive = true;
        p.solver.checkpoint_dir = Some(directory.0.clone());
        let options = LocalOptions {
            defer_projection: deferred,
            stable_sector_samples: true,
            fallback_to_global: false,
            positive_pair_separation: false,
            // Isolate scheduling from the existing one-variation theorem,
            // which already omits this octic's discriminant on the positive ray.
            descartes_one: false,
            ..Default::default()
        };
        let n = normalize(&p, &p.variables).unwrap();
        let guard = canonical(&n.algebra.parse("z^8-a").unwrap());
        let mut archive = hint_archive::HintArchive::new(&p, &n, &options, Instant::now()).unwrap();
        archive.observe(&[Rational::one()], &BTreeSet::from([0]));
        archive.snapshot(std::iter::empty(), &[guard], &n.algebra, true);
        assert_eq!(archive.report.snapshots, 1, "{:?}", archive.report);
        let result = solve_local(&p, &options).unwrap();
        assert_eq!(
            result.solution.status,
            Status::CompleteGeneric,
            "{:?}",
            result.evidence
        );
        assert!(verify_local(&result).unwrap().verified);
        result
    }

    fn report(result: &LocalSolveResult) -> &LocalReport {
        let LocalEvidence::Local { report, .. } = &result.evidence else {
            panic!()
        };
        report
    }
    fn with_evidence(mut result: LocalSolveResult) -> SolveResult {
        result.solution.local = Some(Box::new(result.evidence));
        result.solution
    }

    #[test]
    fn stale_octic_hint_is_removed_before_its_discriminant_and_child_sample_is_preserved() {
        let deferred = hinted(false, true);
        let ordinary = hinted(false, false);
        assert!(report(&deferred).hint_archive.as_ref().unwrap().seeded_cuts > 0);
        assert!(
            report(&deferred).compacted_cuts > 0,
            "{:?}",
            report(&deferred)
        );
        assert!(report(&deferred).compacted_children > 0);
        assert_eq!(report(&deferred).cache.discriminant_misses, 0);
        assert!(report(&ordinary).cache.discriminant_misses > 0);
        assert_eq!(deferred.solution.cells.len(), 2);
        assert_eq!(ordinary.solution.cells.len(), 4);
        // The ordinary sample in the enlarged z>0 sector is 1. The first old
        // sector 0<z<1 instead contributes its unchanged sample 1/2.
        assert!(
            deferred
                .solution
                .cells
                .iter()
                .all(|cell| cell.axes[1].sample == "1/2")
        );
        assert_eq!(
            crate::comparison::compare_generic(
                &with_evidence(deferred),
                &with_evidence(ordinary),
                &crate::Limits::default()
            )
            .unwrap()
            .equivalent,
            Some(true)
        );
    }

    #[test]
    fn live_octic_requirement_survives_and_forged_removal_fails_replay() {
        let result = hinted(true, true);
        let n = normalize(&result.solution.problem, &result.solution.order).unwrap();
        let guard = canonical(&n.algebra.parse("z^8-a").unwrap());
        let id = result
            .solution
            .polynomials
            .iter()
            .position(|p| n.algebra.parse(p).unwrap() == guard)
            .unwrap();
        assert!(result.solution.projection_levels[1].contains(&id));
        let mut forged = result.clone();
        let LocalEvidence::Local { root, .. } = &mut forged.evidence else {
            panic!()
        };
        fn omit(node: &mut LocalNode, id: usize) -> bool {
            if let LocalNode::Stack { cuts, sectors, .. } = node {
                if cuts.contains(&id) {
                    cuts.retain(|&cut| cut != id);
                    return true;
                }
                return sectors.iter_mut().any(|sector| omit(&mut sector.child, id));
            }
            false
        }
        assert!(omit(root, id));
        assert!(verify_local(&forged).is_err());
    }

    fn leaf(sample: i64) -> Child {
        Child {
            sample: Rational::from(sample),
            built: Built {
                node: LocalNode::Rejected {
                    constraint: sample.unsigned_abs() as usize,
                },
                required: BTreeSet::from([sample.unsigned_abs() as usize]),
                cells: 0,
            },
        }
    }
    #[test]
    fn coarser_sectors_require_a_whole_strictly_interior_old_child() {
        let a = Algebra::new(&["z".into()]).unwrap();
        let roots = isolate_union(vec![(
            0,
            a.specialize_univariate(&a.parse("z").unwrap(), 0, &[])
                .unwrap(),
        )])
        .unwrap();
        assert!(select_children(vec![leaf(-1)], &roots).is_none());
        assert!(select_children(vec![leaf(-1), leaf(0)], &roots).is_none());
        let selected = select_children(vec![leaf(-3), leaf(-1), leaf(2), leaf(4)], &roots).unwrap();
        assert_eq!(
            selected
                .iter()
                .map(|c| c.sample.clone())
                .collect::<Vec<_>>(),
            [Rational::from(-3), Rational::from(2)]
        );
        assert_eq!(selected[0].built.required, BTreeSet::from([3]));
        assert_eq!(selected[1].built.required, BTreeSet::from([2]));
        assert!(matches!(
            selected[0].built.node,
            LocalNode::Rejected { constraint: 3 }
        ));
    }
}
