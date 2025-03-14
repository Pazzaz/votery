use super::{rank::Rank, tied_rank_ref::TiedRankRef, unique};

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

    pub fn to_owned(&self) -> Rank {
        Rank::new(self.elements, self.order.to_vec())
    }

    pub fn winner(&self) -> usize {
        debug_assert!(!self.order.is_empty());
        self.order[0]
    }

    pub fn to_tied(self, tied: &'a [bool]) -> TiedRankRef<'a> {
        TiedRankRef::new(self.elements, self.order, tied)
    }
}
