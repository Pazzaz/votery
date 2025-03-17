use std::{
    fmt::{self, Display},
    io::BufRead,
};

// TODO: A lot of implementation details are shared between PartialRanking and
// TotalRanking. Should they be combined somehow?
use rand::seq::SliceRandom;

use super::{DenseOrders, remove_newline, toi::TiedOrdersIncomplete};
use crate::{get_order, pairwise_lt};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TotalRankingDense {
    // Has size elements * orders_count
    pub orders: Vec<usize>,
    pub(crate) elements: usize,
    pub orders_count: usize,
}

impl TotalRankingDense {
    pub fn new(elements: usize) -> Self {
        TotalRankingDense { orders: Vec::new(), elements, orders_count: 0 }
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    // Check if a given total ranking is valid, i.e.
    // 1. len(orders) = elements * orders_count
    // 2. Every ranking is total
    fn valid(&self) -> bool {
        if self.elements == 0 && (self.orders_count != 0 || !self.orders.is_empty())
            || self.orders.len() != self.orders_count * self.elements
        {
            return false;
        }

        let seen: &mut [bool] = &mut vec![false; self.elements];
        for i in 0..self.orders_count {
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
            self.orders_count += 1;
        }
        debug_assert!(self.valid());
        Ok(())
    }
}

impl Display for TotalRankingDense {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in 0..self.orders_count {
            for j in 0..(self.elements - 1) {
                let v = self.orders[i * self.elements + j];
                write!(f, "{},", v)?;
            }
            let v_last = self.orders[i * self.elements + (self.elements - 1)];
            writeln!(f, "{}", v_last)?;
        }
        Ok(())
    }
}

impl<'a> DenseOrders<'a> for TotalRankingDense {
    type Order = &'a [usize];
    fn elements(&self) -> usize {
        self.elements
    }

    fn add(&mut self, v: Self::Order) -> Result<(), &'static str> {
        if v.len() != self.elements {
            return Err("Order must contains all elements");
        }
        self.orders.try_reserve(self.elements).or(Err("Could not add order"))?;
        for c in v {
            self.orders.push(*c);
        }
        self.orders_count += 1;
        Ok(())
    }

    fn remove_element(&mut self, target: usize) -> Result<(), &'static str> {
        let targets = &[target];
        if targets.is_empty() {
            return Ok(());
        }
        debug_assert!(pairwise_lt(targets));
        let new_elements = self.elements - targets.len();
        for i in 0..self.orders_count {
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
        self.orders.truncate(self.orders_count * new_elements);
        self.elements = new_elements;
        debug_assert!(self.valid());
        Ok(())
    }

    fn to_partial_ranking(self) -> TiedOrdersIncomplete {
        unimplemented!();
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
        self.orders_count += new_orders;
        debug_assert!(self.valid());
    }
}
