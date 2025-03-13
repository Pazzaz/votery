//! Different orders of candidates
//!
//! There are two main types of orders:
//! - [`Rank`] - An order of candidates without ties, where earlier elements are
//!   ranked higher. There are also reference versions which don't own the data:
//!   [`RankRef`]
//! - [`TiedRank`] - An order of candidates with ties,  where earlier elements
//!   are ranked higher and where some candidates can be tied with others. There
//!   are also reference versions which don't own the data: [`TiedRankRef`].

use std::fmt::{self, Write};

use rand::{
    seq::{IteratorRandom, SliceRandom},
    Rng,
};
use rand_distr::{Bernoulli, Uniform};

// A vote without any ties
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rank {
    candidates: usize,
    order: Vec<usize>,
}

// A vote without any ties
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RankRef<'a> {
    pub(crate) candidates: usize,
    pub(crate) order: &'a [usize],
}

impl Rank {
    pub fn new(candidates: usize, order: Vec<usize>) -> Self {
        debug_assert!(unique(&order));
        Rank { candidates, order }
    }

    pub fn parse_vote(candidates: usize, s: &str) -> Option<Self> {
        let mut order: Vec<usize> = Vec::with_capacity(candidates);
        for number in s.split(',') {
            let n: usize = match number.parse() {
                Ok(n) => n,
                Err(_) => return None,
            };
            if !(n < candidates) {
                return None;
            }
            order.push(n);
        }

        Some(Rank::new(candidates, order))
    }

    pub fn as_ref(&self) -> RankRef {
        RankRef { candidates: self.candidates, order: &self.order[..] }
    }
}

