// There are several different types of borda count. We have tried to handle
// every variation. See also the Dowdall system, a similar method.

use super::fptp::order_to_vote;
use crate::{
    formats::{orders::TiedRank, toi::TiedOrdersIncomplete, VoteFormat},
    methods::VotingMethod,
};

pub struct Borda {
    score: Vec<usize>,
}

impl<'a> VotingMethod<'a> for Borda {
    type Format = TiedOrdersIncomplete;

    fn count(data: &TiedOrdersIncomplete) -> Result<Self, &'static str> {
        let n = data.candidates();
        let mut score: Vec<usize> = vec![0; n];
        for vote in data {
            // println!("{:?}", &vote);
            let mut seen = 0;
            for group in vote.iter_groups() {
                let ties = group.len();
                // TODO: Is this correct?
                debug_assert!(n >= (seen + ties));
                let ranked_below = n - (seen + ties);
                for &c in group {
                    // Add one point for every candidate `c` is preferred to, and a half point for
                    // every other one `c` is tied with. We don't want to store 0.5 so everything is
                    // multiplied by 2.
                    score[c] += 2 * ranked_below + ties;
                }
                seen += ties;
            }
        }
        Ok(Borda { score })
    }

    fn get_score(&self) -> &Vec<usize> {
        &self.score
    }
}

impl Borda {
    pub fn as_vote(&self) -> TiedRank {
        let order = self.get_order();
        order_to_vote(&order)
    }
}
