use rand::{Rng, distr::Uniform, prelude::Distribution};
use votery::orders::tied::TiedIRef;

use crate::{ImageConfig, MAX, MIN, SampleResult, most_common, vector::Vector};

/// Decides how candidates should act over time, used for configuration
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum CandidatesMovement {
    /// Their positions are static, doesn't change
    Static,

    /// Their positions bounce around the state space
    ///
    /// Parameter is the speed of the candidates
    Bouncing { speed: f64 },

    // TODO: Is this description correct?
    /// Each candidate optimizes their position independently to improve their
    /// ranking
    ///
    /// They move towards candidates with a better ranking, and move away from
    /// candidates with a worse ranking
    ///
    /// Parameter is the speed of the candidates
    Optimizing { speed: f64 },
}

/// Each candidate's state, used during computation
pub enum CandidatesState {
    Static(Vec<Vector>),
    Bouncing(BouncingCandidates),
    Optimizing(OptimizingCandidates),
}

impl CandidatesState {
    pub fn candidates(&self) -> &[Vector] {
        match self {
            CandidatesState::Static(candidates) => candidates,
            CandidatesState::Bouncing(s) => &s.candidates,
            CandidatesState::Optimizing(s) => &s.candidates,
        }
    }

    // res is taken as mutable just to be able to sort the list of rankings
    pub fn step(&mut self, config: &ImageConfig, res: &mut SampleResult) {
        match self {
            // Static candidates don't change
            CandidatesState::Static(_) => {}

            CandidatesState::Bouncing(s) => s.step(),
            CandidatesState::Optimizing(s) => {
                // TODO: Why do we use the middle samples for this?
                let x = config.resolution / 2;
                let y = config.resolution / 2;
                let v = most_common(&mut res.all_rankings[y][x]);
                s.step(v.as_ref());
            }
        }
    }
}

// A struct to represent a set of candidates which "bounce around" in the yee
// diagram.
pub struct BouncingCandidates {
    pub candidates: Vec<Vector>,
    pub directions: Vec<Vector>,
}

impl BouncingCandidates {
    pub fn new(candidates: Vec<Vector>, directions: Vec<Vector>) -> Self {
        debug_assert!(candidates.len() == directions.len());
        BouncingCandidates { candidates, directions }
    }

    // Create a new `BouncingCandidates` where each direction has been chosen
    // randomly. All candidates will move at the same `speed`.
    pub fn new_random_direction<R: Rng>(rng: &mut R, speed: f64, candidates: Vec<Vector>) -> Self {
        let circle_uniform = Uniform::new(0f64, std::f64::consts::TAU).unwrap();
        let directions: Vec<Vector> = candidates
            .iter()
            .map(|_| {
                let v = circle_uniform.sample(rng);
                let (x, y) = v.sin_cos();
                Vector { x: x * speed, y: y * speed }
            })
            .collect();
        BouncingCandidates::new(candidates, directions)
    }

    fn len(&self) -> usize {
        self.candidates.len()
    }

    pub fn step(&mut self) {
        for j in 0..self.len() {
            let new = Vector::add(&self.candidates[j], &self.directions[j]);
            let new_x = new.x;
            let new_y = new.y;
            if new_x < 0.0 {
                self.candidates[j].x = 0.0;
                self.directions[j].x = -self.directions[j].x;
            } else if new_x > 1.0 {
                self.candidates[j].x = 1.0;
                self.directions[j].x = -self.directions[j].x;
            } else {
                self.candidates[j].x = new_x;
            }

            if new_y < 0.0 {
                self.candidates[j].y = 0.0;
                self.directions[j].y = -self.directions[j].y;
            } else if new_y > 1.0 {
                self.candidates[j].y = 1.0;
                self.directions[j].y = -self.directions[j].y;
            } else {
                self.candidates[j].y = new_y;
            }
        }
    }
}

pub struct OptimizingCandidates {
    pub candidates: Vec<Vector>,
    speed: f64,
}

impl OptimizingCandidates {
    pub fn new(candidates: Vec<Vector>, speed: f64) -> Self {
        debug_assert!(0.0 < speed && speed <= 1.0);
        OptimizingCandidates { candidates, speed }
    }

    fn len(&self) -> usize {
        self.candidates.len()
    }

    pub fn step(&mut self, ranking: TiedIRef) {
        let old = &self.candidates;
        let mut new_candidates: Vec<Vector> = Vec::with_capacity(self.len());
        for c1 in 0..self.candidates.len() {
            let v1 = old[c1];
            let mut dv = Vector { x: 0.0, y: 0.0 };
            let mut before = true;
            for group in ranking.iter_groups() {
                if group.contains(&c1) {
                    // We don't move towards candidates with the same ranking
                    before = false;
                    continue;
                }
                for c2 in group {
                    let v2 = old[*c2];

                    // This is the vector from c2 to c1.
                    let v3: Vector = v1.sub(&v2);
                    // Max distance: sqrt(MAX + MAX), min distance: 0. When the distance
                    // between them is MAX, then we don't want to push them away
                    // from each other at all. When they are right next to each
                    // other, we want to push them a lot but not an insane
                    // amount.
                    let dv_c2 = if before {
                        // Move towards c2.
                        v3.scaled(-self.speed)
                    } else {
                        // Move away from v2
                        // One interesting way to do this would be to say that "max" would be
                        // calculated using v3, so it's in some direction
                        // The question is: find sx1 such that v1.x + v3.x * sx1 == 0.0 and sx2 such
                        // that v1.x + v3.x * sx2 == 1.0 and then the same for sy1 and sy2.
                        // We then take the min of them all to find the maximum multiple we could
                        // move. Then we multiply it with speed to find how long to move :)
                        let sx1 = (MIN - v1.x) / v3.x;
                        let sx2 = (MAX - v1.x) / v3.x;
                        let sy1 = (MIN - v1.y) / v3.y;
                        let sy2 = (MAX - v1.y) / v3.y;
                        let max_mul = [sx1, sx2, sy1, sy2]
                            .into_iter()
                            .filter(|x| *x >= 0.0)
                            .fold(f64::NAN, |a, b| a.min(b));
                        if max_mul.is_nan() {
                            continue;
                        }
                        v3.scaled(max_mul * self.speed)
                    };
                    dv.add_assign(&dv_c2);
                }
            }
            dv.div_assign_s(self.len() as f64);
            let new_c1 = v1.add(&dv).clamp(MIN, MAX);
            new_candidates.push(new_c1);
        }
        self.candidates = new_candidates;
    }
}
