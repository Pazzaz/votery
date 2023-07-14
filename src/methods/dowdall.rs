
use crate::formats::total_ranking::TotalRanking;
use crate::methods::VotingMethod;

pub struct Dowdall {
    score: Vec<usize>,
}

impl VotingMethod for Dowdall {
    type Format = TotalRanking;

    fn count(data: &TotalRanking) -> Result<Self, &'static str> {
        let mut score: Vec<usize> = vec![0; data.candidates];
        unimplemented!();
        Ok(Dowdall { score })
    }

    fn get_score(&self) -> &Vec<usize> {
        &self.score
    }
}
