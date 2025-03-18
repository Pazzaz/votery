use rand::{
    distr::{Bernoulli, Distribution, Uniform},
    seq::SliceRandom,
};

use super::TiedOrdersComplete;
use crate::{
    DenseOrders,
    cardinal::CardinalDense,
    strict::StrictIDense,
    tied_rank::{TiedRank, TiedRankRef},
};

/// TOI - Orders with Ties - Incomplete List
///
/// A packed list of (possibly incomplete) orders with ties, with related
/// methods. One can see it as a `Vec<TiedRank>`, but more efficient.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TiedOrdersIncomplete {
    // Has length count * elements
    pub(crate) orders: Vec<usize>,

    // Says if a value is tied with the next value.
    // Has length count * (elements - 1)
    pub(crate) ties: Vec<bool>,

    // Where each order ends
    pub(crate) order_end: Vec<usize>,
    pub(crate) elements: usize,
}

impl TiedOrdersIncomplete {
    pub fn new(elements: usize) -> Self {
        TiedOrdersIncomplete {
            orders: Vec::new(),
            ties: Vec::new(),
            order_end: Vec::new(),
            elements,
        }
    }

    pub fn from_parts(
        orders: Vec<usize>,
        ties: Vec<bool>,
        order_end: Vec<usize>,
        elements: usize,
    ) -> Self {
        let count = if elements == 0 {
            0
        } else {
            assert!(orders.len() % elements == 0);
            orders.len() / elements
        };
        assert!(ties.len() == count * elements.saturating_sub(1));
        Self { orders, ties, order_end, elements }
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    pub fn get(&self, i: usize) -> TiedRankRef {
        assert!(i < self.count());
        let start = if i == 0 { 0 } else { self.order_end[i - 1] };
        let end = self.order_end[i];
        TiedRankRef::new(
            self.elements,
            &self.orders[start..end],
            &self.ties[(start - i)..(end - i - 1)],
        )
    }

    pub fn iter(&self) -> impl Iterator<Item = TiedRankRef> {
        (0..self.count()).map(|i| self.get(i))
    }

    pub fn count(&self) -> usize {
        self.order_end.len()
    }

    /// Add a single order from a string. Return true if it was a valid order.
    pub fn add_from_str(&mut self, s: &str) -> bool {
        self.add_from_str_i(s, 1)
    }

    /// Add a order from a string, `i` times. Return true if it was a valid
    /// order.
    pub fn add_from_str_i(&mut self, s: &str, i: usize) -> bool {
        debug_assert!(i != 0);
        match TiedRank::parse_order(self.elements, s) {
            Some(order) => {
                for _ in 0..i {
                    self.add(order.as_ref()).unwrap();
                }
                true
            }
            None => false,
        }
    }

    /// Returns true if this struct is in a valid state, used for debugging.
    #[cfg(test)]
    pub(crate) fn valid(&self) -> bool {
        let mut orders_len = 0;
        let mut ties_len = 0;
        for v in self.iter() {
            let len = v.len();
            if len == 0 {
                return false;
            }
            orders_len += len;
            ties_len += len - 1;
        }
        if orders_len != self.orders.len() || ties_len != self.ties.len() {
            return false;
        }
        let mut seen = vec![false; self.elements];
        for order in self.iter() {
            seen.fill(false);
            for &i in order.order() {
                if i >= self.elements || seen[i] {
                    return false;
                }
                seen[i] = true;
            }
        }
        true
    }

    // Increase the number of elements to `n`. Panics if `n < self.elements`
    pub fn set_elements(&mut self, n: usize) {
        debug_assert!(n >= self.elements);
        self.elements = n;
    }

    /// If an order ranks element `n`, then add a tie with a new element,
    /// as if the new element was a clone of `n`.
    pub fn add_clone(&mut self, n: usize) {
        let c = self.elements;
        let mut new = TiedOrdersIncomplete::new(c + 1);
        for order in self.iter() {
            let mut new_order: Vec<usize> = order.order().to_vec();
            let mut tied: Vec<bool> = order.tied().to_vec();
            if let Some(i) = new_order.iter().position(|&x| x == n) {
                new_order.insert(i, c);
                tied.insert(i, true);
            };
            let yeah = TiedRank::new(c + 1, new_order, tied);
            new.add(yeah.as_ref()).unwrap();
        }
        *self = new;
    }

    // Returns all elements who more than 50% of orders has ranked as their
    // highest alternative. If multiple elements are tied as their highest
    // alternative, then they all count, so multiple elements can be the
    // majority.
    pub fn majority(&self) -> Vec<usize> {
        if self.elements == 1 {
            return vec![0];
        }
        let mut firsts = vec![0; self.elements];
        for order in self.iter() {
            for &c in order.winners() {
                firsts[c] += 1;
            }
        }
        firsts
            .into_iter()
            .enumerate()
            .filter(|(_, score)| *score > self.count() / 2)
            .map(|(i, _)| i)
            .collect()
    }

    /// TODO: NOT SAME AS MAJORITY
    /// Same as `majority`, but contains a list of elements to ignore. Useful
    /// for methods like "Instant-runoff voting". Assumes `ignore is sorted`,
    /// and then does binary searches to find if a element should be ignored.
    pub fn majority_ignore(&self, ignore: &[usize]) -> Vec<usize> {
        if self.elements == 1 {
            return vec![0];
        }
        let mut firsts = vec![0; self.elements];
        for order in self.iter() {
            for group in order.iter_groups() {
                let mut found = false;
                for c in group {
                    if ignore.binary_search(c).is_err() {
                        // We found a element which isn't ignored. We'll iterate through all its
                        // ties, and then break.
                        firsts[*c] += 1;
                        found = true;
                    }
                }
                if found {
                    break;
                }
            }
        }
        firsts
    }

    /// Check if a set of elements is a set of clones such that there does not
    /// exists a element outside the set with ranking i, and two elements in
    /// the set with ranking n and m, where n <= i <= m.
    pub fn is_clone_set(&self, clones: &[usize]) -> bool {
        if clones.len() < 2 {
            return true;
        }
        let mut is_clone = vec![false; self.elements];
        for &c in clones {
            debug_assert!(c < self.elements);
            is_clone[c] = true;
        }
        for order in self.iter() {
            let mut seen_n = false;
            let mut seen_i = false;
            for group in order.iter_groups() {
                // We first check what's in the current group
                let mut has_clone = false;
                let mut has_normal = false;

                // Note that we do not do anything special when all of {n, i, m} are in the same
                // group. We just treat it as if we've encountered n and i.
                for &c in group {
                    if is_clone[c] {
                        has_clone = true;
                    } else {
                        has_normal = true;
                    }
                }
                if seen_i && has_clone || (seen_n && has_clone && has_normal) {
                    // We found "n <= i <= m" in the order
                    return false;
                }
                if has_clone {
                    seen_n = true;
                }
                if seen_n && has_normal {
                    seen_i = true;
                }
            }
        }
        true
    }

    pub fn to_cardinal(self) -> Result<CardinalDense, &'static str> {
        let mut v = TiedRank::new_tied(self.elements);
        let mut cardinal_rank = vec![0; self.elements];
        let max = self.elements - 1;
        let mut cardinal_orders = CardinalDense::new(self.elements, 0..=max);
        for order in self.iter() {
            v.copy_from(order);
            v.make_complete(false);
            v.as_ref().cardinal_high(&mut cardinal_rank, 0, max);
            cardinal_orders.add(&cardinal_rank)?;
            cardinal_rank.fill(0);
        }
        Ok(cardinal_orders)
    }
}

