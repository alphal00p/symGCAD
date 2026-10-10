//! Bounded, reproducible MCTS over legal variable-elimination prefixes.
//!
//! Algebra runs in the caller's killable worker callback. A discarded search
//! branch is not a mathematical rejection, and only a complete exact solve
//! may be reported as `TrialStatus::Completed`.

use crate::ordering::validate_groups;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

pub mod checkpoint;
pub mod local_progress;
pub use checkpoint::{CheckpointReport, SearchCheckpointOptions, search_checkpointed};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchOptions {
    pub max_rollouts: usize,
    pub wall_time_secs: f64,
    pub seed: u64,
    pub exploration_constant: f64,
    pub candidate_timeouts_secs: Vec<f64>,
    /// Promote an unfinished candidate after this many fresh trials.
    pub promotion_interval: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self::for_campaign(3600.0)
    }
}

impl SearchOptions {
    pub fn for_campaign(campaign_secs: f64) -> Self {
        Self {
            max_rollouts: 256,
            wall_time_secs: (campaign_secs * 0.2).min(300.0),
            seed: 0,
            exploration_constant: std::f64::consts::SQRT_2,
            candidate_timeouts_secs: vec![1.0, 10.0, 60.0],
            promotion_interval: 8,
        }
    }

    fn validate(&self) -> Result<()> {
        ensure!(
            self.max_rollouts > 0,
            "search requires a positive rollout limit"
        );
        ensure!(
            self.wall_time_secs.is_finite() && self.wall_time_secs > 0.0,
            "search time must be finite and positive"
        );
        ensure!(
            self.exploration_constant.is_finite() && self.exploration_constant >= 0.0,
            "MCTS exploration constant must be finite and nonnegative"
        );
        ensure!(
            !self.candidate_timeouts_secs.is_empty()
                && self
                    .candidate_timeouts_secs
                    .iter()
                    .all(|t| t.is_finite() && *t > 0.0)
                && self.candidate_timeouts_secs.windows(2).all(|w| w[0] < w[1]),
            "candidate deadlines must be finite, positive and increasing"
        );
        ensure!(
            self.promotion_interval > 0,
            "promotion interval must be positive"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrialRequest {
    /// Full order, from the first lifted to the last lifted variable.
    pub order: Vec<usize>,
    /// Tree prefix in elimination order, useful for worker checkpoint reuse.
    pub elimination_prefix: Vec<usize>,
    pub timeout_secs: f64,
    pub tier: usize,
    pub seed: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrialStatus {
    /// All requested cells were obtained by exact complete lifting.
    Completed,
    Timeout,
    MemoryLimit,
    /// Heuristic early abandonment. This has no logical implication.
    Pruned,
    Failed,
    /// A completion claimed by a restored search log. It is only a heuristic
    /// until this invocation obtains a fresh complete solve of that order.
    Unverified,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrialOutcome {
    pub status: TrialStatus,
    pub elapsed_secs: f64,
    pub cells: Option<usize>,
    /// Optional nonnegative structural cost estimate; smaller is promising.
    /// Used only to prioritize unfinished candidates, never as a certificate.
    pub progress_score: Option<f64>,
    /// Optional ranking for timed-out candidate promotions only. It does not
    /// affect UCT rewards, pruning, or the completed-winner comparison.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promotion_score: Option<f64>,
    pub message: Option<String>,
    /// A measured, expensive elimination prefix. Search may avoid repeating
    /// it during this bounded campaign; it is not an exceptional set or a
    /// mathematical rejection. A later campaign/final solve may retry it.
    #[serde(default)]
    pub abandoned_prefix: Option<Vec<usize>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrialRecord {
    pub request: TrialRequest,
    pub outcome: TrialOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchStop {
    RolloutLimit,
    TimeLimit,
    Exhausted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchReport {
    /// Present only if at least one worker completed an exact solve.
    pub best_order: Option<Vec<usize>>,
    pub best_elapsed_secs: Option<f64>,
    /// An order to try with the remaining campaign budget. If best_order is
    /// None this is merely a heuristic recommendation, not a solved problem.
    pub suggested_order: Vec<usize>,
    pub trials: Vec<TrialRecord>,
    pub iterations: usize,
    pub wall_time_secs: f64,
    pub stopped: SearchStop,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revalidations: Vec<TrialRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<CheckpointReport>,
}

#[derive(Default)]
struct Node {
    prefix: Vec<usize>,
    children: Vec<(usize, usize)>,
    visits: u64,
    reward_sum: f64,
}

struct Candidate {
    order: Vec<usize>,
    prefix: Vec<usize>,
    path: Vec<usize>,
    tier: usize,
    last: TrialOutcome,
}

/// The callback must enforce request.timeout_secs, including cancellation of a
/// single long-running factorization/resultant. Thread-level time checks alone
/// are insufficient; the CLI uses benchmark::run_bounded on a worker process.
/// Seeds and priorities are cheap inputs. No algebra is executed in this parent.
pub fn search<F>(
    groups: &[Vec<usize>],
    elimination_priority: &[usize],
    seeds: &[Vec<usize>],
    options: &SearchOptions,
    evaluate: F,
) -> Result<SearchReport>
where
    F: FnMut(&TrialRequest) -> Result<TrialOutcome>,
{
    search_impl(
        groups,
        elimination_priority,
        seeds,
        options,
        None,
        None,
        false,
        evaluate,
    )
}

fn search_impl<F>(
    groups: &[Vec<usize>],
    elimination_priority: &[usize],
    seeds: &[Vec<usize>],
    options: &SearchOptions,
    restored: Option<&checkpoint::Archive>,
    mut journal: Option<&mut checkpoint::Journal>,
    replay_only: bool,
    mut evaluate: F,
) -> Result<SearchReport>
where
    F: FnMut(&TrialRequest) -> Result<TrialOutcome>,
{
    options.validate()?;
    let variables: usize = groups.iter().map(Vec::len).sum();
    validate_groups(groups, variables)?;
    ensure!(
        elimination_priority.len() == variables
            && elimination_priority
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                == (0..variables).collect(),
        "elimination priority must be a permutation of variable indices"
    );
    for seed in seeds {
        validate_order(seed, groups)?;
    }
    let mut rank = vec![0; variables];
    for (i, &v) in elimination_priority.iter().enumerate() {
        rank[v] = i;
    }
    let total_orders = groups.iter().fold(1usize, |total, g| {
        (2..=g.len()).fold(total, usize::saturating_mul)
    });
    let started = Instant::now();
    let mut rng = SplitMix64(options.seed);
    let mut nodes = vec![Node::default()];
    let mut candidates = Vec::<Candidate>::new();
    let mut seen = BTreeMap::<Vec<usize>, usize>::new();
    let mut records = Vec::new();
    let mut abandoned = BTreeMap::<Vec<usize>, Option<f64>>::new();
    let mut best: Option<usize> = None;
    let mut seed_index = 0;
    let mut fresh_since_promotion = 0;
    let mut promotions = 0;
    let mut iterations = 0;
    let mut stopped = SearchStop::RolloutLimit;
    let restored_iterations = restored.map_or(0, |a| a.iterations);
    let mut replay_cursor = 0;
    let mut revalidations = restored.map_or_else(Vec::new, |a| a.revalidations.clone());
    let old_revalidations = revalidations.len();
    let mut revalidated = restored.is_none();
    let mut revalidated_best: Option<TrialRecord> = None;
    while iterations < options.max_rollouts {
        let replaying = iterations < restored_iterations;
        ensure!(
            !replay_only || started.elapsed().as_secs_f64() < options.wall_time_secs,
            "checkpoint replay exceeded invocation wall budget"
        );
        if !replaying && !revalidated && !replay_only {
            ensure!(
                replay_cursor == restored.unwrap().records.len(),
                "unused restored trials"
            );
            // Archived successful statuses are untrusted. Re-run the best
            // historical candidates before allowing any of them to win.
            let mut completed: Vec<_> = candidates
                .iter()
                .filter(|c| c.last.status == TrialStatus::Completed)
                .collect();
            completed.sort_by(|a, b| {
                a.last
                    .elapsed_secs
                    .total_cmp(&b.last.elapsed_secs)
                    .then_with(|| a.last.cells.cmp(&b.last.cells))
                    .then_with(|| a.order.cmp(&b.order))
            });
            for c in completed.into_iter().take(options.max_rollouts) {
                let remaining = options.wall_time_secs - started.elapsed().as_secs_f64();
                if remaining <= 0.0 {
                    break;
                }
                let request = TrialRequest {
                    order: c.order.clone(),
                    elimination_prefix: c.prefix.clone(),
                    timeout_secs: options.candidate_timeouts_secs[c.tier].min(remaining),
                    tier: c.tier,
                    seed: options.seed,
                };
                let outcome = evaluate(&request)?;
                validate_outcome(&request, &outcome, variables)?;
                let record = TrialRecord { request, outcome };
                if record.outcome.status == TrialStatus::Completed
                    && revalidated_best.as_ref().is_none_or(|b| {
                        completed_cost(&record.outcome) < completed_cost(&b.outcome)
                    })
                {
                    revalidated_best = Some(record.clone());
                }
                revalidations.push(record);
                if let Some(j) = journal.as_deref_mut() {
                    j.save(iterations, &records, &revalidations);
                }
            }
            revalidated = true;
        }
        let remaining = options.wall_time_secs - started.elapsed().as_secs_f64();
        if !replaying && remaining <= 0.0 {
            stopped = SearchStop::TimeLimit;
            break;
        }
        if !replaying
            && iterations - restored_iterations + revalidations.len() - old_revalidations
                >= options.max_rollouts
        {
            break;
        }
        let promote = promotion_candidate(&candidates, options, promotions);
        let all_orders_seen = seen.len() == total_orders;
        let use_promotion = promote.is_some()
            && (fresh_since_promotion >= options.promotion_interval || all_orders_seen);
        if all_orders_seen && promote.is_none() {
            stopped = SearchStop::Exhausted;
            break;
        }

        let (order, prefix, path, tier, candidate_id) = if use_promotion {
            let id = promote.unwrap();
            let c = &candidates[id];
            promotions += 1;
            fresh_since_promotion = 0;
            (
                c.order.clone(),
                c.prefix.clone(),
                c.path.clone(),
                c.tier + 1,
                Some(id),
            )
        } else {
            let (order, prefix) = if seed_index < seeds.len() {
                let order = seeds[seed_index].clone();
                seed_index += 1;
                (order, Vec::new())
            } else {
                propose(
                    &mut nodes,
                    groups,
                    &rank,
                    options.exploration_constant,
                    &mut rng,
                )
            };
            let path = ensure_path(&mut nodes, &order.iter().rev().copied().collect::<Vec<_>>());
            if let Some(&id) = seen.get(&order) {
                backpropagate(&mut nodes, &path, reward(&candidates[id].last));
                iterations += 1;
                if !replaying {
                    if let Some(j) = journal.as_deref_mut() {
                        j.save(iterations, &records, &revalidations);
                    }
                }
                continue;
            }
            fresh_since_promotion += 1;
            (order, prefix, path, 0, None)
        };

        let mut request = TrialRequest {
            order: order.clone(),
            elimination_prefix: prefix.clone(),
            timeout_secs: options.candidate_timeouts_secs[tier].min(if replaying {
                options.wall_time_secs
            } else {
                remaining
            }),
            tier,
            seed: options.seed,
        };
        let cached_outcome = if let Some((prefix, score)) = abandoned.iter().find(|(prefix, _)| {
            request
                .order
                .iter()
                .rev()
                .copied()
                .take(prefix.len())
                .eq(prefix.iter().copied())
        }) {
            Some(TrialOutcome {
                status: TrialStatus::Pruned,
                elapsed_secs: 0.0,
                cells: None,
                progress_score: *score,
                promotion_score: None,
                message: Some("shared prefix abandoned earlier in this search campaign".into()),
                abandoned_prefix: Some(prefix.clone()),
            })
        } else {
            None
        };
        let outcome = if replaying {
            let saved = restored
                .unwrap()
                .records
                .get(replay_cursor)
                .ok_or_else(|| anyhow::anyhow!("restored trial journal ended early"))?;
            ensure!(
                saved.request.timeout_secs.is_finite()
                    && saved.request.timeout_secs > 0.0
                    && saved.request.timeout_secs <= options.candidate_timeouts_secs[tier],
                "invalid restored trial deadline"
            );
            request.timeout_secs = saved.request.timeout_secs;
            ensure!(
                serde_json::to_value(&request)? == serde_json::to_value(&saved.request)?,
                "restored trial does not match deterministic scheduler"
            );
            if let Some(cached) = &cached_outcome {
                ensure!(
                    serde_json::to_value(cached)? == serde_json::to_value(&saved.outcome)?,
                    "restored shared-prefix decision differs"
                );
            }
            replay_cursor += 1;
            saved.outcome.clone()
        } else if let Some(cached) = cached_outcome {
            cached
        } else {
            evaluate(&request)?
        };
        validate_outcome(&request, &outcome, variables)?;
        if let Some(prefix) = &outcome.abandoned_prefix {
            ensure!(
                outcome.status == TrialStatus::Pruned
                    && !prefix.is_empty()
                    && prefix.len() < variables
                    && request
                        .order
                        .iter()
                        .rev()
                        .copied()
                        .take(prefix.len())
                        .eq(prefix.iter().copied()),
                "worker returned an invalid abandoned prefix"
            );
            abandoned
                .entry(prefix.clone())
                .or_insert(outcome.progress_score);
        }
        backpropagate(&mut nodes, &path, reward(&outcome));
        let id = match candidate_id {
            Some(id) => {
                candidates[id].tier = tier;
                candidates[id].last = outcome.clone();
                id
            }
            None => {
                let id = candidates.len();
                seen.insert(order.clone(), id);
                candidates.push(Candidate {
                    order,
                    prefix,
                    path,
                    tier,
                    last: outcome.clone(),
                });
                id
            }
        };
        if !replaying
            && outcome.status == TrialStatus::Completed
            && best.is_none_or(|b| completed_cost(&outcome) < completed_cost(&candidates[b].last))
        {
            best = Some(id);
        }
        records.push(TrialRecord { request, outcome });
        iterations += 1;
        if !replaying {
            if let Some(j) = journal.as_deref_mut() {
                j.save(iterations, &records, &revalidations);
            }
        }
    }
    ensure!(
        iterations >= restored_iterations
            && replay_cursor == restored.map_or(0, |a| a.records.len()),
        "restored journal cannot be replayed within rollout limit"
    );
    // A finished checkpoint may have exhausted its scheduler limit. It still
    // must revalidate a completion before returning a winner; allow a separate
    // bounded validation pass without adding scheduler iterations.
    if !revalidated && !replay_only {
        let mut completed: Vec<_> = candidates
            .iter()
            .filter(|c| c.last.status == TrialStatus::Completed)
            .collect();
        completed.sort_by(|a, b| {
            a.last
                .elapsed_secs
                .total_cmp(&b.last.elapsed_secs)
                .then_with(|| a.last.cells.cmp(&b.last.cells))
                .then_with(|| a.order.cmp(&b.order))
        });
        for c in completed.into_iter().take(options.max_rollouts) {
            let remaining = options.wall_time_secs - started.elapsed().as_secs_f64();
            if remaining <= 0.0 {
                break;
            }
            let request = TrialRequest {
                order: c.order.clone(),
                elimination_prefix: c.prefix.clone(),
                timeout_secs: options.candidate_timeouts_secs[c.tier].min(remaining),
                tier: c.tier,
                seed: options.seed,
            };
            let outcome = evaluate(&request)?;
            validate_outcome(&request, &outcome, variables)?;
            let record = TrialRecord { request, outcome };
            if record.outcome.status == TrialStatus::Completed
                && revalidated_best
                    .as_ref()
                    .is_none_or(|b| completed_cost(&record.outcome) < completed_cost(&b.outcome))
            {
                revalidated_best = Some(record.clone());
            }
            revalidations.push(record);
            if let Some(j) = journal.as_deref_mut() {
                j.save(iterations, &records, &revalidations);
            }
        }
    }
    let historical_restart = candidates
        .iter()
        .filter(|c| c.last.status == TrialStatus::Completed)
        .min_by(|a, b| {
            a.last
                .elapsed_secs
                .total_cmp(&b.last.elapsed_secs)
                .then_with(|| a.last.cells.cmp(&b.last.cells))
                .then_with(|| a.order.cmp(&b.order))
        })
        .map(|c| c.order.clone());
    let fallback = historical_restart
        .or_else(|| {
            candidates
                .iter()
                .filter(|c| matches!(c.last.status, TrialStatus::Timeout | TrialStatus::Pruned))
                .min_by(|a, b| partial_cost(&a.last).total_cmp(&partial_cost(&b.last)))
                .map(|c| c.order.clone())
        })
        .or_else(|| seeds.first().cloned())
        .unwrap_or_else(|| groups.iter().flatten().copied().collect());
    let mut best_order = best.map(|b| candidates[b].order.clone());
    let mut best_elapsed_secs = best.map(|b| candidates[b].last.elapsed_secs);
    if let Some(validated) = revalidated_best {
        if best.is_none_or(|b| {
            completed_cost(&validated.outcome) < completed_cost(&candidates[b].last)
        }) {
            best_order = Some(validated.request.order);
            best_elapsed_secs = Some(validated.outcome.elapsed_secs);
        }
    }
    for record in records
        .iter_mut()
        .take(replay_cursor)
        .chain(revalidations.iter_mut().take(old_revalidations))
    {
        if record.outcome.status == TrialStatus::Completed {
            record.outcome.status = TrialStatus::Unverified;
        }
    }
    Ok(SearchReport {
        suggested_order: best_order.clone().unwrap_or(fallback),
        best_order,
        best_elapsed_secs,
        trials: records,
        iterations,
        wall_time_secs: started.elapsed().as_secs_f64(),
        stopped,
        revalidations,
        checkpoint: None,
    })
}

fn validate_outcome(
    request: &TrialRequest,
    outcome: &TrialOutcome,
    variables: usize,
) -> Result<()> {
    ensure!(
        outcome.status != TrialStatus::Unverified,
        "worker returned an unverified completion claim"
    );
    ensure!(
        outcome.elapsed_secs.is_finite() && outcome.elapsed_secs >= 0.0,
        "worker returned an invalid elapsed time"
    );
    ensure!(
        outcome
            .progress_score
            .is_none_or(|x| x.is_finite() && x >= 0.0),
        "worker returned an invalid progress score"
    );
    ensure!(
        outcome
            .promotion_score
            .is_none_or(|x| x.is_finite() && x >= 0.0),
        "worker returned an invalid promotion score"
    );
    ensure!(
        outcome.status != TrialStatus::Completed || outcome.cells.is_some(),
        "completed worker omitted its cell count"
    );
    if let Some(prefix) = &outcome.abandoned_prefix {
        ensure!(
            outcome.status == TrialStatus::Pruned
                && !prefix.is_empty()
                && prefix.len() < variables
                && request
                    .order
                    .iter()
                    .rev()
                    .copied()
                    .take(prefix.len())
                    .eq(prefix.iter().copied()),
            "worker returned an invalid abandoned prefix"
        );
    }
    Ok(())
}

fn validate_order(order: &[usize], groups: &[Vec<usize>]) -> Result<()> {
    let n: usize = groups.iter().map(Vec::len).sum();
    ensure!(
        order.len() == n && order.iter().copied().collect::<BTreeSet<_>>() == (0..n).collect(),
        "search seed must be a permutation"
    );
    let mut group_of = vec![0; n];
    for (i, group) in groups.iter().enumerate() {
        for &v in group {
            group_of[v] = i;
        }
    }
    ensure!(
        order.windows(2).all(|w| group_of[w[0]] <= group_of[w[1]]),
        "search seed violates variable groups"
    );
    Ok(())
}

fn legal_variables(prefix: &[usize], groups: &[Vec<usize>], rank: &[usize]) -> Vec<usize> {
    for group in groups.iter().rev() {
        let mut left: Vec<_> = group
            .iter()
            .copied()
            .filter(|v| !prefix.contains(v))
            .collect();
        if !left.is_empty() {
            left.sort_by_key(|&v| rank[v]);
            return left;
        }
    }
    Vec::new()
}

fn ensure_child(nodes: &mut Vec<Node>, parent: usize, variable: usize) -> usize {
    if let Some(&(_, child)) = nodes[parent].children.iter().find(|&&(v, _)| v == variable) {
        return child;
    }
    let mut prefix = nodes[parent].prefix.clone();
    prefix.push(variable);
    let id = nodes.len();
    nodes.push(Node {
        prefix,
        ..Node::default()
    });
    nodes[parent].children.push((variable, id));
    id
}

fn ensure_path(nodes: &mut Vec<Node>, elimination: &[usize]) -> Vec<usize> {
    let mut node = 0;
    let mut path = vec![node];
    for &v in elimination {
        node = ensure_child(nodes, node, v);
        path.push(node);
    }
    path
}

fn propose(
    nodes: &mut Vec<Node>,
    groups: &[Vec<usize>],
    rank: &[usize],
    c: f64,
    rng: &mut SplitMix64,
) -> (Vec<usize>, Vec<usize>) {
    let mut node = 0;
    loop {
        let legal = legal_variables(&nodes[node].prefix, groups, rank);
        if legal.is_empty() {
            break;
        }
        if let Some(&v) = legal
            .iter()
            .find(|&&v| !nodes[node].children.iter().any(|&(w, _)| w == v))
        {
            node = ensure_child(nodes, node, v);
            break;
        }
        let log_visits = (nodes[node].visits.max(1) as f64).ln();
        node = nodes[node]
            .children
            .iter()
            .max_by(|&&(va, a), &&(vb, b)| {
                let score = |v: usize, child: usize| {
                    let n = &nodes[child];
                    if n.visits == 0 {
                        return f64::INFINITY;
                    }
                    n.reward_sum / n.visits as f64
                        + c * (log_visits / n.visits as f64).sqrt()
                        + 0.1 / ((rank[v] + 1) as f64 * (n.visits + 1) as f64)
                };
                score(va, a)
                    .total_cmp(&score(vb, b))
                    .then_with(|| rank[vb].cmp(&rank[va]))
            })
            .unwrap()
            .1;
    }
    let prefix = nodes[node].prefix.clone();
    let mut order = prefix.clone();
    loop {
        let legal = legal_variables(&order, groups, rank);
        if legal.is_empty() {
            break;
        }
        // Mostly follow inexpensive features, but retain broad exploration.
        let choice = if rng.next() % 5 == 0 {
            rng.next() as usize % legal.len()
        } else {
            0
        };
        order.push(legal[choice]);
    }
    order.reverse();
    (order, prefix)
}

fn backpropagate(nodes: &mut [Node], path: &[usize], reward: f64) {
    for &id in path {
        nodes[id].visits += 1;
        nodes[id].reward_sum += reward;
    }
}

fn completed_cost(outcome: &TrialOutcome) -> (f64, usize) {
    (outcome.elapsed_secs, outcome.cells.unwrap_or(usize::MAX))
}

fn partial_cost(outcome: &TrialOutcome) -> f64 {
    outcome.progress_score.unwrap_or(f64::MAX)
}

fn promotion_cost(outcome: &TrialOutcome) -> f64 {
    outcome
        .promotion_score
        .unwrap_or_else(|| partial_cost(outcome))
}

fn reward(outcome: &TrialOutcome) -> f64 {
    if outcome.status == TrialStatus::Completed {
        0.5 + 0.5 / (1.0 + outcome.elapsed_secs.ln_1p())
    } else if outcome.status == TrialStatus::Timeout {
        outcome
            .progress_score
            .map_or(0.0, |cost| 0.25 / (1.0 + cost.ln_1p()))
    } else {
        0.0
    }
}

fn promotion_candidate(
    candidates: &[Candidate],
    options: &SearchOptions,
    promotions: usize,
) -> Option<usize> {
    // Usually advance shallow trials first; every fourth promotion gives a
    // promising survivor the longer tier rather than starving 60-second trials.
    let deep = promotions % 4 == 3;
    candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.last.status == TrialStatus::Timeout
                && c.tier + 1 < options.candidate_timeouts_secs.len()
        })
        .min_by(|(ia, a), (ib, b)| {
            let tier = if deep {
                b.tier.cmp(&a.tier)
            } else {
                a.tier.cmp(&b.tier)
            };
            tier.then_with(|| promotion_cost(&a.last).total_cmp(&promotion_cost(&b.last)))
                .then_with(|| ia.cmp(ib))
        })
        .map(|(id, _)| id)
}

struct SplitMix64(u64);

/// Cheap, deliberately conservative triggers for abandoning a search trial.
/// Measurements refer to completed exact projection prefixes. These thresholds
/// are predictions of cost, not bounds on solvability; the final selected solve
/// and independent verifier do not call this function. Returns the elimination
/// depth so the same expensive prefix can be avoided in this search campaign.
pub fn growth_abandonment(
    checkpoints: &[crate::optimized_projection::ProjectionCheckpoint],
    variables: usize,
) -> Option<(usize, String)> {
    let mut depth = 0;
    let mut previous_terms = None;
    for checkpoint in checkpoints {
        if checkpoint.eliminated_variable.is_some() {
            depth += 1;
        }
        let remaining = variables.saturating_sub(depth);
        let terms = checkpoint.term_count;
        let explosive = depth > 0
            && remaining >= 3
            && terms >= 4_000
            && previous_terms
                .is_some_and(|previous: usize| terms >= previous.max(1).saturating_mul(8));
        let absolute = depth > 0 && remaining >= 2 && terms >= 100_000;
        if explosive || absolute {
            return Some((
                depth,
                format!(
                    "projection prefix of length {depth} has {terms} terms across {} factors with {remaining} variables remaining; heuristic restart required",
                    checkpoint.polynomial_count
                ),
            ));
        }
        previous_terms = Some(terms);
    }
    None
}

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}
