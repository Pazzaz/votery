use crate::{formats::Specific, methods::VotingMethod};

pub struct Fptp {
    score: Vec<usize>,
}

impl<'a> VotingMethod<'a> for Fptp {
    type Format = Specific;

    fn count(data: &Specific) -> Result<Self, &'static str> {
        let mut score: Vec<usize> = vec![0; data.candidates];
        for vote in &data.votes {
            debug_assert!(*vote < data.candidates);
            score[*vote] = score[*vote]
                .checked_add(1)
                .ok_or("Integer overflow: Too many votes for same candidate")?;
        }
        Ok(Fptp { score })
    }

    fn get_score(&self) -> &Vec<usize> {
        &self.score
    }
}
