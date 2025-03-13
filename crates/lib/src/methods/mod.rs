/// Trait shared by every voting method
pub trait VotingMethod<'a> {
    /// Every voting method accepts some specific vote format as input.
    type Format: VoteFormat<'a> + Clone;

    /// Counts all the votes, into a format which makes it fast to compute other
    /// methods such as `get_order`.
    fn count(data: &Self::Format) -> Result<Self, &'static str>
    where
        Self: Sized;

    /// Internal score, e.g. the number of votes for each candidate for methods
    /// like first-past-the-post, but may not make sense for all methods.
    /// Return value should be able to be used by `get_order` to get the
    /// result of the voting method. Larger values are higher rank.
    fn get_score(&self) -> &Vec<usize>;

    /// Gets a partial order of the candidates
    fn get_order(&self) -> Vec<usize> {
        get_order(self.get_score(), true)
    }
}

/// A version of `VotingMethod`, but randomness can be used when calculating the
/// winner
pub trait RandomVotingMethod<'a> {
    /// Every voting method accepts some specific vote format as input.
    type Format: VoteFormat<'a> + Clone;

    /// Counts all the votes, into a format which makes it fast to compute other
    /// methods such as `get_order`. Uses `rng` to perform random decisions.
    /// `positions` may be used to somplify the method if we only care about the
    /// top `positions`.
    fn count<R>(data: &Self::Format, rng: &mut R, positions: usize) -> Result<Self, &'static str>
    where
        R: Rng,
        Self: Sized;

    /// Internal score, e.g. the number of votes for each candidate for methods
    /// like first-past-the-post, but may not make sense for all methods.
    /// Return value should be able to be used by `get_order` to get the
    /// result of the voting method. Larger values are higher rank.
    fn get_score(&self) -> &Vec<usize>;

    /// Gets a partial order of the candidates
    fn get_order(&self) -> Vec<usize> {
        get_order(self.get_score(), true)
    }
}

// Convert a list of numbers to the partial order of the list. High numbers in
// input list will get high numbers in new list, but can be changed using
// `reverse`. We do not clone the original list.
pub fn get_order<T: Ord>(v: &[T], reverse: bool) -> Vec<usize> {
    if v.is_empty() {
        return Vec::new();
    } else if v.len() == 1 {
        return vec![0];
    }

    let mut tmp: Vec<(usize, &T)> = Vec::with_capacity(v.len());
    for i in 0..v.len() {
        tmp.push((i, &v[i]));
    }
    tmp.sort_by(|a, b| (*a.1).cmp(b.1));
    if reverse {
        tmp.reverse();
    }
    let mut out = vec![0; v.len()];
    if let Some((b, bs)) = tmp.split_first_mut() {
        let mut current: &T = b.1;
        let mut i: usize = 0;
        for x in bs.iter_mut() {
            if *x.1 != *current {
                current = x.1;
                i += 1;
            }
            out[x.0] = i;
        }
    }
    out
}

// TODO: This method makes no sense
// Returns
//     Ordering::Less    if i is ranked better than j
//     Ordering::Equal   if they are ranked equally
//     Ordering::Greater if i is ranked worse than j
// pub fn pairwise_comparison<'a, M, F>(mut v: F, i: usize, j: usize) ->
// Result<Ordering, &'static str> where
//     F: VoteFormat<'a> + Clone,
//     M: VotingMethod<'a, Format = F>,
// {
//     let c = v.candidates();
//     debug_assert!(i < c && j < c);
//     if i == j {
//         return Ok(Ordering::Equal);
//     }
//     let remove: Vec<usize> = (0..c).filter(|&x| x != i && x != j).collect();
//     v.remove_candidates(&remove)?;
//     debug_assert!(v.candidates() == 2);
//     let order = M::count(&v)?.get_order();
//     debug_assert!(order.len() == 2);
//     let o = order[0].cmp(&order[1]);
//     if i > j {
//         Ok(o.reverse())
//     } else {
//         Ok(o)
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_order_ordered() {
        let a: Vec<usize> = (0..20).into_iter().collect();
        let mut b = a.clone();
        b.reverse();
        assert_eq!(get_order(&a, true), b);
    }

    #[test]
    fn get_order_single() {
        let a = vec![6425654];
        let b = vec![0];
        assert_eq!(get_order(&a, true), b);
    }

    #[test]
    fn get_order_empty() {
        let a: Vec<usize> = vec![];
        let b = vec![];
        assert_eq!(get_order(&a, true), b);
    }

    #[test]
    fn get_order_ties() {
        let a: Vec<usize> = vec![43, 5, 5, 12, 5, 10, 12, 0, 60, 4];
        let b = vec![1, 4, 4, 2, 4, 3, 2, 6, 0, 5];
        assert_eq!(get_order(&a, true), b);
    }

    #[quickcheck]
    fn qc_get_order_involution(xs: Vec<usize>) -> bool {
        let a = get_order(&xs, true);
        let b = get_order(&a, true);
        let c = get_order(&b, true);
        a == c
    }

    #[quickcheck]
    fn qc_get_order_idempotent(xs: Vec<usize>) -> bool {
        let a = get_order(&xs, false);
        let b = get_order(&a, false);
        a == b
    }

    #[quickcheck]
    fn qc_get_order_basic(xs: Vec<usize>, reverse: bool) -> bool {
        let a = get_order(&xs, reverse);
        let mut ys = xs.clone();
        ys.sort();
        ys.dedup();
        if reverse {
            ys.reverse();
        }
        for i in 0..xs.len() {
            let value = xs[i];
            let order = a[i];
            let correct_order = ys.iter().position(|&x| x == value).unwrap();
            if order != correct_order {
                return false;
            }
        }
        true
    }
}

mod approval;
pub use approval::Approval;
mod borda;
pub use borda::Borda;
mod fptp;
pub use fptp::Fptp;
pub mod random_ballot;
use orders::formats::VoteFormat;
use rand::Rng;
mod star;
pub use star::Star;
