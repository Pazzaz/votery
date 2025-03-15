use crate::order::unique;

use super::{rank_ref::RankRef, total_rank::TotalRank};


#[derive(Debug, Clone, Copy)]
pub struct TotalRankRef<'a> {
    pub(crate) order: &'a [usize],
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

    pub fn to_incomplete(self) -> RankRef<'a> {
        let Self { order } = self;
        let elements = order.len();
        RankRef { elements, order }
    }
}
