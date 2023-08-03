//! A spatial model of voting behaviour, where every candidate is a point in
//! some space, and voters vote for nearby candidates.
use std::{
    mem,
    slice::{ChunksExact, ChunksExactMut},
};

use rand_distr::{num_traits::Pow, Distribution, Normal};

use crate::formats::{orders::TiedVote, toc::TiedOrdersComplete};

pub struct Gaussian {
    dimensions: usize,
    candidates: Vec<f64>,
    variance: f64,
    points: usize,
    fuzzy: FuzzyType,
}

/// Decides when two candidates should be tied
#[derive(Clone, Copy)]
pub enum FuzzyType {
    /// There are ties if the distance to two candidates are less than `fuzzy`
    Absolute(f64),
    /// Candidates further away are harder to differentiate, so larger distances
    /// are treated as tied
    Scaling(f64),
    /// There are only ties if two candidates are exactly the same distance away
    Equal,
}

impl Gaussian {
    pub fn new(dimensions: usize, variance: f64, points: usize, fuzzy: FuzzyType) -> Self {
        Gaussian { dimensions, candidates: Vec::new(), variance: variance, points, fuzzy }
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

    pub fn sample<R: rand::Rng>(&self, rng: &mut R, mean: &[f64]) -> TiedOrdersComplete {
        let mut votes = TiedOrdersComplete::new(self.candidates());
        for _ in 0..self.points {
            let point = generate_point(self.dimensions, mean, self.variance, rng);
            let candidate_score: Vec<f64> =
                self.iter_candidates().map(|c| euclidean_dist(&point, c)).collect();

            let vote = score_to_vote(&candidate_score, self.fuzzy);
            votes.add(vote.slice());
        }

        votes
    }
}

fn are_fuzzy(w0: f64, w1: f64, fuzzy: FuzzyType) -> bool {
    match fuzzy {
        FuzzyType::Absolute(f) => (w0 - w1).abs() <= f,
        FuzzyType::Equal => w0 == w1,
        FuzzyType::Scaling(f) => {
            let (x, y) = if w0 < w1 { (w1, w0) } else { (w0, w1) };
            y >= x - (x / ((1.0 - f.powf(0.1)) * 10.0)).powi(2)
        }
    }
}

fn score_to_vote(scores: &[f64], fuzzy: FuzzyType) -> TiedVote {
    let mut list: Vec<(usize, f64)> = scores.iter().cloned().enumerate().collect();
    list.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());
    // TODO: We assume self.dimension = 2 here
    let tied: Vec<bool> = list.windows(2).map(|w| are_fuzzy(w[0].1, w[1].1, fuzzy)).collect();
    let order: Vec<usize> = list.into_iter().map(|(i, _)| i).collect();
    TiedVote::new(order, tied)
}

fn generate_point<R: rand::Rng>(len: usize, mean: &[f64], variance: f64, rng: &mut R) -> Vec<f64> {
    debug_assert!(mean.len() == len);
    (0..len)
        .map(|i| {
            let normal = Normal::new(mean[i], variance).unwrap();
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