impl<'a> DenseOrders<'a> for TiedOrdersIncomplete {
    type Order = TiedRankRef<'a>;
    /// List the number of elements
    fn elements(&self) -> usize {
        self.elements
    }

    fn add(&mut self, order: TiedRankRef) -> Result<(), &'static str> {
        assert!(order.elements() == self.elements);
        assert!(!order.is_empty());
        self.orders.reserve(order.len());
        self.ties.reserve(order.len() - 1);
        self.order_end.reserve(1);

        self.orders.extend_from_slice(order.order());
        self.ties.extend_from_slice(order.tied());
        let start = self.order_end.last().unwrap_or(&0);
        self.order_end.push(*start + order.len());
        Ok(())
    }

    /// Remove the element with index `n`, and shift indices of elements
    /// with higher index. May remove orders if they only contain `n`.
    fn remove_element(&mut self, n: usize) -> Result<(), &'static str> {
        let new_elements = self.elements - 1;
        let mut new = TiedOrdersIncomplete::new(new_elements);
        for order in self.iter() {
            let mut new_order: Vec<usize> = Vec::with_capacity(order.order().len() - 1);
            let mut new_tied: Vec<bool> = Vec::with_capacity(order.tied().len().saturating_sub(1));
            for i in 0..new_order.len() {
                let mut v = new_order[i];
                if v == n {
                    continue;
                }
                if v > n {
                    v -= 1;
                }
                new_order.push(v);
                if i != new_tied.len() {
                    new_tied.push(new_tied[i]);
                }
            }
            if new_order.is_empty() {
                continue;
            }
            let out = TiedRank::new(new_elements, new_order, new_tied);
            new.add(out.as_ref())?;
        }
        *self = new;
        Ok(())
    }

    fn generate_uniform<R: rand::Rng>(&mut self, rng: &mut R, new_orders: usize) {
        if self.elements == 0 {
            return;
        }
        let v: &mut [usize] = &mut (0..self.elements).collect::<Vec<usize>>();
        self.orders.reserve(new_orders * self.elements);
        self.ties.reserve(new_orders * (self.elements - 1));
        let dist = Bernoulli::new(0.5).unwrap();
        let range = Uniform::new(0, self.elements).unwrap();
        let mut new_end = 0;
        for _ in 0..new_orders {
            let elements = range.sample(rng) + 1;
            v.shuffle(rng);
            self.orders.extend_from_slice(&v[..elements]);
            for _ in 0..(elements - 1) {
                self.ties.push(dist.sample(rng));
            }

            new_end += elements;
            self.order_end.push(new_end);
        }
    }
}

