use crate::order::incomplete::unique;

use super::rank::TotalRank;

#[derive(Debug, Clone, Copy)]
pub struct TotalRankRef<'a> {
    pub(super) order: &'a [usize],
}

impl<'a> TotalRankRef<'a> {
    pub fn new(v: &'a [usize]) -> Self {
        if !v.is_empty() {
            assert!(unique(v));
            assert!(v.contains(&0));
        }
        TotalRankRef { order: v }
    }

    pub unsafe fn new_unchecked(v: &'a [usize]) -> Self {
        TotalRankRef { order: v }
    }

    pub fn elements(&self) -> usize {
        self.order.len()
    }

    pub fn top(&self, n: usize) -> &[usize] {
        &self.order[..n]
    }

    pub fn owned(&self) -> TotalRank {
        TotalRank { order: self.order.to_vec() }
    }
}
