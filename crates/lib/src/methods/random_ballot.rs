use orders::formats::{orders::{Rank, TiedRank}, soi::StrictOrdersIncomplete, toi::TiedOrdersIncomplete};
use rand::{prelude::SliceRandom, Rng};
use rand_distr::Uniform;

use super::{get_order, RandomVotingMethod};

/// Draw random votes until they create a ranking
///
/// It will try to have a unique winner for the top `positions`. It will
/// continue drawing random votes to rank the remaining unranked candidates,
/// until it has a total order of the top `positions`.
pub struct RandomBallot {
    ranking: Rank,
}

impl<'a> RandomVotingMethod<'a> for RandomBallot {
    // TODO: Could this be extended to allow ties? It would be a lot more
    // complicated.
    type Format = StrictOrdersIncomplete;

    fn count<R>(data: &Self::Format, rng: &mut R, positions: usize) -> Result<Self, &'static str>
    where
        R: Rng,
        Self: Sized,
    {
        debug_assert!(data.voters() != 0);
        debug_assert!(positions <= data.elements());
        let mut left = positions;
        let mut order: Vec<usize> = Vec::new();
        let mut values: Vec<usize> = (0..data.voters()).collect();
        values.shuffle(rng);
        'outer: for i in values {
            let vote = data.vote_i(i);
            for v in vote.order {
                let l = order.len();
                // Quadratic, maybe bad
                if !order[0..l].contains(v) {
                    order.push(*v);
                    left -= 1;
                    if left == 0 {
                        break 'outer;
                    }
                }
            }
        }
        Ok(RandomBallot { ranking: Rank::new(data.elements(), order) })
    }

    fn get_score(&self) -> &Vec<usize> {
        unimplemented!()
    }

    fn get_order(&self) -> Vec<usize> {
        get_order(self.get_score(), true)
    }
}

/// Draw a single random vote
pub struct RandomBallotSingle {
    ranking: TiedRank,
}

impl<'a> RandomVotingMethod<'a> for RandomBallotSingle {
    type Format = TiedOrdersIncomplete;

    fn count<R>(data: &Self::Format, rng: &mut R, positions: usize) -> Result<Self, &'static str>
    where
        R: Rng,
        Self: Sized,
    {
        let _ = positions;
        let i: usize = rng.sample(Uniform::new(0, data.voters()));
        let vote = data.vote_i(i);
        Ok(RandomBallotSingle { ranking: vote.owned() })
    }

    fn get_score(&self) -> &Vec<usize> {
        unimplemented!();
    }

    fn get_order(&self) -> Vec<usize> {
        get_order(self.get_score(), true)
    }
}

impl RandomBallotSingle {
    pub fn as_vote(&self) -> TiedRank {
        self.ranking.clone()
    }
}
