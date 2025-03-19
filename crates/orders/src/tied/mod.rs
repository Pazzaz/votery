mod dense;
mod dense_complete;
mod groups;
mod split_ref;
mod tied_incomplete;
mod tied_incomplete_ref;

pub use dense::*;
pub use dense_complete::*;
pub use groups::*;
use split_ref::SplitRef;
pub use tied_incomplete::*;
pub use tied_incomplete_ref::*;

use crate::{Order, OrderOwned, OrderRef, unique_and_bounded};

pub struct Tied {
    order: Vec<usize>,
    tied: Vec<bool>,
}

impl Tied {
    pub fn new(order: Vec<usize>, tied: Vec<bool>) -> Self {
        Self::try_new(order, tied).unwrap()
    }

    pub fn try_new(order: Vec<usize>, tied: Vec<bool>) -> Option<Self> {
        let correct_len = order.len() == 0 && tied.len() == 0 || tied.len() + 1 == order.len();
        if correct_len && unique_and_bounded(order.len(), &order) {
            Some(Tied { order, tied })
        } else {
            None
        }
    }

    pub unsafe fn new_unchecked(order: Vec<usize>, tied: Vec<bool>) -> Self {
        Tied { order, tied }
    }

    pub fn order(&self) -> &[usize] {
        &self.order
    }

    pub fn tied(&self) -> &[bool] {
        &self.tied
    }
}

impl Order for Tied {
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

impl<'a> OrderOwned<'a> for Tied {
    type Ref = TiedRef<'a>;

    fn as_ref(&'a self) -> Self::Ref {
        TiedRef::new(&self.order, &self.tied)
    }
}

impl From<Tied> for TiedI {
    fn from(Tied { order, tied }: Tied) -> Self {
        TiedI::new(order.len(), order, tied)
    }
}

pub struct TiedRef<'a> {
    order_tied: SplitRef<'a>,
}

impl<'a> TiedRef<'a> {
    pub fn new(order: &'a [usize], tied: &'a [bool]) -> Self {
        Self::try_new(order, tied).unwrap()
    }

    pub fn try_new(order: &'a [usize], tied: &'a [bool]) -> Option<Self> {
        let correct_len = order.len() == 0 && tied.len() == 0 || tied.len() + 1 == order.len();
        if correct_len && unique_and_bounded(order.len(), order) {
            Some(TiedRef { order_tied: SplitRef::new(order, tied) })
        } else {
            None
        }
    }

    pub unsafe fn new_unchecked(order: &'a [usize], tied: &'a [bool]) -> Self {
        TiedRef { order_tied: SplitRef::new(order, tied) }
    }

    pub fn elements(&self) -> usize {
        self.order().len()
    }

    pub fn order(&self) -> &'a [usize] {
        self.order_tied.a()
    }

    pub fn tied(&self) -> &'a [bool] {
        self.order_tied.b()
    }

    pub fn winners(&self) -> &'a [usize] {
        let ti: TiedIRef = self.into();
        ti.winners()
    }

    pub fn iter_groups(&self) -> GroupIterator {
        GroupIterator { order: self.into() }
    }
}

impl<'a> OrderRef for TiedRef<'a> {
    type Owned = Tied;

    fn to_owned(self) -> Self::Owned {
        Tied::new(self.order().to_vec(), self.tied().to_vec())
    }
}

impl<'a> From<TiedRef<'a>> for TiedIRef<'a> {
    fn from(value: TiedRef<'a>) -> Self {
        TiedIRef::new(value.elements(), value.order(), value.tied())
    }
}

impl<'a> From<&TiedRef<'a>> for TiedIRef<'a> {
    fn from(value: &TiedRef<'a>) -> Self {
        TiedIRef::new(value.elements(), value.order(), value.tied())
    }
}
