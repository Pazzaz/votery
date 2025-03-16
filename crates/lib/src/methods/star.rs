use std::cmp::Ordering;

use orders::{dense::Cardinal, tied_rank::TiedRank};

use super::VotingMethod;

/// STAR (Score Then Automatic Runoff) voting is a single winner protocol.
/// Ties are resolved according to the "Official Tiebreaker Protocol" described at https://www.starvoting.org/ties
pub struct Star {
    score: TiedRank,
}

// We can break ties by...
// (0) removing those that lost the most matchups
// (1) keeping those that got the most max star ratings,
// (2) removing those that has the most min star ratings,
// (3) choosing randomly (I can include this as an option, but then just don't
// use it for now)
//
// I should make seperate functions for all of these, which will take in a
// &[usize] and &Cardinal and then return a TiedRank. I'm starting to think I
// should just do it very low level and have functions be f: (&mut [usize],
// &Cardinal) -> &mut [usize], so we just modify the orignal slice. This could
// be done by calculating a tiedvote and then reusing it when we need to break
// more ties... wait, that won't work.

/// Rank the candidates according to how many pairwise matchups they won against
/// eachother.
///
/// Higher rank means they won more matchups
fn rank_by_matchups(v: &[usize], data: &Cardinal) -> TiedRank {
    let mut matrix = vec![0; v.len() * v.len()];
    data.fill_preference_matrix(v, &mut matrix);

    let mut matchups_won: Vec<usize> = vec![0; v.len()];
    for i in 0..v.len() {
        for j in (i + 1)..v.len() {
            let vi = matrix[i * v.len() + j];
            let vj = matrix[j * v.len() + i];
            match vi.cmp(&vj) {
                Ordering::Less => matchups_won[j] += 1,
                Ordering::Greater => matchups_won[i] += 1,
                Ordering::Equal => {}
            }
        }
    }
    TiedRank::from_score(data.elements(), v.to_vec(), &mut matchups_won)
}

/// Rank the candidates according to how many they got of a specific rating
///
/// Higher rank means they got the rating more often.
fn rank_by_specific(v: &[usize], data: &Cardinal, rating: usize) -> TiedRank {
    debug_assert!(data.min <= rating && rating <= data.max);

    let mut count: Vec<usize> = vec![0; v.len()];
    for vote in data.iter() {
        for i in 0..v.len() {
            let e = v[i];
            if vote[e] == rating {
                count[i] += 1;
            }
        }
    }
    TiedRank::from_score(data.elements(), v.to_vec(), &mut count)
}

enum TieBreaker {
    Matchups,
    Max,
    Min,
    Random,
}

// The "Official Tiebreaker Protocol" for the scoring round of star voting.
// We tiebreak `ranking` until it is well defined which ones are ranked better
// than `goal_len`. Returns `true` if it manages to tiebreak, else `false`.
fn tiebreak_scoring_official(ranking: &mut TiedRank, goal_len: usize, data: &Cardinal) -> bool {
    let mut tiebreaker = TieBreaker::Matchups;
    loop {
        // We will only tiebreak those that are tied, who would change
        // which candidates are ranked better than `goal_len`.
        let (order_slice, tied_slice) = ranking.top_n_threshold(goal_len);
        let tiebreak_rank = match tiebreaker {
            TieBreaker::Matchups => rank_by_matchups(&order_slice, data),
            TieBreaker::Max => rank_by_specific(&order_slice, data, data.max),
            TieBreaker::Min => {
                let mut r = rank_by_specific(&order_slice, data, data.min);
                r.reverse();
                r
            }
            // We don't handle randomness in this function.
            TieBreaker::Random => return false,
        };

        // TODO: We shouldn't need to copy over things, we should just be able to modify
        // them directly.
        order_slice.copy_from_slice(&tiebreak_rank.order);
        tied_slice.copy_from_slice(&tiebreak_rank.tied);

        let before_len = ranking.len();

        // We "eliminate" candidates which won't win. This affects
        // `TieBreaker::Matchups`.
        ranking.keep_top(goal_len);
        if ranking.len() == goal_len {
            return true;
        }
        let change = ranking.len().cmp(&before_len);

        // We see in this "transition diagram" of `tiebreaker` that this loop
        // is finite, as `change` can only be "Less" a finite number of times.
        tiebreaker = match (change, tiebreaker) {
            (Ordering::Equal, TieBreaker::Matchups) => TieBreaker::Max,
            (Ordering::Equal, TieBreaker::Max) => TieBreaker::Min,
            (Ordering::Equal, TieBreaker::Min) => TieBreaker::Random,
            (Ordering::Equal, TieBreaker::Random) => unreachable!(),
            (Ordering::Less, _) => TieBreaker::Matchups,
            (Ordering::Greater, _) => unreachable!(),
        }
    }
}

