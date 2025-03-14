use orders::formats::Binary;

use super::VotingMethod;

pub struct Approval {
    score: Vec<usize>,
}

impl<'a> VotingMethod<'a> for Approval {
    type Format = Binary;

    fn count(data: &Binary) -> Result<Self, &'static str> {
        debug_assert!(data.votes.len() == data.voters * data.elements());
        let mut score: Vec<usize> = vec![0; data.elements()];
        for i in 0..data.voters {
            for j in 0..data.elements() {
                if data.votes[i * data.elements() + j] {
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
