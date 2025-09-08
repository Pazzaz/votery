// TODO: A lot of implementation details are shared between PartialRanking and
// TotalRanking. Should they be combined somehow?
use rand::seq::SliceRandom;

use super::TotalRef;
use crate::{DenseOrders, get_order, pairwise_lt};

#[derive(Debug, PartialEq, Eq)]
pub struct TotalDense {
    pub(crate) orders: Vec<usize>,
    pub(crate) elements: usize,
}

impl Clone for TotalDense {
    fn clone(&self) -> Self {
        Self { orders: self.orders.clone(), elements: self.elements }
    }

    fn clone_from(&mut self, source: &Self) {
        self.orders.clone_from(&source.orders);
        self.elements = source.elements;
    }
}

pub enum AddError {
    Alloc,
    Elements,
}

impl TotalDense {
    pub fn new(elements: usize) -> Self {
        TotalDense { orders: Vec::new(), elements }
    }

    pub fn iter(&self) -> impl Iterator<Item = TotalRef<'_>> {
        (0..self.len()).map(|i| self.get(i))
    }

    // Check if a given total ranking is valid, i.e.
    // 1. len(orders) % elements == 0
    // 2. Every ranking is total
    #[cfg(test)]
    fn valid(&self) -> bool {
        if self.elements == 0 {
            self.orders.is_empty()
        } else if self.orders.len() % self.elements != 0 {
            false
        } else {
            let seen: &mut [bool] = &mut vec![false; self.elements];
            for i in 0..self.len() {
                seen.fill(false);
                for j in 0..self.elements {
                    let order = self.orders[i * self.elements + j];
                    if order >= self.elements {
                        return false;
                    }
                    if seen[order] {
                        return false;
                    }
                    seen[order] = true;
                }
                for &s in &*seen {
                    if !s {
                        return false;
                    }
                }
            }
            true
        }
    }
}

impl<'a> DenseOrders<'a> for TotalDense {
    type Order = TotalRef<'a>;
    fn elements(&self) -> usize {
        self.elements
    }

    fn len(&self) -> usize {
        if self.elements == 0 { 0 } else { self.orders.len() / self.elements }
    }

    fn try_get(&'a self, i: usize) -> Option<Self::Order> {
        if i < self.len() {
            let start = i * self.elements;
            let end = (i + 1) * self.elements;
            let s = &self.orders[start..end];
            // TODO: Use unsafe?
            Some(TotalRef::new(s))
        } else {
            None
        }
    }

    fn add(&mut self, v: Self::Order) -> Result<(), &'static str> {
        // TODO: Make this the normal add
        fn inner(a: &mut TotalDense, v: TotalRef) -> Result<(), AddError> {
            if v.elements() != a.elements || a.elements == 0 {
                Err(AddError::Elements)
            } else if a.orders.try_reserve(a.elements).is_err() {
                Err(AddError::Alloc)
            } else {
                a.orders.extend_from_slice(v.order);
                Ok(())
            }
        }
        inner(self, v).map_err(|_| "Could not add order")
    }

    fn remove_element(&mut self, target: usize) -> Result<(), &'static str> {
        let targets = &[target];
        if targets.is_empty() {
            return Ok(());
        }
        debug_assert!(pairwise_lt(targets));
        let new_elements = self.elements - targets.len();
        for i in 0..self.len() {
            let mut t_i = 0;
            let mut offset = 0;
            for j in 0..self.elements {
                if targets[t_i] == j {
                    t_i += 1;
                    offset += 1;
                } else {
                    let old_index = i * self.elements + j;
                    let new_index = i * new_elements + (j - offset);
                    debug_assert!(new_index <= old_index);
                    self.orders[new_index] = self.orders[old_index];
                }
            }
            let new_order = &mut self.orders[(i * new_elements)..((i + 1) * new_elements)];

            // TODO: Can we do this in place?
            new_order.clone_from_slice(&get_order(new_order, false));
        }
        self.orders.truncate(self.len() * new_elements);
        self.elements = new_elements;
        Ok(())
    }

    fn generate_uniform<R: rand::Rng>(&mut self, rng: &mut R, new_orders: usize) {
        if self.elements == 0 {
            return;
        }
        let mut v: Vec<usize> = (0..self.elements).collect();
        self.orders.reserve(self.elements * new_orders);
        for _ in 0..new_orders {
            v.shuffle(rng);
            self.orders.extend_from_slice(&v);
        }
    }
}
