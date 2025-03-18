use rand::{
    distr::{Distribution, Uniform},
    seq::SliceRandom,
};

use super::dense::StrictOrdersComplete;
use crate::{
    DenseOrders, OrderOwned,
    strict::{StrictI, StrictIRef},
};

/// SOI - Strict Orders - Incomplete List
///
/// A packed list of (possibly incomplete) strict orders, with related methods.
#[derive(Clone, Debug)]
pub struct StrictIDense {
    pub(crate) orders: Vec<usize>,

    // End position of order
    pub(crate) order_end: Vec<usize>,
    pub(crate) elements: usize,
}

impl StrictIDense {
    pub fn new(elements: usize) -> Self {
        StrictIDense { orders: Vec::new(), order_end: Vec::new(), elements }
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    pub fn count(&self) -> usize {
        self.order_end.len()
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
        let order = StrictI::new(self.elements, order);
        self.add(order.as_ref()).unwrap();
        debug_assert!(self.valid());
        true
    }

    /// Returns true if this struct is in a valid state, used for debugging.
    fn valid(&self) -> bool {
        let mut seen = vec![false; self.elements];
        for v in self.iter() {
            seen.fill(false);
            for &i in v.order {
                if i >= self.elements || seen[i] {
                    return false;
                }
                seen[i] = true;
            }
        }
        for &o in &self.order_end {
            if o >= self.orders.len() {
                return false;
            }
        }
        for o in self.order_end.windows(2) {
            if o[0] >= o[1] {
                return false;
            }
        }
        true
    }

    pub fn get(&self, i: usize) -> StrictIRef {
        assert!(i < self.count());
        let start: usize = if i == 0 { 0 } else { self.order_end[i - 1] };
        let end = start + self.order_end[i];
        StrictIRef::new(self.elements, &self.orders[start..end])
    }

    pub fn iter(&self) -> impl Iterator<Item = StrictIRef> {
        (0..self.count()).map(|i| self.get(i))
    }
}

impl<'a> DenseOrders<'a> for StrictIDense {
    type Order = StrictIRef<'a>;

    fn elements(&self) -> usize {
        self.elements
    }

    fn add(&mut self, v: Self::Order) -> Result<(), &'static str> {
        assert!(v.elements == self.elements);
        self.orders.reserve(v.len());
        let start = self.order_end.last().unwrap_or(&0);
        self.order_end.push(*start + v.len());
        self.orders.extend_from_slice(v.order);
        Ok(())
    }

    fn remove_element(&mut self, _target: usize) -> Result<(), &'static str> {
        todo!();
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
            let start = self.order_end.last().unwrap_or(&0);
            self.order_end.push(*start + elements);
        }
        debug_assert!(self.valid());
    }
}

impl From<StrictOrdersComplete> for StrictIDense {
    fn from(value: StrictOrdersComplete) -> Self {
        let orders: usize = value.orders();
        let order_end = (0..orders).map(|i| (i + 1) * value.elements).collect();
        let s = StrictIDense { orders: value.orders, order_end, elements: value.elements };
        debug_assert!(s.valid());
        s
    }
}
