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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrialOutcome {
    pub status: TrialStatus,
    pub elapsed_secs: f64,
    pub cells: Option<usize>,
    /// Optional nonnegative structural cost estimate; smaller is promising.
    /// Used only to prioritize unfinished candidates, never as a certificate.
    pub progress_score: Option<f64>,
    pub message: Option<String>,
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
    let mut best: Option<usize> = None;
    let mut seed_index = 0;
    let mut fresh_since_promotion = 0;
    let mut promotions = 0;
    let mut iterations = 0;
    let mut stopped = SearchStop::RolloutLimit;
    while iterations < options.max_rollouts {
        let remaining = options.wall_time_secs - started.elapsed().as_secs_f64();
        if remaining <= 0.0 {
            stopped = SearchStop::TimeLimit;
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
                continue;
            }
            fresh_since_promotion += 1;
            (order, prefix, path, 0, None)
        };

        let request = TrialRequest {
            order: order.clone(),
            elimination_prefix: prefix.clone(),
            timeout_secs: options.candidate_timeouts_secs[tier].min(remaining),
            tier,
            seed: options.seed,
        };
        let outcome = evaluate(&request)?;
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
        if outcome.status == TrialStatus::Completed
            && best.is_none_or(|b| completed_cost(&outcome) < completed_cost(&candidates[b].last))
        {
            best = Some(id);
        }
        records.push(TrialRecord { request, outcome });
        iterations += 1;
    }
    let fallback = candidates
        .iter()
        .filter(|c| c.last.status == TrialStatus::Timeout)
        .min_by(|a, b| partial_cost(&a.last).total_cmp(&partial_cost(&b.last)))
        .map(|c| c.order.clone())
        .or_else(|| seeds.first().cloned())
        .unwrap_or_else(|| groups.iter().flatten().copied().collect());
    let best_order = best.map(|b| candidates[b].order.clone());
    Ok(SearchReport {
        suggested_order: best_order.clone().unwrap_or(fallback),
        best_order,
        best_elapsed_secs: best.map(|b| candidates[b].last.elapsed_secs),
        trials: records,
        iterations,
        wall_time_secs: started.elapsed().as_secs_f64(),
        stopped,
    })
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

fn reward(outcome: &TrialOutcome) -> f64 {
    if outcome.status == TrialStatus::Completed {
        1.0 / (1.0 + outcome.elapsed_secs.ln_1p())
    } else if outcome.status == TrialStatus::Timeout {
        outcome
            .progress_score
            .map_or(0.0, |cost| 0.001 / (1.0 + cost.ln_1p()))
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
            tier.then_with(|| partial_cost(&a.last).total_cmp(&partial_cost(&b.last)))
                .then_with(|| ia.cmp(ib))
        })
        .map(|(id, _)| id)
}

struct SplitMix64(u64);
impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}
