use std::{
    fmt::{self, Display},
    io::BufRead,
};

// TODO: A lot of implementation details are shared between PartialRanking and
// TotalRanking. Should they be combined somehow?
use rand::seq::SliceRandom;

use super::StrictRef;
use crate::{DenseOrders, get_order, pairwise_lt, remove_newline};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrictDense {
    pub orders: Vec<usize>,
    pub(crate) elements: usize,
}

pub enum AddError {
    Alloc,
    Elements,
}

impl StrictDense {
    pub fn new(elements: usize) -> Self {
        StrictDense { orders: Vec::new(), elements }
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    pub fn add(&mut self, v: StrictRef) -> Result<(), AddError> {
        if v.elements() != self.elements {
            Err(AddError::Elements)
        } else if let Err(_) = self.orders.try_reserve(self.elements) {
            Err(AddError::Alloc)
        } else {
            self.orders.extend_from_slice(&v.order);
            Ok(())
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = StrictRef> {
        (0..self.count()).map(|i| self.get(i))
    }

    // Check if a given total ranking is valid, i.e.
    // 1. len(orders) % elements == 0
    // 2. Every ranking is total
    fn valid(&self) -> bool {
        if self.elements == 0 {
            self.orders.len() == 0
        } else if self.orders.len() % self.elements != 0 {
            false
        } else {
            let seen: &mut [bool] = &mut vec![false; self.elements];
            for i in 0..self.count() {
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

    pub fn parse_add<T: BufRead>(&mut self, f: &mut T) -> Result<(), &'static str> {
        if self.elements == 0 {
            return Ok(());
        }
        let mut buf = String::with_capacity(self.elements * 2);

        // Used to find gaps in a ranking
        let seen: &mut [bool] = &mut vec![false; self.elements];
        loop {
            buf.clear();
            let bytes = f.read_line(&mut buf).or(Err("Failed to read line of order"))?;
            if bytes == 0 {
                break;
            }
            remove_newline(&mut buf);

            seen.fill(false);
            let mut count = 0;
            for s in buf.split(',') {
                count += 1;
                let v: usize = s.parse().or(Err("Order is not a number"))?;
                if v >= self.elements {
                    return Err("Ranking of element larger than or equal to number of elements");
                }
                if seen[v] {
                    return Err("Not a total ranking");
                }
                seen[v] = true;
                self.orders.push(v);
            }
            match count.cmp(&self.elements) {
                std::cmp::Ordering::Greater => return Err("Too many elements listed in order"),
                std::cmp::Ordering::Less => return Err("Too few elements listed in order"),
                std::cmp::Ordering::Equal => {}
            }
            for &s in &*seen {
                if !s {
                    return Err("Invalid order, gap in ranking");
                }
            }
        }
        debug_assert!(self.valid());
        Ok(())
    }
}

impl Display for StrictDense {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let end = self.count();
        write!(f, "[")?;
        for (i, v) in self.iter().enumerate() {
            if i == end.saturating_sub(1) {
                write!(f, "({})", v)?;
            } else {
                write!(f, "({}), ", v)?;
            }
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl<'a> DenseOrders<'a> for StrictDense {
    type Order = StrictRef<'a>;
    fn elements(&self) -> usize {
        self.elements
    }

    fn count(&self) -> usize {
        if self.elements == 0 { 0 } else { self.orders.len() / self.elements }
    }

    fn try_get(&'a self, i: usize) -> Option<StrictRef<'a>> {
        if i >= self.count() {
            None
        } else {
            let start = i * self.elements;
            let end = (i + 1) * self.elements;
            let s = &self.orders[start..end];
            // TODO: Use unsafe?
            Some(StrictRef::new(s))
        }
    }

    fn add(&mut self, v: Self::Order) -> Result<(), &'static str> {
        self.orders.try_reserve(self.elements).or(Err("Could not add order"))?;
        self.orders.extend_from_slice(&v.order);
        Ok(())
    }

    fn remove_element(&mut self, target: usize) -> Result<(), &'static str> {
        let targets = &[target];
        if targets.is_empty() {
            return Ok(());
        }
        debug_assert!(pairwise_lt(targets));
        let new_elements = self.elements - targets.len();
        for i in 0..self.count() {
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
        self.orders.truncate(self.count() * new_elements);
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

// TODO: Why do we have two identical structs

/// SOC - Strict Orders - Complete List
///
/// A packed list of complete strict orders, with related methods. Each order is
/// a permutation of the elements
#[derive(Clone, Debug)]
pub struct StrictOrdersComplete {
    pub(crate) orders: Vec<usize>,
    pub(crate) elements: usize,
}

impl StrictOrdersComplete {
    pub fn new(elements: usize) -> Self {
        StrictOrdersComplete { orders: Vec::new(), elements }
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    pub fn add(&mut self, order: &[usize]) {
        debug_assert!(order.len() == self.elements);
        self.orders.reserve(self.elements);
        let mut seen = vec![false; self.elements];
        for &i in order {
            debug_assert!(i < self.elements || !seen[i]);
            seen[i] = true;
            self.orders.push(i);
        }
        debug_assert!(self.valid());
    }

    pub fn orders(&self) -> usize {
        debug_assert!(self.orders.len() % self.elements == 0);
        self.orders.len() / self.elements
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
        if order.len() != self.elements {
            return false;
        }
        self.add(&order);
        debug_assert!(self.valid());
        true
    }

    /// Returns true if this struct is in a valid state, used for debugging.
    fn valid(&self) -> bool {
        for order in self {
            let mut seen = vec![false; self.elements];
            for &i in order {
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
        self.orders.reserve(self.elements * new_orders);
        for _ in 0..new_orders {
            v.shuffle(rng);
            for &el in &*v {
                self.orders.push(el);
            }
        }
        debug_assert!(self.valid());
    }
}

impl<'a> IntoIterator for &'a StrictOrdersComplete {
    type Item = &'a [usize];
    type IntoIter = StrictOrdersCompleteIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        StrictOrdersCompleteIterator { orig: self, i: 0 }
    }
}

pub struct StrictOrdersCompleteIterator<'a> {
    orig: &'a StrictOrdersComplete,
    i: usize,
}

impl<'a> Iterator for StrictOrdersCompleteIterator<'a> {
    type Item = &'a [usize];
    fn next(&mut self) -> Option<Self::Item> {
        let len = self.orig.elements;
        let start = self.i * self.orig.elements;
        let order = &self.orig.orders[start..(start + len)];
        self.i += 1;
        Some(order)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.orig.orders() - self.i;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for StrictOrdersCompleteIterator<'_> {}
