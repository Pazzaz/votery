use rand::{
    distr::{Bernoulli, Distribution},
    seq::{IndexedRandom, SliceRandom},
};

use super::TiedRef;
use crate::{DenseOrders, cardinal::CardinalDense, specific::SpecificDense, strict::TotalDense};

/// TOC - Orders with Ties - Complete List
///
/// A packed list of complete orders with ties, with related methods.
#[derive(Clone, Debug)]
pub struct TiedDense {
    // Has length orders_count * elements
    pub(crate) orders: Vec<usize>,

    // Says if a value is tied with the next value.
    // Has length orders_count * (elements - 1)
    pub(crate) ties: Vec<bool>,
    pub(crate) elements: usize,
}

impl TiedDense {
    pub fn new(elements: usize) -> Self {
        TiedDense { orders: Vec::new(), ties: Vec::new(), elements }
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    pub fn count(&self) -> usize {
        if self.elements == 0 { 0 } else { self.orders.len() / self.elements }
    }

    // TODO: Use `TiedRef`
    pub fn get(&self, i: usize) -> TiedRef {
        assert!(i < self.count());
        let start = i * self.elements;
        let end = (i + 1) * self.elements;
        TiedRef::new(&self.orders[start..end], &self.ties[(start - i)..(end - i)])
    }

    pub fn iter(&self) -> impl Iterator<Item = TiedRef> {
        (0..self.count()).map(|i| self.get(i))
    }

    pub fn add(&mut self, v: TiedRef) {
        let order = v.order();
        let tie = v.tied();
        assert!(order.len() == self.elements && self.elements != 0);

        self.orders.reserve(order.len() * self.elements);
        self.ties.reserve(tie.len() * (self.elements - 1));

        self.orders.extend_from_slice(order);
        self.ties.extend_from_slice(tie);
    }

    /// Returns true if this struct is in a valid state, used for debugging.
    fn valid(&self) -> bool {
        if self.orders.len() != self.count() * self.elements
            || self.ties.len() != self.count() * (self.elements - 1)
        {
            return false;
        }
        let mut seen = vec![false; self.elements];
        for order in self.iter() {
            seen.fill(false);
            if order.order().len() != self.elements || order.tied().len() != self.elements - 1 {
                return false;
            }
            for &i in order.order() {
                if i >= self.elements || seen[i] {
                    return false;
                }
                seen[i] = true;
            }
        }
        true
    }

    pub fn generate_uniform<R: rand::Rng>(&mut self, rng: &mut R, new_orders: usize) {
        if self.elements == 0 {
            return;
        }
        let v: &mut [usize] = &mut (0..self.elements).collect::<Vec<usize>>();
        self.orders.reserve(new_orders * self.elements);
        self.ties.reserve(new_orders * (self.elements - 1));
        let dist = Bernoulli::new(0.5).unwrap();
        for _ in 0..new_orders {
            v.shuffle(rng);
            for &el in &*v {
                self.orders.push(el);
            }

            for _ in 0..(self.elements - 1) {
                let b = dist.sample(rng);
                self.ties.push(b);
            }
        }
    }

    /// Pick a winning element from each ordering, randomly from their highest
    /// ranked (tied) elements.
    pub fn to_specific_using<R: rand::Rng>(self, rng: &mut R) -> SpecificDense {
        let elements = self.elements;
        let mut orders: SpecificDense =
            self.iter().map(|v| *v.winners().choose(rng).unwrap()).collect();

        orders.set_elements(elements);
        orders
    }
}

impl TryFrom<TiedDense> for CardinalDense {
    type Error = &'static str;

    /// Convert each ordering to a cardinal order, with the highest rank
    /// elements receiving a score of `self.elements`.
    ///
    /// Returns `Err` if it failed to allocate.
    fn try_from(value: TiedDense) -> Result<Self, Self::Error> {
        let mut orders: Vec<usize> = Vec::new();
        orders.try_reserve_exact(value.elements * value.count()).or(Err("Could not allocate"))?;
        let max = value.elements - 1;
        let mut new_order = vec![0; value.elements];
        for order in value.iter() {
            for (i, group) in order.iter_groups().enumerate() {
                for &c in group {
                    debug_assert!(max >= i);
                    new_order[c] = max - i;
                }
            }
            // `order` is a ranking of all elements, so `new_order` will be different
            // between iterations.
            orders.extend(&new_order);
        }
        Ok(CardinalDense { orders, elements: value.elements, min: 0, max })
    }
}

impl From<TotalDense> for TiedDense {
    fn from(value: TotalDense) -> Self {
        let orders: usize = value.count();
        TiedDense {
            orders: value.orders,
            ties: vec![false; (value.elements - 1) * orders],
            elements: value.elements,
        }
    }
}

impl<'a> FromIterator<TiedRef<'a>> for TiedDense {
    /// Panics if any orders have a different number of elements.
    fn from_iter<T: IntoIterator<Item = TiedRef<'a>>>(iter: T) -> Self {
        let mut ii = iter.into_iter();
        if let Some(first_value) = ii.next() {
            let elements = first_value.elements();
            let mut out = TiedDense::new(elements);
            out.add(first_value);
            for v in ii {
                assert!(v.elements() == elements);
                out.add(v);
            }
            out
        } else {
            TiedDense::new(0)
        }
    }
}
