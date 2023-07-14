// There are several different types of borda count. We have tried to handle
// every variation. See also the Dowdall system, a similar method.

use crate::{formats::PartialRanking, methods::VotingMethod};

pub struct Borda {
    score: Vec<usize>,
}

impl<'a> VotingMethod<'a> for Borda {
    type Format = PartialRanking;

    fn count(data: &PartialRanking) -> Result<Self, &'static str> {
        let mut score: Vec<usize> = vec![0; data.candidates];
        for v in 0..data.voters {
            for c1 in 0..data.candidates {
                // Each candidate will get a score of 2*x, if it's ranked higher
                // than x candidates, and +1 for every tie.
                let mut c1_score: usize = 0;
                let c1_rank = data.votes[v * data.candidates + c1];
                for c2 in 0..data.candidates {
                    if c1 == c2 {
                        continue;
                    }
                    let c2_rank = data.votes[v * data.candidates + c2];
                    if c1_rank > c2_rank {
                        c1_score = c1_score
                            .checked_add(2)
                            .ok_or("Integer overflow: Too many votes for same candidate")?;
                    } else if c1_rank == c2_rank {
                        c1_score = c1_score
                            .checked_add(1)
                            .ok_or("Integer overflow: Too many votes for same candidate")?;
                    }
                }
                score[c1] = score[c1]
                    .checked_add(c1_score)
                    .ok_or("Integer overflow: Too many votes for same candidate")?;
            }
        }
        Ok(Borda { score })
    }

    fn get_score(&self) -> &Vec<usize> {
        &self.score
    }
}
