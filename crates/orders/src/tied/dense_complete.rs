use rand::{
    distr::{Bernoulli, Distribution},
    seq::{IndexedRandom, SliceRandom},
};

use crate::{
    cardinal::CardinalDense, specific::SpecificDense, strict::StrictOrdersComplete, tied::TiedIRef,
};

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
        if self.elements == 0 { 0 } else { self.orders() / self.elements }
    }

    // TODO: Use `TiedRef`
    pub fn get(&self, i: usize) -> TiedIRef {
        assert!(i < self.count());
        let start = i * self.elements;
        let end = (i + 1) * self.elements;
        TiedIRef::new(self.elements, &self.orders[start..end], &self.ties[(start - i)..(end - i)])
    }

    pub fn iter(&self) -> impl Iterator<Item = TiedIRef> {
        (0..self.count()).map(|i| self.get(i))
    }

    pub fn add(&mut self, v: TiedIRef) {
        let order = v.order();
        let tie = v.tied();
        debug_assert!(order.len() == self.elements);
        debug_assert!(!order.is_empty());
        debug_assert!(tie.len() + 1 == order.len());
        self.orders.reserve(order.len() * self.elements);
        self.ties.reserve(tie.len() * (self.elements - 1));
        let mut seen = vec![false; self.elements];
        for &i in order {
            debug_assert!(i < self.elements || !seen[i]);
            seen[i] = true;
            self.orders.push(i);
        }
        self.ties.extend(tie);
        debug_assert!(self.valid());
    }

    pub fn orders(&self) -> usize {
        debug_assert!(self.orders.len() % self.elements == 0);
        self.orders.len() / self.elements
    }

    /// Add a single order from a string. Return true if it was a valid order.
    pub fn add_from_str(&mut self, s: &str) -> bool {
        let mut order: Vec<usize> = Vec::with_capacity(self.elements);
        let mut tie: Vec<bool> = Vec::with_capacity(self.elements);
        let mut grouped = false;
        for part in s.split(',') {
            let number: &str = if grouped {
                part.strip_suffix('}').map_or(part, |s| {
                    grouped = !grouped;
                    s
                })
            } else {
                part.strip_prefix('{').map_or(part, |s| {
                    grouped = !grouped;
                    s
                })
            };
            let n: usize = match number.parse() {
                Ok(n) => n,
                Err(_) => return false,
            };
            if n >= self.elements {
                return false;
            }
            order.push(n);
            tie.push(grouped);
        }
        // The last one will never be tied, so we'll ignore it.
        tie.pop();

        // We didn't end our group or we didn't list all elements
        if grouped || order.len() != self.elements {
            return false;
        }
        self.add(TiedIRef::new(self.elements, &order, &tie));
        debug_assert!(self.valid());
        true
    }

    /// Returns true if this struct is in a valid state, used for debugging.
    fn valid(&self) -> bool {
        if self.orders.len() != self.orders() * self.elements
            || self.ties.len() != self.orders() * (self.elements - 1)
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
        debug_assert!(self.valid());
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

    /// Convert each ordering to a cardinal order, with the highest rank
    /// elements receiving a score of `self.elements`.
    ///
    /// Returns `Err` if it failed to allocate
    pub fn to_cardinal(&self) -> Result<CardinalDense, &'static str> {
        let mut orders: Vec<usize> = Vec::new();
        orders.try_reserve_exact(self.elements * self.orders()).or(Err("Could not allocate"))?;
        let max = self.elements - 1;
        let mut new_order = vec![0; self.elements];
        for order in self.iter() {
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
        let v = CardinalDense { orders, elements: self.elements, min: 0, max };
        debug_assert!(v.valid());
        Ok(v)
    }
}

impl From<StrictOrdersComplete> for TiedDense {
    fn from(value: StrictOrdersComplete) -> Self {
        let orders: usize = value.orders();
        let s = TiedDense {
            orders: value.orders,
            ties: vec![false; (value.elements - 1) * orders],
            elements: value.elements,
        };
        s
    }
}
