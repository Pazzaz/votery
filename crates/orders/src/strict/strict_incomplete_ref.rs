use super::strict_ref::StrictRef;
use crate::{tied::TiedIRef, unique};

/// A possibly incomplete order without any ties
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct StrictIRef<'a> {
    pub(crate) elements: usize,
    pub order: &'a [usize],
}

impl<'a> StrictIRef<'a> {
    pub fn new(elements: usize, order: &'a [usize]) -> Self {
        debug_assert!(unique(order));
        StrictIRef { elements, order }
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn top(&self, n: usize) -> Self {
        StrictIRef::new(self.elements, &self.order[0..n])
    }

    pub fn winner(&self) -> usize {
        debug_assert!(!self.order.is_empty());
        self.order[0]
    }

    pub fn to_tied(self, tied: &'a [bool]) -> TiedIRef<'a> {
        TiedIRef::new(self.elements, self.order, tied)
    }

    /// Converts to complete ranking. Panics if not all elements are ranked.
    pub fn to_complete(self) -> StrictRef<'a> {
        let StrictIRef { elements, order } = self;
        assert!(elements == order.len());
        StrictRef { order }
    }
}
