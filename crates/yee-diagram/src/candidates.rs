use rand::{distributions::Uniform, prelude::Distribution, Rng};
use votery::orders::order::TiedRankRef;

use crate::{MAX, MIN, vector::Vector};

// A struct to represent a set of candidates which "bounce around" in the yee
// diagram.
pub struct BouncingCandidates {
    pub candidates: Vec<[f64; 2]>,
    pub directions: Vec<[f64; 2]>,
}

impl BouncingCandidates {
    pub fn new(candidates: Vec<[f64; 2]>, directions: Vec<[f64; 2]>) -> Self {
        debug_assert!(candidates.len() == directions.len());
        BouncingCandidates { candidates, directions }
    }

    // Create a new `BouncingCandidates` where each direction has been chosen
    // randomly. All candidates will move at the same `speed`.
    pub fn new_random_direction<R: Rng>(
        rng: &mut R,
        speed: f64,
        candidates: Vec<[f64; 2]>,
    ) -> Self {
        let circle_uniform = Uniform::new(0f64, std::f64::consts::TAU);
        let directions: Vec<[f64; 2]> = candidates
            .iter()
            .map(|_| {
                let v = circle_uniform.sample(rng);
                let (x, y) = v.sin_cos();
                [x * speed, y * speed]
            })
            .collect();
        BouncingCandidates::new(candidates, directions)
    }

    fn len(&self) -> usize {
        self.candidates.len()
    }

    pub fn step(&mut self) {
        for j in 0..self.len() {
            let [x, y] = self.candidates[j];
            let [dx, dy] = self.directions[j];
            let new_x = x + dx;
            let new_y = y + dy;
            if new_x < 0.0 {
                self.candidates[j][0] = 0.0;
                self.directions[j][0] = -self.directions[j][0];
            } else if new_x > 1.0 {
                self.candidates[j][0] = 1.0;
                self.directions[j][0] = -self.directions[j][0];
            } else {
                self.candidates[j][0] = new_x;
            }

            if new_y < 0.0 {
                self.candidates[j][1] = 0.0;
                self.directions[j][1] = -self.directions[j][1];
            } else if new_y > 1.0 {
                self.candidates[j][1] = 1.0;
                self.directions[j][1] = -self.directions[j][1];
            } else {
                self.candidates[j][1] = new_y;
            }
        }
    }
}

pub struct OptimizingCandidates {
    pub candidates: Vec<[f64; 2]>,
    speed: f64,
}

impl OptimizingCandidates {
    pub fn new(candidates: Vec<[f64; 2]>, speed: f64) -> Self {
        debug_assert!(0.0 < speed && speed <= 1.0);
        OptimizingCandidates { candidates, speed }
    }

    fn len(&self) -> usize {
        self.candidates.len()
    }

    pub fn step(&mut self, ranking: TiedRankRef) {
        let old = &self.candidates;
        let mut new_candidates = Vec::with_capacity(self.len());
        for c1 in 0..self.candidates.len() {
            let v1 = Vector::from_array(old[c1]);
            let mut dv = Vector { x: 0.0, y: 0.0 };
            let mut before = true;
            for group in ranking.iter_groups() {
                if group.contains(&c1) {
                    // We don't move towards candidates with the same ranking
                    before = false;
                    continue;
                }
                for c2 in group {
                    let v2 = Vector::from_array(old[*c2]);

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
            new_candidates.push(new_c1.as_array());
        }
        self.candidates = new_candidates;
    }
}
