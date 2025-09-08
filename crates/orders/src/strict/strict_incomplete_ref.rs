use super::strict_ref::TotalRef;
use crate::{tied::TiedIRef, unique_and_bounded};

/// A possibly incomplete order without any ties
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ChainRef<'a> {
    pub(crate) elements: usize,
    pub(crate) order: &'a [usize],
}

impl<'a> ChainRef<'a> {
    /// Create a reference to a strictly ordered (possible incomplete) order.
    ///
    /// # Panics
    ///
    /// Elements in `order` have to be less than `elements`, without duplicates;
    /// otherwise it panics.
    pub fn new(elements: usize, order: &'a [usize]) -> Self {
        Self::try_new(elements, order).unwrap()
    }

    /// Tries to create a reference to a strictly ordered (possible incomplete)
    /// order.
    ///
    /// Elements in `order` have to be less than `elements`, without duplicates;
    /// otherwise it returns None.
    pub fn try_new(elements: usize, order: &'a [usize]) -> Option<Self> {
        if unique_and_bounded(elements, order) { Some(ChainRef { elements, order }) } else { None }
    }

    /// Create a reference to a strictly ordered (possible incomplete) order.
    ///
    /// # Safety
    ///
    /// Elements in `order` have to be less than `elements`, without duplicates.
    pub unsafe fn new_unchecked(elements: usize, order: &'a [usize]) -> Self {
        ChainRef { elements, order }
    }

    pub fn order(&self) -> &[usize] {
        self.order
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn top(&self, n: usize) -> Self {
        ChainRef::new(self.elements, &self.order[0..n])
    }

    pub fn winner(&self) -> usize {
        debug_assert!(!self.order.is_empty());
        self.order[0]
    }

    pub fn to_tied(self, tied: &'a [bool]) -> TiedIRef<'a> {
        TiedIRef::new(self.elements, self.order, tied)
    }
}

impl<'a> TryFrom<ChainRef<'a>> for TotalRef<'a> {
    type Error = ();

    /// Convert to complete order, returns `Err(())` if the order isn't actually
    /// complete.
    fn try_from(ChainRef { elements, order }: ChainRef<'a>) -> Result<Self, Self::Error> {
        if elements == order.len() { Ok(TotalRef { order }) } else { Err(()) }
    }
}
