use orders::binary::BinaryDense;
use orders::DenseOrders;

use super::VotingMethod;

pub struct Approval {
    score: Vec<usize>,
}

impl<'a> VotingMethod<'a> for Approval {
    type Format = BinaryDense;

    fn count(data: &BinaryDense) -> Result<Self, &'static str> {
        debug_assert!(data.orders.len() == data.len() * data.elements());
        let mut score: Vec<usize> = vec![0; data.elements()];
        for i in 0..data.len() {
            for j in 0..data.elements() {
                if data.orders[i * data.elements() + j] {
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
