use rand::{
    distr::{Distribution, Uniform},
    seq::{IndexedRandom, SliceRandom},
};

use super::{Tied, TiedDense};
use crate::{
    DenseOrders, add_bool,
    cardinal::{CardinalDense, CardinalRef},
    specific::SpecificDense,
    strict::ChainDense,
    tied::{TiedI, TiedIRef},
};

/// TOI - Orders with Ties - Incomplete List
///
/// A packed list of (possibly incomplete) orders with ties, with related
/// methods. One can see it as a `Vec<TiedRank>`, but more efficient.
#[derive(Debug, PartialEq, Eq)]
pub struct TiedIDense {
    // Has length count * elements
    pub(crate) orders: Vec<usize>,

    // Says if a value is tied with the next value.
    // Has length count * (elements - 1)
    pub(crate) ties: Vec<bool>,

    // Where each order ends
    pub(crate) order_end: Vec<usize>,
    pub(crate) elements: usize,
}

impl Clone for TiedIDense {
    fn clone(&self) -> Self {
        Self {
            orders: self.orders.clone(),
            ties: self.ties.clone(),
            order_end: self.order_end.clone(),
            elements: self.elements,
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.orders.clone_from(&source.orders);
        self.ties.clone_from(&source.ties);
        self.order_end.clone_from(&source.order_end);
        self.elements = source.elements;
    }
}

impl TiedIDense {
    pub fn new(elements: usize) -> Self {
        TiedIDense { orders: Vec::new(), ties: Vec::new(), order_end: Vec::new(), elements }
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
            assert!(orders.len().is_multiple_of(elements));
            orders.len() / elements
        };
        assert!(ties.len() == count * elements.saturating_sub(1));
        Self { orders, ties, order_end, elements }
    }

    pub fn elements(&self) -> usize {
        self.elements
    }

    pub fn iter(&self) -> impl Iterator<Item = TiedIRef<'_>> {
        (0..self.len()).map(|i| self.get(i))
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
        let mut new = TiedIDense::new(c + 1);
        for order in self.iter() {
            let mut new_order: Vec<usize> = order.order().to_vec();
            let mut tied: Vec<bool> = order.tied().to_vec();
            if let Some(i) = new_order.iter().position(|&x| x == n) {
                new_order.insert(i, c);
                tied.insert(i, true);
            };
            let yeah = TiedI::new(c + 1, new_order, tied);
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
            .filter(|(_, score)| *score > self.len() / 2)
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
        let mut v: TiedI = Tied::new_tied(self.elements).into();
        let mut cardinal_rank = vec![0; self.elements];
        let max = self.elements - 1;
        let mut cardinal_orders = CardinalDense::new(self.elements, 0..=max);
        for order in self.iter() {
            v.clone_from_ref(order);
            v = v.make_complete(false).into();
            v.as_ref().cardinal_high(&mut cardinal_rank, 0, max);
            cardinal_orders.add(CardinalRef::new(&cardinal_rank))?;
            cardinal_rank.fill(0);
        }
        Ok(cardinal_orders)
    }

    // TODO: Could be inplace
    pub fn to_specific<R: rand::Rng>(self, rng: &mut R) -> Result<SpecificDense, &'static str> {
        // TODO: Add with_capacity
        let mut out = SpecificDense::new(self.elements);
        for order in self.iter() {
            let winners = order.winners();
            let winner = winners.choose(rng).unwrap();
            out.add(*winner).unwrap();
        }
        Ok(out)
    }
}

impl<'a> DenseOrders<'a> for TiedIDense {
    type Order = TiedIRef<'a>;
    /// List the number of elements
    fn elements(&self) -> usize {
        self.elements
    }

    fn len(&self) -> usize {
        self.order_end.len()
    }

    fn try_get(&'a self, i: usize) -> Option<TiedIRef<'a>> {
        if i < self.len() {
            let start = if i == 0 { 0 } else { self.order_end[i - 1] };
            let end = self.order_end[i];
            Some(TiedIRef::new(
                self.elements,
                &self.orders[start..end],
                &self.ties[(start - i)..(end - i - 1)],
            ))
        } else {
            None
        }
    }

    fn add(&mut self, order: TiedIRef) -> Result<(), &'static str> {
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
        let mut new = TiedIDense::new(new_elements);
        let mut tmp = TiedI::new_zero();
        for order in self.iter() {
            tmp.clone_from_ref(order);

            tmp = tmp.remove(n);
            if !tmp.is_empty() {
                new.add(tmp.as_ref())?;
            }
        }
        *self = new;
        Ok(())
    }

