use std::fmt::{self, Display};

use crate::OrderRef;

use super::strict_valid;

use super::{strict::Strict, strict_incomplete_ref::StrictIRef};

#[derive(Debug, Clone, Copy)]
pub struct StrictRef<'a> {
    pub(crate) order: &'a [usize],
}

impl<'a> StrictRef<'a> {
    /// Create a new `StrictRef` from a permutation of `0..s.len()`.
    pub fn new(v: &'a [usize]) -> Self {
        assert!(strict_valid(v));
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

impl Display for StrictRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let o = &self.order;
        if !o.is_empty() {
            for i in 0..(o.len() - 1) {
                let v = o[i];
                write!(f, "{}, ", v)?;
            }
            let v_last = o.last().unwrap();
            writeln!(f, "{}", v_last)?;
        }
        Ok(())
    }
}
