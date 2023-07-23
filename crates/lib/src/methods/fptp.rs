use crate::{
    formats::{toi::TiedVote, Specific},
    methods::VotingMethod,
};

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

impl Fptp {
    pub fn as_vote(&self) -> TiedVote {
        let order = self.get_order();
        order_to_vote(&order)
    }
}

pub fn order_to_vote(v: &[usize]) -> TiedVote {
    let mut order = Vec::new();
    let mut tied = Vec::new();
    for i in 0..v.len() {
        let mut found = false;
        for j in 0..v.len() {
            if v[j] == i {
                order.push(j);
                tied.push(true);
                found = true;
            }
        }
        if !found {
            break;
        }
        tied.pop();
        tied.push(false);
    }
    tied.pop();
    debug_assert!(order.len() == v.len());
    TiedVote::new(order, tied)
}