impl<'a> RankRef<'a> {
    pub fn new(candidates: usize, order: &'a [usize]) -> Self {
        debug_assert!(unique(order));
        RankRef { candidates, order }
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn top(&self, n: usize) -> Self {
        RankRef::new(self.candidates, &self.order[0..n])
    }

    pub fn to_owned(&self) -> Rank {
        Rank::new(self.candidates, self.order.to_vec())
    }

    pub fn winner(&self) -> usize {
        debug_assert!(self.order.len() != 0);
        self.order[0]
    }

    pub fn to_tied(self, tied: &'a [bool]) -> TiedRankRef<'a> {
        TiedRankRef::new(self.candidates, self.order, tied)
    }
}

/// A vote with possible ties.
#[derive(Clone, Debug, PartialEq, Eq, Default, PartialOrd)]
pub struct TiedRank {
    pub order: Vec<usize>,
    pub tied: Vec<bool>,
    pub candidates: usize,
}

impl<'a> TiedRank {
    /// A tiedvote is created using
    pub fn new(candidates: usize, order: Vec<usize>, tied: Vec<bool>) -> Self {
        debug_assert!(tied.len() + 1 == order.len() || tied.len() == 0 && order.len() == 0);
        TiedRank { candidates, order, tied }
    }

    pub fn new_tied_from_slice(candidates: usize, order: &[usize]) -> Self {
        let tie_len = order.len().saturating_sub(1);
        let tied = vec![true; tie_len];
        TiedRank::new(candidates, order.to_vec(), tied)
    }

    pub fn as_ref(&'a self) -> TiedRankRef<'a> {
        TiedRankRef::new(self.candidates, &self.order[..], &self.tied[..])
    }

    /// Return the number of ranked candidates.
    ///
    /// ```
    /// use votery::formats::orders::TiedRank;
    ///
    /// let empty = TiedRank::new_zero();
    /// assert!(empty.len() == 0);
    ///
    /// let full = TiedRank::new_tied(10);
    /// assert!(full.len() == 10);
    /// ```
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Become a copy of `rank`, useful to reuse allocations.
    pub fn copy_from(&mut self, rank: TiedRankRef) {
        self.order.clear();
        self.order.extend_from_slice(rank.order());
        self.tied.clear();
        self.tied.extend_from_slice(rank.tied());
        // TODO: Do we really want to do this?
        self.candidates = rank.candidates;
    }

    /// Create a new ranking of `candidates`, where every candidate is tied.
    ///
    /// ```
    /// use votery::formats::orders::TiedRank;
    ///
    /// let c = 10;
    /// let rank = TiedRank::new_tied(c);
    /// assert!(rank.as_ref().winners().len() == c);
    /// ```
    pub fn new_tied(candidates: usize) -> Self {
        if candidates == 0 {
            return TiedRank::new(0, Vec::new(), Vec::new());
        }
        let mut order = Vec::with_capacity(candidates);
        for i in 0..candidates {
            order.push(i);
        }
        let tied = vec![true; candidates - 1];
        TiedRank::new(candidates, order, tied)
    }

    pub fn increase_candidates(&mut self, candidates: usize) {
        debug_assert!(self.candidates <= candidates);
        self.candidates = candidates;
    }

    /// Try to parse a ranking of `candidates` from `s`. Returns None if `s` is
    /// not a valid ranking.
    ///
    /// ```
    /// use votery::formats::orders::TiedRank;
    ///
    /// let vote_str = "2,{0,1},4";
    /// let vote = TiedRank::parse_vote(5, vote_str).expect("Parse failed");
    /// assert_eq!(format!("{}", vote.as_ref()), vote_str);
    /// ```
    ///
    /// There can be multiple string representations for the same ranking, This
    /// means that `f`, the function from valid string representations of
    /// rankings to actual rankings, is not injective. Example:
    /// ```
    /// use votery::formats::orders::TiedRank;
    ///
    /// let rank = TiedRank::parse_vote(5, "0,{1}").unwrap();
    /// assert!(rank.as_ref().to_string() == "0,1");
    /// ```
    pub fn parse_vote(candidates: usize, s: &str) -> Option<Self> {
        if s == "" {
            let mut rank = TiedRank::new_zero();
            rank.increase_candidates(candidates);
            return Some(rank);
        }
        let l = (s.len() / 2).min(candidates);
        let mut order: Vec<usize> = Vec::with_capacity(l);
        let mut tied: Vec<bool> = Vec::with_capacity(l);
        let mut grouped = false;
        for mut part in s.split(',') {
            // Are we starting a group?
            if !grouped {
                part = part.strip_prefix('{').map_or(part, |s| {
                    grouped = true;
                    s
                });
            }

            // Are we ending a group? We check both cases as this part may be a group with
            // only one element.
            if grouped {
                part = part.strip_suffix('}').map_or(part, |s| {
                    grouped = !grouped;
                    s
                })
            }
            let n: usize = match part.parse() {
                Ok(n) => n,
                Err(_) => return None,
            };
            if !(n < candidates) {
                return None;
            }
            order.push(n);
            tied.push(grouped);
        }
        // The last one will never be tied, so we'll ignore it.
        tied.pop();

        // We didn't end our group
        if grouped {
            return None;
        }
        Some(TiedRank::new(candidates, order, tied))
    }

    pub fn single(candidates: usize, n: usize) -> TiedRank {
        debug_assert!(n < candidates);
        let order = vec![n];
        let tied = Vec::new();
        TiedRank::new(candidates, order, tied)
    }

    /// Given a score to every candidate, create a new TiedRank of those candidates. Higher score is better.
    pub fn from_scores(candidates: usize, v: &[usize]) -> TiedRank {
        debug_assert!(v.len() == candidates);
        let mut list: Vec<(usize, usize)> = v.iter().cloned().enumerate().collect();
        list.sort_by(|(_, a), (_, b)| a.cmp(b).reverse());
        let tied: Vec<bool> = list.windows(2).map(|w| w[0].1 == w[1].1).collect();
        let order: Vec<usize> = list.into_iter().map(|(i, _)| i).collect();
        TiedRank::new(candidates, order, tied)
    }

    /// Make the vote into a ranking which ranks all `candidates`. Use
    /// `tied_last` to decide if the newly added candidates should be tied
    /// with the last ranking candidate in the vote.
    pub fn make_complete(&mut self, tied_last: bool) {
        let empty_first = self.len() == 0;
        if self.order.len() == self.candidates {
            // It's already complete
            return;
        }
        self.order.reserve_exact(self.candidates);
        self.tied.reserve_exact(self.candidates - 1);
        let mut seen = vec![false; self.candidates];
        for &i in &self.order {
            debug_assert!(!seen[i]);
            seen[i] = true;
        }
        for i in 0..self.candidates {
            if !seen[i] {
                self.order.push(i);
            }
        }
        if !empty_first {
            self.tied.push(tied_last);
        }
        self.tied.resize(self.candidates - 1, true)
    }

    pub fn from_score(candidates: usize, mut order: Vec<usize>, score: &mut [usize]) -> TiedRank {
        let l = order.len();
        debug_assert!(l != 0);
        sort_using(&mut order, score);
        let mut tied = Vec::with_capacity(l - 1);
        for i in 0..(l - 1) {
            tied.push(order[i] == order[i + 1]);
        }
        TiedRank::new(candidates, order, tied)
    }

    /// Reverses the ranking in place.
    pub fn reverse(&mut self) {
        self.order.reverse();
        self.tied.reverse();
    }

    /// Remove every candidate from the ranking which had the highest ranking
    pub fn remove_winners(&mut self) {
        let l = self.order.len();
        if l == 0 {
            return;
        }
        let mut winners = 1;
        for &b in &self.tied {
            if b {
                winners += 1;
            }
        }
        self.order.copy_within(winners..l, 0);
        self.order.truncate(l - winners);
        if l == winners {
            self.tied.clear();
        } else {
            self.tied.copy_within(winners..(l - 1), 0);
            self.tied.truncate(l - 1 - winners);
        }
    }

    /// Remove every candidate from the ranking which had the lowest ranking
    pub fn remove_last(&mut self) {
        let l = self.order.len();
        if l == 0 {
            return;
        }
        let mut losers = 1;
        for b in self.tied.iter().rev() {
            if *b {
                losers += 1;
            } else {
                break;
            }
        }
        self.order.truncate(l - losers);
        if l == losers {
            self.tied.clear();
        } else {
            self.tied.truncate(l - 1 - losers);
        }
    }

    /// Create a ranking of zero candidates
    pub fn new_zero() -> Self {
        TiedRank::new(0, Vec::new(), Vec::new())
    }

    /// Generate a random tied ranking of `candidates`.
    pub fn random<R: Rng>(rng: &mut R, candidates: usize) -> Self {
        if candidates == 0 {
            return TiedRank::new_zero();
        }
        let order_len = rng.sample(Uniform::new(0, candidates));
        let mut order = (0..candidates).into_iter().choose_multiple(rng, order_len);
        order.shuffle(rng);
        let tied_len = order_len.saturating_sub(1);
        let mut tied = Vec::with_capacity(tied_len);
        let d = Bernoulli::new(0.5).unwrap();
        for _ in 0..tied_len {
            tied.push(rng.sample(&d));
        }
        TiedRank::new(candidates, order, tied)
    }

    /// Normalize the inner representation of `self`, i.e. sorting the tied
    /// groups.
    ///
    /// ```
    /// use votery::formats::orders::TiedRank;
    ///
    /// let a = TiedRank::parse_vote(3, "{0,1,2}").unwrap();
    /// let mut b = TiedRank::parse_vote(3, "{2,1,0}").unwrap();
    /// assert!(a != b);
    /// b.normalize();
    /// assert!(a == b);
    /// ```
    pub fn normalize(&mut self) {
        let max = self.len();
        if max < 2 {
            return;
        }
        let mut start = 0;
        while start < max {
            let mut end = start + 1;
            for b in &self.tied[start..] {
                if *b {
                    end += 1;
                } else {
                    break;
                }
            }
            // We sort the group but not `self.tied`, because all of these are tied.
            let group = &mut self.order[start..end];
            group.sort();
            start = end;
        }
    }

    pub fn keep_top(&mut self, n: usize) {
        if n == 0 {
            self.order.clear();
            self.tied.clear();
            return;
        }
        debug_assert!(n <= self.len());
        let mut i = n;
        while i < self.order.len() {
            if self.tied[i - 1] {
                i += 1;
            } else {
                break;
            }
        }
        self.order.truncate(i);
        self.tied.truncate(i - 1);
    }

    /// Return the group which is on the threshold of being top `n`.
    /// If the ties would be broken, then we would have a top `n`.
    /// Will return empty lists if top `n` is already decided.
    pub fn top_n_threshold(&mut self, n: usize) -> (&mut [usize], &mut [bool]) {
        if n == 0 {
            return (&mut [], &mut []);
        }
        let mut i = n;
        while i < self.order.len() {
            if self.tied[i - 1] {
                i += 1;
            } else {
                break;
            }
        }
        (&mut self.order[(n - 1)..i], &mut self.tied[(n - 1)..(i - 1)])
    }

    pub fn random_total<R: Rng>(rng: &mut R, candidates: usize, order: &[usize]) -> TiedRank {
        let mut v = order.to_vec();
        v.shuffle(rng);
        let tied_len = v.len().saturating_sub(1);
        let tied = vec![false; tied_len];
        TiedRank::new(candidates, v, tied)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TiedRankRef<'a> {
    /// The total number of candidates this ranking concerns, some of them may
    /// not actually be part of the ranking.
    pub candidates: usize,

    order: &'a [usize],
    tied: &'a [bool],
}

impl<'a> fmt::Display for TiedRankRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut left = self.len();
        for group in self.iter_groups() {
            left -= group.len();
            let grouped = group.len() > 1;
            let (last, aa) = group.split_last().unwrap();
            if grouped {
                f.write_char('{')?;
            }
            for a in aa {
                write!(f, "{},", a)?;
            }
            write!(f, "{}", last)?;
            if grouped {
                f.write_char('}')?;
            }
            if left != 0 {
                f.write_char(',')?;
            }
        }
        Ok(())
    }
}

impl<'a> TiedRankRef<'a> {
    pub fn new(candidates: usize, order: &'a [usize], tied: &'a [bool]) -> Self {
        debug_assert!(tied.len() + 1 == order.len() || order.len() == 0 && tied.len() == 0);
        debug_assert!(unique(order));
        for i in order {
            debug_assert!(*i < candidates);
        }
        TiedRankRef { candidates, order, tied }
    }