impl From<StrictIDense> for TiedOrdersIncomplete {
    fn from(value: StrictIDense) -> Self {
        let orders: usize = value.count();
        let s = TiedOrdersIncomplete::from_parts(
            value.orders,
            vec![false; orders * (value.elements - 1)],
            value.order_end,
            value.elements,
        );
        s
    }
}

impl From<TiedOrdersComplete> for TiedOrdersIncomplete {
    fn from(value: TiedOrdersComplete) -> Self {
        let orders: usize = value.orders();
        let order_end = (0..value.count()).map(|i| (i + 1) * value.elements()).collect();
        let s = TiedOrdersIncomplete::from_parts(
            value.orders,
            vec![false; orders * (value.elements - 1)],
            order_end,
            value.elements,
        );
        s
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;
    use crate::tests::std_rng;

    impl Arbitrary for TiedOrdersIncomplete {
        fn arbitrary(g: &mut Gen) -> Self {
            let (mut orders_count, mut elements): (usize, usize) = Arbitrary::arbitrary(g);

            // `Arbitrary` for numbers will generate "problematic" examples such as
            // `usize::max_value()` and `usize::min_value()` but we'll use them to
            // allocate vectors so we'll limit them.
            orders_count = orders_count % g.size();
            elements = elements % g.size();

            let mut orders = TiedOrdersIncomplete::new(elements);
            orders.generate_uniform(&mut std_rng(g), orders_count);
            orders
        }
    }

    #[quickcheck]
    fn arbitrary(orders: TiedOrdersIncomplete) -> bool {
        orders.valid()
    }

    #[quickcheck]
    fn clone_remove(orders: TiedOrdersIncomplete, i: usize) -> bool {
        let mut orders = orders.clone();
        let c = orders.elements;
        if c == 0 {
            return true;
        }
        orders.add_clone(i % c);
        orders.remove_element(c).is_ok()
    }
}
