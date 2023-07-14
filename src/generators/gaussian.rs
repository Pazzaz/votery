//! A spatial model of voting behaviour, where every candidate is a point in
//! some space, and voters vote for nearby candidates.
use std::slice::{ChunksExact, ChunksExactMut};

use rand_distr::{Distribution, Normal};

use crate::formats::toc::TiedOrdersComplete;

pub struct Gaussian {
    dimensions: usize,
    candidates: Vec<f64>,
    mean: Vec<f64>,
    variance: Vec<f64>,
    points: usize,
}

impl Gaussian {
    pub fn new(dimensions: usize, mean: &[f64], variance: &[f64], points: usize) -> Self {
        Gaussian {
            dimensions,
            candidates: Vec::new(),
            mean: mean.to_vec(),
            variance: variance.to_vec(),
            points,
        }
    }

    pub fn candidates(&self) -> usize {
        debug_assert!(self.candidates.len() % self.dimensions == 0);
        self.candidates.len() / self.dimensions
    }

    pub fn add_candidate(&mut self, candidate: &[f64]) {
        debug_assert!(candidate.len() == self.dimensions);
        self.candidates.extend(candidate);
    }

    pub fn iter_candidates(&self) -> ChunksExact<f64> {
        self.candidates.chunks_exact(self.dimensions)
    }

    pub fn iter_candidates_mut(&mut self) -> ChunksExactMut<f64> {
        self.candidates.chunks_exact_mut(self.dimensions)
    }

    pub fn sample<R: rand::Rng>(&self, rng: &mut R, n: usize) -> TiedOrdersComplete {
        let mut votes = TiedOrdersComplete::new(self.candidates());
        for _ in 0..n {
            // First we generate a distribution of points centered around the mean
            let points: Vec<Vec<f64>> = (0..self.points)
                .map(|_| generate_point(self.dimensions, &self.mean, &self.variance, rng))
                .collect();

            // Then we go through every candidate and check their score compared to the
            // distribution
            let candidate_score: Vec<f64> = self
                .iter_candidates()
                .map(|c| {
                    points.iter().map(|x| euclidean_dist(c, x)).sum::<f64>() / points.len() as f64
                })
                .collect();

            // Then we create a vote using this score, lower is better
            let vote = sort_indices(&candidate_score);

            // Then we add the score.
            votes.add(&vote.0, &vote.1);
        }
        votes
    }
}

fn sort_indices(scores: &[f64]) -> (Vec<usize>, Vec<bool>) {
    debug_assert!(scores.len() != 0);
    let mut list: Vec<(usize, f64)> = scores.iter().cloned().enumerate().collect();
    list.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());
    let ties: Vec<bool> = list.windows(2).map(|w| w[0].1 == w[1].1).collect();
    let order: Vec<usize> = list.into_iter().map(|(i, _)| i).collect();
    debug_assert!(ties.len() + 1 == order.len());
    (order, ties)
}

fn generate_point<R: rand::Rng>(
    len: usize,
    mean: &[f64],
    variance: &[f64],
    rng: &mut R,
) -> Vec<f64> {
    debug_assert!(mean.len() == len && variance.len() == len);
    (0..len)
        .map(|i| {
            let normal = Normal::new(mean[i], variance[i]).unwrap();
            normal.sample(rng)
        })
        .collect()
}

fn euclidean_dist(a: &[f64], b: &[f64]) -> f64 {
    debug_assert!(a.len() == b.len());
    let mut sum = 0.0;
    for (&a, &b) in a.iter().zip(b) {
        sum += (a - b) * (a - b)
    }
    sum.sqrt()
}