    fn generate_uniform<R: rand::Rng>(&mut self, rng: &mut R, new_orders: usize) {
        assert!(self.elements != 0 || new_orders == 0);
        if self.elements == 0 || new_orders == 0 {
            return;
        }
        let v: &mut [usize] = &mut (0..self.elements).collect::<Vec<usize>>();
        self.orders.reserve(new_orders * self.elements);
        self.ties.reserve(new_orders * (self.elements - 1));
        self.order_end.reserve(new_orders);
        let range = Uniform::new(0, self.elements).unwrap();
        let mut new_end = 0;
        for _ in 0..new_orders {
            let elements = range.sample(rng) + 1;
            v.shuffle(rng);
            self.orders.extend_from_slice(&v[..elements]);

            new_end += elements;
            self.order_end.push(new_end);
        }
        let tied_count = new_end - new_orders;
        add_bool(rng, &mut self.ties, tied_count);
    }
}

impl From<ChainDense> for TiedIDense {
    fn from(value: ChainDense) -> Self {
        let orders: usize = value.len();
        TiedIDense::from_parts(
            value.orders,
            vec![false; orders * (value.elements - 1)],
            value.order_end,
            value.elements,
        )
    }
}

impl From<TiedDense> for TiedIDense {
    fn from(value: TiedDense) -> Self {
        let orders: usize = value.len();
        let order_end = (0..value.len()).map(|i| (i + 1) * value.elements()).collect();
        TiedIDense::from_parts(
            value.orders,
            vec![false; orders * (value.elements - 1)],
            order_end,
            value.elements,
        )
    }
}

impl<'a> FromIterator<TiedIRef<'a>> for TiedIDense {
    /// # Panics
    ///
    /// Panics if any orders have different numbers of elements.
    fn from_iter<T: IntoIterator<Item = TiedIRef<'a>>>(iter: T) -> Self {
        let mut ii = iter.into_iter();
        if let Some(first_v) = ii.next() {
            let elements = first_v.elements();
            let mut new = TiedIDense::new(elements);
            new.add(first_v).unwrap();
            for v in ii {
                assert!(v.elements() == elements);
                new.add(v).unwrap();
            }
            new
        } else {
            TiedIDense::new(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};
    use rand::SeedableRng;
    use rand_chacha::ChaCha12Rng;
    use test::Bencher;

    use super::*;
    use crate::tests::std_rng;

    impl Arbitrary for TiedIDense {
        fn arbitrary(g: &mut Gen) -> Self {
            let (mut orders_count, mut elements): (usize, usize) = Arbitrary::arbitrary(g);

            // `Arbitrary` for numbers will generate "problematic" examples such as
            // `usize::max_value()` and `usize::min_value()` but we'll use them to
            // allocate vectors so we'll limit them.
            elements = elements % g.size();
            orders_count = if elements != 0 { orders_count % g.size() } else { 0 };

            let mut orders = TiedIDense::new(elements);
            orders.generate_uniform(&mut std_rng(g), orders_count);
            orders
        }
    }

    #[quickcheck]
    fn arbitrary(orders: TiedIDense) -> bool {
        orders.valid()
    }

    #[quickcheck]
    fn remove(orders: TiedIDense, n: usize) -> bool {
        let old_elements = orders.elements();
        if old_elements == 0 {
            return true;
        }
        let n = n % old_elements;
        let mut a = orders;
        let b: Vec<TiedI> = a.iter().map(|x| x.owned().remove(n)).collect();
        a.remove_element(n).unwrap();
        let mut res: TiedIDense =
            b.iter().filter_map(|x| if x.is_empty() { None } else { Some(x.as_ref()) }).collect();
        res.set_elements(old_elements - 1);
        a == res
    }

    // These three benches compare different ways to do "generate_uniform".
    #[bench]
    fn bench_add_random(b: &mut Bencher) {
        let rng = ChaCha12Rng::from_seed([1; 32]);
        b.iter(|| {
            let mut rng = rng.clone();
            let mut d = TiedIDense::new(10);
            d.generate_uniform(&mut rng, 1000);
        });
    }

    // See above
    #[bench]
    fn bench_add_random_iter(b: &mut Bencher) {
        const ELEMENTS: usize = 10;
        let rng = ChaCha12Rng::from_seed([1; 32]);
        b.iter(|| {
            let mut rng = rng.clone();
            let mut d = TiedIDense::new(ELEMENTS);
            for _ in 0..1000 {
                let t: TiedI = TiedI::random(&mut rng, ELEMENTS);
                if t.is_empty() {
                    continue;
                }
                d.add(t.as_ref()).unwrap();
            }
        });
    }

    // See above
    #[bench]
    fn bench_add_random_iter_2(b: &mut Bencher) {
        const ELEMENTS: usize = 10;
        let rng = ChaCha12Rng::from_seed([1; 32]);
        b.iter(|| {
            let mut rng = rng.clone();
            let mut d = TiedIDense::new(ELEMENTS);
            let mut tmp = TiedI::new_zero();
            for _ in 0..1000 {
                tmp.into_random(&mut rng, ELEMENTS);
                if tmp.is_empty() {
                    continue;
                }
                d.add(tmp.as_ref()).unwrap();
            }
        });
    }
}
