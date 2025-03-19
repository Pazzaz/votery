use std::cmp;

use super::{strict_incomplete::StrictI, strict_ref::StrictRef};
use crate::{Order, OrderOwned, unique_and_bounded};

#[derive(Debug)]
pub struct Strict {
    pub(crate) order: Vec<usize>,
}

impl Clone for Strict {
    fn clone(&self) -> Self {
        Self { order: self.order.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.order.clone_from(&source.order);
    }
}

impl Strict {
    pub fn new(v: Vec<usize>) -> Self {
        assert!(unique_and_bounded(v.len(), &v));
        Self { order: v }
    }

    pub unsafe fn new_unchecked(v: Vec<usize>) -> Self {
        Self { order: v }
    }

    pub fn new_default(n: usize) -> Self {
        Strict { order: (0..n).collect() }
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

    pub fn copy_from_ref(&mut self, other: StrictRef) {
        self.order.clear();
        self.order.extend_from_slice(other.order);
    }

    pub fn to_incomplete(self) -> StrictI {
        let Self { order } = self;
        let elements = order.len();
        StrictI { elements, order }
    }
}

impl Order for Strict {
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

impl<'a> OrderOwned<'a> for Strict {
    type Ref = StrictRef<'a>;

    fn as_ref(&'a self) -> Self::Ref {
        StrictRef { order: &self.order }
    }
}
