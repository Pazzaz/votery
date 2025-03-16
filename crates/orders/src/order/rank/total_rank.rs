use std::cmp;

use super::{rank::Rank, total_rank_ref::TotalRankRef};
use crate::order::unique;

#[derive(Debug)]
pub struct TotalRank {
    pub(crate) order: Vec<usize>,
}

impl Clone for TotalRank {
    fn clone(&self) -> Self {
        Self { order: self.order.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.order.clone_from(&source.order);
    }
}

impl TotalRank {
    pub fn new(v: Vec<usize>) -> Self {
        if !v.is_empty() {
            assert!(unique(&v));
            assert!(v.contains(&0));
        }
        Self { order: v }
    }

    pub unsafe fn new_unchecked(v: Vec<usize>) -> Self {
        Self { order: v }
    }

    pub fn new_empty() -> Self {
        TotalRank { order: Vec::new() }
    }

    pub fn new_default(n: usize) -> Self {
        TotalRank { order: (0..n).collect() }
    }

    pub fn as_ref(&self) -> TotalRankRef {
        TotalRankRef { order: &self.order }
    }

    pub fn get_inner(self) -> Vec<usize> {
        let Self { order } = self;
        order
    }

    /// Remove element `n` from the order
    pub fn remove(&mut self, n: usize) {
        let mut j = 0;
        for i in 0..self.order.len() {
            match n.cmp(&self.order[i]) {
                cmp::Ordering::Less => {
                    self.order[j] = self.order[i] - 1;
                    j += 1;
                }
                cmp::Ordering::Greater => {
                    self.order[j] = self.order[i];
                    j += 1;
                }
                cmp::Ordering::Equal => {}
            }
        }
        self.order.drain(j..);
    }

    pub fn sort_by<F: Fn(&usize, &usize) -> cmp::Ordering>(&mut self, f: F) {
        self.order.sort_by(f);
    }

    pub fn copy_from_ref(&mut self, other: TotalRankRef) {
        self.order.clear();
        self.order.extend(other.order);
    }

    pub fn to_incomplete(self) -> Rank {
        let Self { order } = self;
        let elements = order.len();
        Rank { elements, order }
    }
}
