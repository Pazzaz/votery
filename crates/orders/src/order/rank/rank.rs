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
use crate::order::unique;

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

    pub fn elements(&self) -> usize {
        self.elements
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

    pub fn as_ref(&self) -> RankRef {
        RankRef { elements: self.elements, order: &self.order[..] }
    }

    /// Converts to complete ranking. Panics if not all elements are ranked.
    pub fn to_complete(self) -> TotalRank {
        let Rank { elements, order } = self;
        assert!(elements == order.len());
        TotalRank { order }
    }
}
