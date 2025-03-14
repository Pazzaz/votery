use std::cmp;

use crate::order::incomplete::unique;

use super::TotalRankRef;

#[derive(Debug, Clone)]
pub struct TotalRank {
    pub(super) order: Vec<usize>,
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

    pub fn sort_with<F: Fn(&usize, &usize) -> cmp::Ordering>(&mut self, f: F) {
        self.order.sort_by(f);
    }
}