// Get a ranking of the candidates sorted by their total score
fn score_ranking(data: &Cardinal) -> TiedRank {
    if data.elements() < 2 {
        return TiedRank::new_tied(data.elements());
    }
    let mut sum = vec![0; data.elements()];
    for vote in data.iter() {
        for i in 0..data.elements() {
            sum[i] += vote[i];
        }
    }
    TiedRank::from_scores(data.elements(), &sum)
}

// Return a comparison between `a` and `b`, a "greater" result means `a` has a
// better rank.
fn runoff_round(a: usize, b: usize, data: &Cardinal) -> Ordering {
    let mut matrix = [0; 4];
    data.fill_preference_matrix(&[a, b], &mut matrix);
    let a_v = matrix[1];
    let b_v = matrix[2];
    a_v.cmp(&b_v)
        .then_with(|| data.compare(a, b))
        .then_with(|| data.compare_specific(a, b, data.max))
}

impl<'a> VotingMethod<'a> for Star {
    type Format = Cardinal;

    fn count(data: &Cardinal) -> Result<Self, &'static str> {
        if data.elements() < 2 {
            return Ok(Star { score: TiedRank::new_tied(data.elements()) });
        }

        // The Scoring Round
        let mut v = score_ranking(data);
        let found_top_two = tiebreak_scoring_official(&mut v, 2, data);

        // We return if the scoring round didn't find top 2.
        if !found_top_two {
            v.make_complete(false);
            return Ok(Star { score: v });
        }
        let a = v.order[0];
        let b = v.order[1];

        // The Runoff Round
        let mut rank = match runoff_round(a, b, data) {
            Ordering::Less => TiedRank::new(data.elements(), vec![b, a], vec![false]),
            Ordering::Equal => TiedRank::new(data.elements(), vec![a, b], vec![true]),
            Ordering::Greater => TiedRank::new(data.elements(), vec![a, b], vec![false]),
        };
        rank.make_complete(false);

        Ok(Star { score: rank })
    }

    fn get_score(&self) -> &Vec<usize> {
        // TODO: fix
        &self.score.order
    }
}

impl Star {
    pub fn as_vote(&self) -> TiedRank {
        self.score.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orders::dense::DenseOrders;

    #[test]
    fn simple_example() {
        let mut votes = Cardinal::new(4,0,4);
        votes.add(&[1, 3, 2, 4]).unwrap();
        votes.add(&[3, 1, 1, 3]).unwrap();
        votes.add(&[0, 2, 1, 2]).unwrap();
        votes.add(&[2, 4, 2, 2]).unwrap();
        // Scoring round should have 1 and 3 as the candidates.
        // Then 3 is preferred on two ballots, tied on one and not preferred on one, so it should win.
        let res = Star::count(&votes).unwrap().as_vote();
        let correct_winner = match res.as_ref().winners() {
            &[win] => win == 3,
            _ => false,
        };
        assert!(correct_winner);
    }
}