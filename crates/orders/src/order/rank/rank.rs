//! Different orders of elements
//!
//! There are two main types of orders:
//! - [`Rank`] - An order of elements without ties, where earlier elements are
//!   ranked higher. There are also reference versions which don't own the data:
//!   [`RankRef`]
//! - [`TiedRank`] - An order of elements with ties,  where earlier elements are
//!   ranked higher and where some elements can be tied with others. There are
//!   also reference versions which don't own the data: [`TiedRankRef`].

use super::{rank_ref::RankRef, total_rank::TotalRank};
use crate::order::{
    Order, OrderOwned, OrderRef,
    partial_order::{PartialOrder, PartialOrderManual},
    unique,
};

/// A possibly incomplete order without any ties, owned version of [`RankRef`]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rank {
    pub(crate) elements: usize,
    pub(crate) order: Vec<usize>,
}

impl Rank {
    pub fn new(elements: usize, order: Vec<usize>) -> Self {
        debug_assert!(unique(&order));
        Rank { elements, order }
    }

    pub fn parse_order(elements: usize, s: &str) -> Option<Self> {
        let mut order: Vec<usize> = Vec::with_capacity(elements);
        for number in s.split(',') {
            let n: usize = match number.parse() {
                Ok(n) => n,
                Err(_) => return None,
            };
            if n >= elements {
                return None;
            }
            order.push(n);
        }

        Some(Rank::new(elements, order))
    }

    /// Converts to complete ranking. Panics if not all elements are ranked.
    pub fn to_complete(self) -> TotalRank {
        let Rank { elements, order } = self;
        assert!(elements == order.len());
        TotalRank { order }
    }
}

impl Order for Rank {
    fn elements(&self) -> usize {
        self.elements
    }

    fn len(&self) -> usize {
        self.order.len()
    }

    fn as_partial(self) -> PartialOrder {
        let mut manual = PartialOrderManual::new(self.len());
        let seen: &mut [bool] = &mut vec![false; self.len()];
        for (i1, e1) in self.order.iter().enumerate() {
            seen[*e1] = true;
            for e2 in &self.order[(i1 + 1)..] {
                manual.set(*e2, *e1);
            }
        }
        let rest: Vec<usize> = (*seen)
            .into_iter()
            .enumerate()
            .filter_map(|(i, b)| if !b { Some(i) } else { None })
            .collect();

        for &upper in &self.order {
            for &lower in &rest {
                manual.set(lower, upper);
            }
        }

        // SAFETY: We set the relations in `self.order`, including transitive relations,
        // and every element in `self.order` is larger than the rest. The
        // elements in `rest` have no relations with eachother.
        let out = unsafe { manual.finish_unchecked() };
        debug_assert!(out.valid());
        out
    }
}

impl<'a> OrderOwned<'a> for Rank {
    type Ref = RankRef<'a>;

    fn as_ref(&'a self) -> Self::Ref {
        RankRef { elements: self.elements, order: &self.order }
    }
}

impl OrderRef for RankRef<'_> {
    type Owned = Rank;

    fn as_owned(self) -> Self::Owned {
        Rank::new(self.elements, self.order.to_vec())
    }
}
