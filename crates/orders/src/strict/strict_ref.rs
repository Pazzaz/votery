use super::{strict::Total, strict_incomplete_ref::ChainRef};
use crate::{OrderRef, unique_and_bounded};

#[derive(Debug, Clone, Copy)]
pub struct TotalRef<'a> {
    pub(crate) order: &'a [usize],
}

impl<'a> TotalRef<'a> {
    /// Create a new `StrictRef` from a permutation of `0..s.len()`.
    pub fn new(v: &'a [usize]) -> Self {
        assert!(unique_and_bounded(v.len(), v));
        TotalRef { order: v }
    }

    pub unsafe fn new_unchecked(v: &'a [usize]) -> Self {
        TotalRef { order: v }
    }

    pub fn elements(&self) -> usize {
        self.order.len()
    }

    pub fn top(&self, n: usize) -> &[usize] {
        &self.order[..n]
    }

    pub fn to_incomplete(self) -> ChainRef<'a> {
        let Self { order } = self;
        let elements = order.len();
        ChainRef { elements, order }
    }
}

impl OrderRef for TotalRef<'_> {
    type Owned = Total;

    fn to_owned(self) -> Self::Owned {
        Total { order: self.order.to_vec() }
    }
}