    // TODO: Which ones of these...
    pub fn cardinal_uniform(&self, c: &mut [usize], min: usize, max: usize) {
        debug_assert!(c.len() == self.candidates);
        debug_assert!(min <= max);
        let groups = self.iter_groups().count();
        for (i, group) in self.iter_groups().enumerate() {
            let mapped = (groups - 1 - i) * (max - min) / self.candidates + min;
            for e in group {
                c[*e] = mapped;
            }
        }
    }

    // ...makes sense? Both?
    pub fn cardinal_high(&self, c: &mut [usize], min: usize, max: usize) {
        debug_assert!(c.len() == self.candidates);
        debug_assert!(min <= max);
        for (i, group) in self.iter_groups().enumerate() {
            let mapped = (self.candidates - 1 - i) * (max - min) / self.candidates + min;
            for e in group {
                c[*e] = mapped;
            }
        }
    }

    // We may not want to store whole slice in the future, so use accessor function
    #[inline]
    pub fn order(self: &TiedRankRef<'a>) -> &'a [usize] {
        self.order
    }

    #[inline]
    pub fn tied(self: &TiedRankRef<'a>) -> &'a [bool] {
        self.tied
    }

    pub fn increase_candidates(&mut self, candidates: usize) {
        debug_assert!(self.candidates <= candidates);
        self.candidates = candidates;
    }

    /// Return an empty ranking of zero candidates.
    pub fn new_zero() -> Self {
        TiedRankRef::new(0, &[], &[])
    }

    /// Return an empty ranking of `candidates`.
    pub fn new_zero_c(candidates: usize) -> Self {
        let mut rank = TiedRankRef::new_zero();
        rank.increase_candidates(candidates);
        rank
    }

    /// Return an empty ranking of the same `candidates` as `self`.
    pub fn zeroed(self: &TiedRankRef<'a>) -> TiedRankRef<'a> {
        TiedRankRef::new(self.candidates, &[], &[])
    }

    /// Return a ranking of the top `n` candidates. The ranking will be larger
    /// than `n` if ties prevent us from saying which ones are ranked
    /// higher.
    #[must_use]
    pub fn top(self: &TiedRankRef<'a>, n: usize) -> TiedRankRef<'a> {
        if n == 0 {
            return self.zeroed();
        }
        debug_assert!(n <= self.order.len());
        let mut i = n;
        while i < self.order.len() {
            if self.tied[i - 1] {
                i += 1;
            } else {
                break;
            }
        }
        TiedRankRef {
            candidates: self.candidates,
            order: &self.order[0..i],
            tied: &self.tied[0..(i.saturating_sub(1))],
        }
    }

    pub fn len(&self) -> usize {
        self.order().len()
    }

    pub fn owned(self) -> TiedRank {
        TiedRank::new(self.candidates, self.order().to_vec(), self.tied().to_vec())
    }

    pub fn iter_groups(&self) -> GroupIterator<'a> {
        GroupIterator { vote: *self }
    }

    pub fn group(&self, n: usize) -> Option<&[usize]> {
        self.iter_groups().nth(n)
    }

    /// Returns group of candidate `c`. 0 is highest rank. Takes `O(n)` time
    pub fn group_of(&self, c: usize) -> Option<usize> {
        let mut group = 0;
        for i in 0..self.len() {
            if self.order()[i] == c {
                return Some(group);
            }
            if i != self.len() && !self.tied()[i] {
                group += 1;
            }
        }
        None
    }

    pub fn winners(self: &TiedRankRef<'a>) -> &'a [usize] {
        let i = self.tied().iter().take_while(|x| **x).count();
        &self.order()[0..=i]
    }

    pub fn empty(&self) -> bool {
        self.order().len() == 0
    }

    /// Returns a list of all candidates with the top rank, and a ranking of the
    /// rest
    pub fn split_winner_group(self: &TiedRankRef<'a>) -> (&'a [usize], TiedRankRef<'a>) {
        if self.empty() {
            return (&[], *self);
        }
        let mut values = 1;
        for k in self.tied() {
            if *k {
                values += 1;
            } else {
                break;
            }
        }
        let (out, rest_order, rest_tied): (&[usize], &[usize], &[bool]) = if values == self.len() {
            (self.order, &[], &[])
        } else {
            let (_, rest_tied) = self.tied().split_at(values);
            let (out, rest_order) = self.order().split_at(values);
            (out, rest_order, rest_tied)
        };
        (out, TiedRankRef::new(self.candidates, rest_order, rest_tied))
    }
}

// Splits a vote up into its rankings
pub struct GroupIterator<'a> {
    vote: TiedRankRef<'a>,
}

impl<'a> Iterator for GroupIterator<'a> {
    type Item = &'a [usize];
    fn next(&mut self) -> Option<Self::Item> {
        if self.vote.empty() {
            return None;
        }
        let (group, vote) = self.vote.split_winner_group();
        self.vote = vote;
        debug_assert!(group.len() != 0);
        Some(group)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.vote.empty() {
            // We're done
            (0, Some(0))
        } else {
            // We could have one group if all elements are tied, or one group for each
            // element
            (1, Some(self.vote.len()))
        }
    }
}

// Returns true iff all elements in `l` are different
fn unique<T>(l: &[T]) -> bool
where
    T: std::cmp::PartialEq,
{
    for i in 0..l.len() {
        for j in 0..l.len() {
            if i == j {
                break;
            }
            if l[i] == l[j] {
                return false;
            }
        }
    }
    true
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

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;
    use crate::formats::tests::std_rng;

    impl Arbitrary for TiedRank {
        fn arbitrary(g: &mut Gen) -> Self {
            // Modulo to avoid problematic values
            let candidates = <usize as Arbitrary>::arbitrary(g) % g.size();
            TiedRank::random(&mut std_rng(g), candidates)
        }

        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let x = self.clone();
            let iter = (0..(x.len().saturating_sub(1))).rev().map(move |i| {
                TiedRank::new(
                    x.candidates,
                    x.order[0..i].to_vec(),
                    x.tied[0..(i.saturating_sub(1))].to_vec(),
                )
            });
            Box::new(iter)
        }
    }

    #[quickcheck]
    fn reverse_involution(before: TiedRank) -> bool {
        let mut after = before.clone();
        after.reverse();
        after.reverse();
        before == after
    }

    #[quickcheck]
    fn owned(rank: TiedRank) -> bool {
        rank == rank.as_ref().owned()
    }

    #[test]
    fn iter_groups_zero() {
        let rank = TiedRank::new_zero();
        let first_group = rank.as_ref().iter_groups().next();
        assert!(first_group.is_none());
    }

    #[quickcheck]
    fn iter_groups_len(rank: TiedRank) -> bool {
        let calc_len = rank.as_ref().iter_groups().map(|g| g.len()).sum::<usize>();
        rank.len() == calc_len
    }

    #[quickcheck]
    fn top_len(rank: TiedRank, n: usize) -> bool {
        let values = if rank.len() == 0 { 0 } else { n % rank.len() };
        let l = rank.as_ref().top(values).len();
        values <= l && l <= rank.len()
    }

    #[quickcheck]
    fn make_complete_len(mut rank: TiedRank, tied_last: bool) -> bool {
        rank.make_complete(tied_last);
        rank.len() == rank.candidates
    }

    // We have that rank.to_str.to_rank == rank.
    #[quickcheck]
    fn parse_random(rank: TiedRank) -> bool {
        let new_rank_o = TiedRank::parse_vote(rank.candidates, &format!("{}", rank.as_ref()));
        match new_rank_o {
            Some(new_rank) => rank == new_rank,
            None => false,
        }
    }

    #[test]
    fn top_exact_four() {
        let candidates = 5;
        let x = 4;
        let examples = [
            "0,1,2,3,4",
            "{0,1},2,3,4",
            "0,{1,2},3,4",
            "0,1,{2,3},4",
            "{0,1,2},3,4",
            "0,{1,2,3},4",
            "{0,1,2,3},4",
        ];
        for s in examples {
            let rank = TiedRank::parse_vote(candidates, s).expect("Could not parse");
            assert!(rank.as_ref().top(x).len() == x);
        }
    }

    #[test]
    fn tied_remove_last() {
        let mut r = TiedRank::new_tied(20);
        r.remove_last();
        assert!(r.len() == 0);
    }

    #[quickcheck]
    fn top_idempotent(rank: TiedRank, n: usize) -> bool {
        let values = if rank.len() == 0 { 0 } else { n % rank.len() };
        let first = rank.as_ref().top(values);
        let second = first.top(values);
        first == second
    }

    #[quickcheck]
    fn remove_last_complete(rank: TiedRank) -> bool {
        let mut before = rank.clone();
        before.make_complete(true);
        let mut after = before.clone();
        after.remove_last();
        after.make_complete(false);

        after.normalize();
        before.normalize();

        after == before
    }

    #[quickcheck]
    fn keep_top_n_threshold(mut rank: TiedRank, i: usize) -> bool {
        let n = if rank.len() == 0 { 0 } else { i % rank.len() };
        let (order_group, tied_group) = rank.top_n_threshold(n);
        let o = order_group.to_vec();
        let t = tied_group.to_vec();
        rank.keep_top(n);
        let (new_order_group, new_tied_group) = rank.top_n_threshold(n);

        o == new_order_group && t == new_tied_group
    }

    #[quickcheck]
    fn keep_top_n_len(mut rank: TiedRank, i: usize) -> bool {
        let n = if rank.len() == 0 { 0 } else { i % rank.len() };
        let l1 = rank.len();
        rank.keep_top(n);
        let l2 = rank.len();
        n <= l2 && l2 <= l1
    }

    #[test]
    fn parse_rank_tied_examples() {
        // Arbitrary
        let candidates = 10;

        let examples = [
            ("", true),
            ("1", true),
            ("{1}", true),
            ("{0},{1}", true),
            ("{0},{1}", true),
            (",", false),
            (",,", false),
            (",1", false),
            ("1,", false),
            ("{1", false),
            ("1}", false),
            ("{0,1,{2,3}", false),
            ("{{0}}", false),
            ("{0}}", false),
            ("{0},}", false),
            ("{,{0},}", false),
            (" 1", false),
        ];
        for (s, some) in examples {
            let vote_o = TiedRank::parse_vote(candidates, s);
            match (vote_o, some) {
                (Some(_), true) | (None, false) => {}
                (None, true) => panic!("`{}` could not be parsed", s),
                (Some(vote), false) => panic!("`{}` was parsed to `{}`", s, vote.as_ref()),
            }
        }
    }
}
