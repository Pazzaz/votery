use super::{strict::Strict, strict_incomplete_ref::StrictIRef};
use crate::{unique_and_bounded, OrderRef};

#[derive(Debug, Clone, Copy)]
pub struct StrictRef<'a> {
    pub(crate) order: &'a [usize],
}

impl<'a> StrictRef<'a> {
    /// Create a new `StrictRef` from a permutation of `0..s.len()`.
    pub fn new(v: &'a [usize]) -> Self {
        assert!(unique_and_bounded(v.len(), v));
        StrictRef { order: v }
    }

    pub unsafe fn new_unchecked(v: &'a [usize]) -> Self {
        StrictRef { order: v }
    }

    pub fn elements(&self) -> usize {
        self.order.len()
    }

    pub fn top(&self, n: usize) -> &[usize] {
        &self.order[..n]
    }

    pub fn to_incomplete(self) -> StrictIRef<'a> {
        let Self { order } = self;
        let elements = order.len();
        StrictIRef { elements, order }
    }
}

impl OrderRef for StrictRef<'_> {
    type Owned = Strict;

    fn to_owned(self) -> Self::Owned {
        Strict { order: self.order.to_vec() }
    }
}
