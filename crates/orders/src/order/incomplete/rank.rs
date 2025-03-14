//! Different orders of elements
//!
//! There are two main types of orders:
//! - [`Rank`] - An order of elements without ties, where earlier elements are
//!   ranked higher. There are also reference versions which don't own the data:
//!   [`RankRef`]
//! - [`TiedRank`] - An order of elements with ties,  where earlier elements
//!   are ranked higher and where some elements can be tied with others. There
//!   are also reference versions which don't own the data: [`TiedRankRef`].

use crate::order::RankRef;

use super::unique;

/// A possibly incomplete order without any ties, owned version of [`RankRef`]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rank {
    elements: usize,
    order: Vec<usize>,
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
}

// Sort two arrays, sorted according to the values in `b`.
// Uses insertion sort
pub(crate) fn sort_using<A, B>(a: &mut [A], b: &mut [B])
where
    B: PartialOrd,
{
    debug_assert!(a.len() == b.len());
    let mut i: usize = 1;
    while i < b.len() {
        let mut j = i;
        while j > 0 && b[j - 1] > b[j] {
            a.swap(j, j - 1);
            b.swap(j, j - 1);
            j -= 1;
        }
        i += 1;
    }
}
