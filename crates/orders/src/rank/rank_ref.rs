use super::total_rank_ref::TotalRankRef;
use crate::{tied_rank::TiedRankRef, unique};

/// A possibly incomplete order without any ties
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RankRef<'a> {
    pub(crate) elements: usize,
    pub order: &'a [usize],
}

impl<'a> RankRef<'a> {
    pub fn new(elements: usize, order: &'a [usize]) -> Self {
        debug_assert!(unique(order));
        RankRef { elements, order }
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn top(&self, n: usize) -> Self {
        RankRef::new(self.elements, &self.order[0..n])
    }

    pub fn winner(&self) -> usize {
        debug_assert!(!self.order.is_empty());
        self.order[0]
    }

    pub fn to_tied(self, tied: &'a [bool]) -> TiedRankRef<'a> {
        TiedRankRef::new(self.elements, self.order, tied)
    }

    /// Converts to complete ranking. Panics if not all elements are ranked.
    pub fn to_complete(self) -> TotalRankRef<'a> {
        let RankRef { elements, order } = self;
        assert!(elements == order.len());
        TotalRankRef { order }
    }
}
