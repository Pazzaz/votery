use rand::{
    distr::{Distribution, Uniform},
    seq::SliceRandom,
};

use crate::{
    rank::{Rank, RankRef}, DenseOrders, OrderOwned
};

use super::dense_complete::StrictOrdersComplete;

/// SOI - Strict Orders - Incomplete List
///
/// A packed list of (possibly incomplete) strict orders, with related methods.
#[derive(Clone, Debug)]
pub struct StrictOrdersIncomplete {
    pub(crate) orders: Vec<usize>,

    // Length of each order
    pub(crate) order_len: Vec<usize>,
    pub(crate) elements: usize,
}

impl StrictOrdersIncomplete {
    pub fn new(elements: usize) -> Self {
        StrictOrdersIncomplete { orders: Vec::new(), order_len: Vec::new(), elements }
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    pub fn orders_count(&self) -> usize {
        self.order_len.len()
    }

    /// Return true if it was a valid order.
    pub fn add_from_str(&mut self, s: &str) -> bool {
        let mut order = Vec::with_capacity(self.elements);
        let mut seen = vec![false; self.elements];
        for number in s.split(',') {
            let i: usize = match number.parse() {
                Ok(n) => n,
                Err(_) => return false,
            };
            if i >= self.elements || seen[i] {
                return false;
            }
            seen[i] = true;
            order.push(i);
        }
        let order = Rank::new(self.elements, order);
        self.add(order.as_ref()).unwrap();
        debug_assert!(self.valid());
        true
    }

    /// Returns true if this struct is in a valid state, used for debugging.
    fn valid(&self) -> bool {
        let mut seen = vec![false; self.elements];
        for order in self {
            seen.fill(false);
            for &i in order {
                if i >= self.elements || seen[i] {
                    return false;
                }
                seen[i] = true;
            }
        }
        true
    }

    pub fn order_i(&self, i: usize) -> RankRef {
        let start: usize = self.order_len[0..i].iter().sum();
        let end = start + self.order_len[i];
        RankRef::new(self.elements, &self.orders[start..end])
    }
}

impl<'a> DenseOrders<'a> for StrictOrdersIncomplete {
    type Order = RankRef<'a>;

    fn elements(&self) -> usize {
        self.elements
    }

    fn add(&mut self, v: Self::Order) -> Result<(), &'static str> {
        debug_assert!(v.elements == self.elements);
        self.orders.reserve(v.len());
        let mut seen = vec![false; self.elements];
        for &i in v.order {
            debug_assert!(i < self.elements || !seen[i]);
            seen[i] = true;
        }
        self.order_len.push(v.len());
        self.orders.extend_from_slice(v.order);
        debug_assert!(self.valid());
        Ok(())
    }

    fn remove_element(&mut self, target: usize) -> Result<(), &'static str> {
        if self.orders_count() == 0 {
            return Ok(());
        }
        // where in `orders` will we write
        let mut j_1 = 0;
        // where in `order_len` are we reading
        let mut i_2 = 0;
        // where in `order_len` will we write
        let mut j_2 = 0;

        let mut last = 0;
        let mut i_1 = 0;
        while i_1 < self.orders.len() {
            let el = self.orders[i_1];
            match el.cmp(&target) {
                std::cmp::Ordering::Equal => {
                    self.order_len[i_2] -= 1;
                }
                std::cmp::Ordering::Greater => {
                    self.orders[j_1] = el - 1;
                    j_1 += 1;
                }
                std::cmp::Ordering::Less => {
                    self.orders[j_1] = el;
                    j_1 += 1;
                }
            }
            i_1 += 1;
            if i_1 == last + self.order_len[i_2] {
                last += self.order_len[i_2];
                if self.order_len[i_2] != 0 {
                    self.order_len[j_2] = self.order_len[i_2];
                    j_2 += 1;
                }
                i_2 += 1;
            }
        }
        self.orders.drain(j_1..);
        self.order_len.drain(i_2..);
        debug_assert!(self.valid());
        Ok(())
    }

    fn generate_uniform<R: rand::Rng>(&mut self, rng: &mut R, new_orders: usize) {
        if self.elements == 0 {
            return;
        }
        let v: &mut [usize] = &mut (0..self.elements).collect::<Vec<usize>>();
        self.orders.reserve(self.elements * new_orders);
        let range = Uniform::new(0, self.elements).unwrap();
        for _ in 0..new_orders {
            let elements = range.sample(rng) + 1;
            v.shuffle(rng);
            for &el in &v[..elements] {
                self.orders.push(el);
            }
            self.order_len.push(elements);
        }
        debug_assert!(self.valid());
    }
}

impl<'a> IntoIterator for &'a StrictOrdersIncomplete {
    type Item = &'a [usize];
    type IntoIter = StrictOrdersIncompleteIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        StrictOrdersIncompleteIterator { orig: self, i: 0, start: 0 }
    }
}

pub struct StrictOrdersIncompleteIterator<'a> {
    orig: &'a StrictOrdersIncomplete,
    i: usize,
    start: usize,
}

impl<'a> Iterator for StrictOrdersIncompleteIterator<'a> {
    type Item = &'a [usize];
    fn next(&mut self) -> Option<Self::Item> {
        let len = self.orig.order_len[self.i];
        let order = &self.orig.orders[self.start..(self.start + len)];
        self.i += 1;
        self.start += len;
        Some(order)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.orig.order_len.len() - self.i;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for StrictOrdersIncompleteIterator<'_> {}

impl From<StrictOrdersComplete> for StrictOrdersIncomplete {
    fn from(value: StrictOrdersComplete) -> Self {
        let orders: usize = value.orders();
        let s = StrictOrdersIncomplete {
            orders: value.orders,
            order_len: vec![value.elements; orders],
            elements: value.elements,
        };
        debug_assert!(s.valid());
        s
    }
}
