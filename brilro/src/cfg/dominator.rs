use std::collections::{HashMap, HashSet};

use super::analysis::Cfg;

#[derive(Debug)]
pub struct DominatorTree {
    pub dom: HashMap<usize, HashSet<usize>>,
    pub im_dom: HashMap<usize, HashSet<usize>>,
    pub frontier: HashMap<usize, HashSet<usize>>,
    cfg: Cfg,
}

impl DominatorTree {
    pub fn from_cfg(cfg: &Cfg) -> Self {
        let mut dom: HashMap<usize, HashSet<usize>> = HashMap::new();
        let all_blocks: HashSet<usize> = cfg.blocks.iter().map(|b| b.start).collect();
        for b in &cfg.blocks {
            let e = dom.entry(b.start).or_default();
            if b.start == 0 {
                e.insert(0);
            } else {
                *e = all_blocks.clone();
            }
        }

        let mut changed = true;
        while changed {
            changed = false;
            for block in &cfg.blocks {
                let preds_doms = block
                    .pred
                    .iter()
                    .map(|i| dom[i].clone())
                    .reduce(|acc, e| acc.intersection(&e).copied().collect::<HashSet<_>>());
                if let Some(doms) = preds_doms {
                    let mut doms: HashSet<usize> =
                        doms.intersection(&dom[&block.start]).copied().collect();
                    doms.insert(block.start);
                    changed |= doms != dom[&block.start];
                    dom.insert(block.start, doms);
                }
            }
        }

        // dom[b] is the blocks which dominate b
        // we want the blocks b dominates most of the time so lets reverse that relation.
        let mut dominates: HashMap<usize, HashSet<usize>> = HashMap::new();
        for (domed, domed_by) in dom {
            for b in domed_by {
                dominates.entry(b).or_default().insert(domed);
            }
        }
        let im_dom = dominates.clone();
        // get the transitive closure so we can have the full tree
        let mut changed = true;
        while changed {
            changed = false;
            for (dominator, dominated) in dominates.clone() {
                let one_ahead = dominated
                    .iter()
                    .map(|d| dominates[d].clone())
                    .reduce(|acc, e| e.union(&acc).copied().collect::<HashSet<usize>>());
                if let Some(more) = one_ahead {
                    let unioned: HashSet<usize> = dominated.union(&more).copied().collect();
                    changed |= unioned != dominated;
                    dominates.insert(dominator, unioned);
                }
            }
        }

        let mut frontier: HashMap<usize, HashSet<usize>> = HashMap::new();
        for dominator in &cfg.blocks {
            frontier.entry(dominator.start).or_default();
            for maybe_frontier in &cfg.blocks {
                if (!dominates[&dominator.start].contains(&maybe_frontier.start)
                    || maybe_frontier.start == dominator.start)
                    && maybe_frontier
                        .pred
                        .iter()
                        .any(|b| dominates[&dominator.start].contains(b))
                {
                    frontier
                        .entry(dominator.start)
                        .or_default()
                        .insert(maybe_frontier.start);
                }
            }
        }

        Self {
            dom: dominates,
            frontier,
            cfg: cfg.clone(),
            im_dom,
        }
    }

    pub fn dominators_correct(&self) -> bool {
        fn actually_dominates(
            dominators: &DominatorTree,
            cur: usize,
            looking_for: usize,
            must_have: usize,
            depth: usize,
            max_depth: usize,
        ) -> bool {
            if depth > max_depth || must_have == cur {
                true
            } else if cur == looking_for {
                false
            } else {
                dominators.cfg.block(cur).flows_to.iter().all(|start| {
                    actually_dominates(
                        dominators,
                        *start,
                        looking_for,
                        must_have,
                        depth + 1,
                        max_depth,
                    )
                })
            }
        }

        for dominator in &self.cfg.blocks {
            for dominee in &self.cfg.blocks {
                let dominates = actually_dominates(
                    self,
                    0,
                    dominee.start,
                    dominator.start,
                    0,
                    self.cfg.blocks.len(),
                );
                let thinks_it_dominates = self.dom[&dominator.start].contains(&dominee.start);
                if thinks_it_dominates != dominates {
                    return false;
                }
            }
        }
        true
    }
}
