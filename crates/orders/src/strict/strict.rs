use std::cmp;

use super::{strict_incomplete::Chain, strict_ref::TotalRef};
use crate::{Order, OrderOwned, unique_and_bounded};

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
    pub fn new(v: Vec<usize>) -> Self {
        assert!(unique_and_bounded(v.len(), &v));
        Self { order: v }
    }

    pub unsafe fn new_unchecked(v: Vec<usize>) -> Self {
        Self { order: v }
    }

    pub fn new_default(n: usize) -> Self {
        Total { order: (0..n).collect() }
    }

    pub fn get_inner(self) -> Vec<usize> {
        let Self { order } = self;
        order
    }

    /// Remove element `n` from the order
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

    pub fn sort_by<F: Fn(&usize, &usize) -> cmp::Ordering>(&mut self, f: F) {
        self.order.sort_by(f);
    }

    pub fn copy_from_ref(&mut self, other: TotalRef) {
        self.order.clear();
        self.order.extend_from_slice(other.order);
    }

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

    fn to_partial(self) -> crate::partial_order::PartialOrder {
        todo!()
    }
}

impl<'a> OrderOwned<'a> for Total {
    type Ref = TotalRef<'a>;

    fn as_ref(&'a self) -> Self::Ref {
        TotalRef { order: &self.order }
    }
}
