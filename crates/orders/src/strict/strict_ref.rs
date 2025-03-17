use super::{strict::Strict, strict_incomplete_ref::StrictIRef};
use crate::unique;

#[derive(Debug, Clone, Copy)]
pub struct StrictRef<'a> {
    pub(crate) order: &'a [usize],
}

impl<'a> StrictRef<'a> {
    pub fn new(v: &'a [usize]) -> Self {
        if !v.is_empty() {
            assert!(unique(v));
            assert!(v.contains(&0));
        }
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

    pub fn owned(&self) -> Strict {
        Strict { order: self.order.to_vec() }
    }

    pub fn to_incomplete(self) -> StrictIRef<'a> {
        let Self { order } = self;
        let elements = order.len();
        StrictIRef { elements, order }
    }
}
