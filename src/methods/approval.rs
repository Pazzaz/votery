use crate::{formats::Binary, methods::VotingMethod};

pub struct Approval {
    score: Vec<usize>,
}

impl<'a> VotingMethod<'a> for Approval {
    type Format = Binary;

    fn count(data: &Binary) -> Result<Self, &'static str> {
        debug_assert!(data.votes.len() == data.voters * data.candidates);
        let mut score: Vec<usize> = vec![0; data.candidates];
        for i in 0..data.voters {
            for j in 0..data.candidates {
                if data.votes[i * data.candidates + j] {
                    score[j] = score[j]
                        .checked_add(1)
                        .ok_or("Integer overflow: Too many votes for same candidate")?;
                }
            }
        }
        Ok(Approval { score })
    }

    fn get_score(&self) -> &Vec<usize> {
        &self.score
    }
}
