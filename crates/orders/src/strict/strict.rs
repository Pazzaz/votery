use std::cmp;

use rand::{Rng, prelude::SliceRandom};

use super::{strict_incomplete::Chain, strict_ref::TotalRef};
use crate::{Order, OrderOwned, partial_order::PartialOrder, unique_and_bounded};

/// An owned total order.
///
/// Consists of a list of elements, arranged from highest to lowest elements,
/// ordering all elements.
#[derive(Debug)]
pub struct Total {
    pub(crate) order: Vec<usize>,
}

impl Clone for Total {
    fn clone(&self) -> Self {
        Self { order: self.order.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.order.clone_from(&source.order);
    }
}

impl Total {
    /// Create a new total order.
    ///
    /// # Panics
    ///
    /// Panics if the input is not a total order.
    pub fn new(v: Vec<usize>) -> Self {
        Self::try_new(v).unwrap()
    }

    /// Create a new total order.
    ///
    /// Returns `None` if the input is not a total order.
    pub fn try_new(v: Vec<usize>) -> Option<Self> {
        if unique_and_bounded(v.len(), &v) { Some(Self { order: v }) } else { None }
    }

    /// Create a new total order.
    ///
    /// # Safety
    ///
    /// Expects `v` to be a total order.
    pub unsafe fn new_unchecked(v: Vec<usize>) -> Self {
        Self { order: v }
    }

    /// Create a new total order of `n` elements, in the natural order,
    pub fn new_default(n: usize) -> Self {
        Total { order: (0..n).collect() }
    }

    /// Clones from `source` to `self`, similar to [`Clone::clone_from`].
    pub fn clone_from_ref(&mut self, source: TotalRef) {
        self.order.clone_from_slice(source.order);
    }

    /// Get the order as a `Vec`.
    pub fn into_inner(self) -> Vec<usize> {
        let Self { order } = self;
        order
    }

    /// Remove element `n` from the order.
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

    /// Sort the order using a closure, similar to
    /// [`[usize]::sort_by`](slice::sort_by).
    pub fn sort_by<F: Fn(&usize, &usize) -> cmp::Ordering>(&mut self, f: F) {
        self.order.sort_by(f);
    }

    pub fn random<R: Rng>(rng: &mut R, elements: usize) -> Total {
        let mut order: Vec<usize> = (0..elements).collect();
        order.shuffle(rng);
        Total { order }
    }

    /// Lossless conversion to `Chain`.
    pub fn to_incomplete(self) -> Chain {
        let Self { order } = self;
        let elements = order.len();
        Chain { elements, order }
    }
}

impl Order for Total {
    fn elements(&self) -> usize {
        self.order.len()
    }

    fn len(&self) -> usize {
        self.order.len()
    }

    fn to_partial(self) -> PartialOrder {
        self.to_incomplete().to_partial()
    }
}

impl<'a> OrderOwned<'a> for Total {
    type Ref = TotalRef<'a>;

    fn as_ref(&'a self) -> Self::Ref {
        TotalRef { order: &self.order }
    }
}
