//! A library to generate [Yee Diagrams][electopedia], which illustrate voting
//! behaviour in a two-dimensional voting space.
//!
//! [electopedia]: https://electowiki.org/wiki/Yee_diagram

use rand::{
    distr::{Uniform, uniform::SampleRange},
    prelude::Distribution,
};
use rayon::{iter::ParallelIterator, prelude::ParallelDrainRange};
pub use votery::generators::gaussian::FuzzyType;
use votery::{
    generators::gaussian::Gaussian,
    methods::{Borda, Fptp, VotingMethod as _},
    orders::tied::TiedI,
};

use crate::{
    candidates::{BouncingCandidates, CandidatesMovement, CandidatesState, OptimizingCandidates},
    color::{Color, DUTCH_FIELD_LEN, VoteColorBlending, blend_colors},
    vector::Vector,
};

pub mod candidates;
pub mod color;

mod vector;

// We only support 2 dimensional images right now
pub const DIMENSIONS: usize = 2;

// Each image is contained in a box [0.0, 1.0] x [0.0, 1.0]
const MIN: f64 = 0.0;
const MAX: f64 = 1.0;

// TODO: Is this correct?
// TODO: Should it be called "DynamicSampling"?
/// Should the sampling procedure be adaptive, meaning we sample more on pixels
/// where the result is unsure
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Adaptive {
    /// Not adaptive
    Disable,

    /// Adaptive
    Enable {
        /// Whether to calculate information about how many samples
        /// were calculated for each pixel
        display: bool,

        /// Noise tolerance threshold, smaller threshold means we'll take more
        /// samples to be sure what a pixel should be
        max_noise: f64,

        /// When dynamically sampling and a pixel is resampled because of noise,
        /// how many of it's neighbours that should be resampled
        around_size: usize,
    },
}

/// How should we blend our samples?
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Blending {
    /// Take the sample that occurs the maximum number of times
    Max,

    /// Take the average of all samples
    Average,
}

// TODO: Should we use struct of arrays or array of structs?
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Candidate {
    pub x: f64,
    pub y: f64,
    pub color: Color,
}

impl Candidate {
    pub fn new_random<R: rand::Rng>(rng: &mut R) -> Self {
        let i = (0..DUTCH_FIELD_LEN).sample_single(rng).unwrap();
        let color = Color::dutch_field(i);
        let dist = Uniform::new_inclusive(MIN, MAX).unwrap();
        let x = dist.sample(rng);
        let y = dist.sample(rng);
        Candidate { x, y, color }
    }
}

/// All parameters used to generate a diagram (may be multiple frames)
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ImageConfig {
    /// Points generated around every pixel, i.e. amount of voters
    pub points: usize,

    /// The pixel width (and height) of the square diagram
    pub resolution: usize,

    /// Timesteps to illustrate
    pub frames: usize,

    /// List of candidates
    pub candidates: Vec<Candidate>,

    /// Samples computed for each pixel, for each round of sampling
    pub sample_size: usize,

    /// Variance when sampling voter positions around a pixel
    pub variance: f64,

    // TODO: Are all of these implemented?
    /// Should the sampling procedure dynamically change how many samples it
    /// uses per pixel
    pub adapt_mode: Adaptive,

    /// Method to blend samples of colors into a single color
    pub blending: Blending,

    /// Method to convert ranking to color
    pub vote_color: VoteColorBlending,

    /// Controls when a voter should rank two candidates as equal
    pub fuzzy: FuzzyType,

    /// The candidates movement over time
    pub candidate_movement: CandidatesMovement,

    /// How each candidate should be drawn in the diagram
    pub draw_candidates: DrawCandidates,

    pub voting_method: VotingMethod,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum VotingMethod {
    Borda,
    Fptp,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum DrawCandidates {
    Disabled,
    // TODO: Is it actually the radius?
    Circle { radius: f64 },
}

impl Default for ImageConfig {
    fn default() -> Self {
        let mut candidates = Vec::new();
        for i in 0..4 {
            let color = Color::dutch_field(i);
            let mut rng = rand::rng();
            let dist = Uniform::new_inclusive(MIN, MAX).unwrap();
            let x = dist.sample(&mut rng);
            let y = dist.sample(&mut rng);
            candidates.push(Candidate { x, y, color });
        }
        ImageConfig {
            points: 1000,
            resolution: 50,
            frames: 1000,
            candidates,
            sample_size: 5,
            variance: 0.2,
            adapt_mode: Adaptive::Enable { display: false, max_noise: 0.5, around_size: 3 },
            blending: Blending::Average,
            vote_color: VoteColorBlending::Harmonic,
            fuzzy: FuzzyType::Scaling(0.4),
            candidate_movement: CandidatesMovement::Optimizing { speed: 0.1 },
            draw_candidates: DrawCandidates::Circle { radius: 0.02 },
            voting_method: VotingMethod::Borda,
        }
    }
}

impl ImageConfig {
    fn candidate_state(&self) -> CandidatesState {
        let candidates: Vec<Vector> =
            self.candidates.iter().map(|c| Vector { x: c.x, y: c.y }).collect();
        match &self.candidate_movement {
            CandidatesMovement::Static => CandidatesState::Static(candidates),
            CandidatesMovement::Bouncing { speed } => {
                // TODO: Choose directions in a better way
                let mut rng = rand::rng();
                let state = BouncingCandidates::new_random_direction(&mut rng, *speed, candidates);
                CandidatesState::Bouncing(state)
            }
            CandidatesMovement::Optimizing { speed } => {
                CandidatesState::Optimizing(OptimizingCandidates::new(candidates, *speed))
            }
        }
    }
}

// We have this big struct to store results from sampling an image, but we
// should use `Option`.
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct SampleResult {
    pub image: Vec<Vec<[u8; 3]>>,
    pub sample_count: Vec<Vec<usize>>,
    pub all_rankings: Vec<Vec<Vec<TiedI>>>,
    pub sample_heatmap: Option<Vec<Vec<[u8; 3]>>>,
    pub candidates: Vec<Vector>,
}

fn get_image(candidates: &[Vector], config: &ImageConfig) -> SampleResult {
    let mut g = Gaussian::new(DIMENSIONS, config.variance, config.points, config.fuzzy);
    for c in candidates {
        g.add_candidate(&c.as_array());
    }
    let mut iterations = 0;
    let mut all_samples: Vec<Vec<Vec<Color>>> =
        vec![vec![Vec::new(); config.resolution]; config.resolution];
    let mut needs_samples = vec![vec![true; config.resolution]; config.resolution];
    let mut queue = Vec::with_capacity(config.resolution * config.resolution);
    let mut sample_count: Vec<Vec<usize>> = vec![vec![0; config.resolution]; config.resolution];
    let mut all_rankings: Vec<Vec<Vec<TiedI>>> =
        vec![vec![Vec::new(); config.resolution]; config.resolution];
    loop {
        iterations += 1;
        // First we'll add every pixel that needs samples to the queue
        queue.clear();
        for yi in 0..config.resolution {
            for xi in 0..config.resolution {
                if needs_samples[yi][xi] {
                    queue.push((xi, yi));
                    needs_samples[yi][xi] = false;
                }
            }
        }
        println!("{}: pixels to sample: {}", iterations, queue.len());
        // Then we actually get some samples
        let new_samples: Vec<(usize, usize, Vec<Color>, Vec<TiedI>)> = queue
            .par_drain(..)
            .map(|(xi, yi)| {
                let mut rng = rand::rng();
                let mut new_samples1 = Vec::with_capacity(config.sample_size);
                let mut new_samples2 = Vec::with_capacity(config.sample_size);
                for _ in 0..config.sample_size {
                    let (color, vote) = sample_pixel(&g, xi, yi, &mut rng, config);
                    new_samples1.push(color);
                    new_samples2.push(vote);
                }
                (xi, yi, new_samples1, new_samples2)
            })
            .collect();
        // Then we need to decide which pixels need more samples. We say that a pixel
        // needs more samples if it hasn't converged, or if any of its neighbours
        // haven't converged yet
        let mut done = true;
        for (xi, yi, new_colors, new_votes) in new_samples {
            all_rankings[yi][xi].extend(new_votes);
            sample_count[yi][xi] += 1;
            let old = &mut all_samples[yi][xi];
            if old.is_empty() || needs_samples[yi][xi] {
                needs_samples[yi][xi] = true;
                old.extend(new_colors);
                done = false;
                continue;
            }
            if let Adaptive::Enable { max_noise, around_size, .. } = config.adapt_mode {
                let more_samples = match config.blending {
                    Blending::Max => {
                        let old_color = most_common(old);
                        old.extend(new_colors);
                        let new_color = most_common(old);
                        old_color != new_color
                    }
                    Blending::Average => {
                        let old_color = blend_colors(old.iter());
                        old.extend(new_colors);
                        let new_color = blend_colors(old.iter());
                        let d = old_color.dist(&new_color);
                        d > max_noise
                    }
                };
                if more_samples {
                    done = false;
                    let max_xi = xi.saturating_add(around_size).min(config.resolution - 1);
                    let min_xi = xi.saturating_sub(around_size);
                    let max_yi = yi.saturating_add(around_size).min(config.resolution - 1);
                    let min_yi = yi.saturating_sub(around_size);
                    for y in min_yi..=max_yi {
                        for x in min_xi..=max_xi {
                            needs_samples[y][x] = true;
                        }
                    }
                }
            };
        }
        if done {
            break;
        }
    }
    let mut image = vec![vec![[0, 0, 0]; config.resolution]; config.resolution];
    for yi in 0..config.resolution {
        for xi in 0..config.resolution {
            image[yi][xi] = blend_colors(all_samples[yi][xi].iter()).quantize();
        }
    }

    let sample_heatmap: Option<Vec<Vec<[u8; 3]>>> = match config.adapt_mode {
        Adaptive::Enable { display: true, .. } => {
            let max_samples = sample_count.iter().map(|c| c.iter().max().unwrap()).max().unwrap();
            let res = sample_count
                .iter()
                .map(|c| c.iter().map(|x| Color::bw(*x, *max_samples).quantize()).collect())
                .collect();
            Some(res)
        }
        Adaptive::Enable { display: false, .. } | Adaptive::Disable => None,
    };

    match config.draw_candidates {
        DrawCandidates::Circle { radius } => {
            for c in &config.candidates {
                add_circle(&mut image, c, config.resolution, radius);
            }
        }
        DrawCandidates::Disabled => {}
    }

    SampleResult {
        image,
        sample_count,
        all_rankings,
        sample_heatmap,
        candidates: candidates.to_vec(),
    }
}

fn most_common<T>(v: &mut [T]) -> T
where
    T: Default + PartialOrd + Clone,
{
    if v.is_empty() {
        return T::default();
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut most_common = None;
    let mut current_count = 0;
    let mut max_count = 0;
    let mut prev = None;
    for o in v.iter() {
        match most_common {
            Some(_) => {
                if prev.unwrap() == o {
                    current_count += 1;
                    if current_count > max_count {
                        max_count = current_count;
                        most_common = Some(o)
                    }
                } else {
                    current_count = 1;
                }
            }
            None => {
                most_common = Some(o);
                current_count = 1;
            }
        }
        prev = Some(o);
    }

    most_common.unwrap().clone()
}

fn add_circle(
    image: &mut Vec<Vec<[u8; 3]>>,
    candidate: &Candidate,
    resolution: usize,
    radius: f64,
) {
    let pi = std::f64::consts::PI;
    let mut angle: f64 = 0.0;
    while angle < 360.0 {
        let mut r_in = 0.0;
        while r_in < radius {
            let x1 = r_in * f64::cos(angle * pi / 180.0);
            let y1 = r_in * f64::sin(angle * pi / 180.0);
            let x = candidate.x + x1;
            let y = candidate.y + y1;
            put_pixel(image, x, y, candidate.color, resolution);
            r_in += 0.001
        }
        angle += 0.1;
    }

    let mut angle: f64 = 0.0;
    while angle < 360.0 {
        let x1 = radius * f64::cos(angle * pi / 180.0);
        let y1 = radius * f64::sin(angle * pi / 180.0);
        let x = candidate.x + x1;
        let y = candidate.y + y1;
        put_pixel(image, x, y, color::BLACK, resolution);
        angle += 0.1;
    }
}

// maps [MIN, MAX) -> [0, RESOLUTION)
fn f64_to_coord(u: f64, resolution: usize) -> usize {
    let s = ((u - MIN) / (MAX - MIN) * resolution as f64) as usize;
    if s >= resolution { resolution - 1 } else { s }
}

fn put_pixel(image: &mut Vec<Vec<[u8; 3]>>, x: f64, y: f64, color: Color, resolution: usize) {
    let xx = f64_to_coord(x, resolution);
    let yy = f64_to_coord(y, resolution);
    image[yy][xx] = color.quantize();
}

fn sample_pixel<R: rand::Rng>(
    g: &Gaussian,
    xi: usize,
    yi: usize,
    rng: &mut R,
    config: &ImageConfig,
) -> (Color, TiedI) {
    let x: f64 = (xi as f64) / (config.resolution as f64) * (MAX - MIN) + MIN;
    let y: f64 = (yi as f64) / (config.resolution as f64) * (MAX - MIN) + MIN;
    let votes = g.sample(rng, &[x, y]).into();
    let vote: TiedI = match config.voting_method {
        VotingMethod::Borda => Borda::count(&votes).unwrap().as_vote(),
        VotingMethod::Fptp => {
            // TODO: Maybe just sample winners directly?
            let winners = votes.to_specific(rng).unwrap();
            Fptp::count(&winners).unwrap().as_vote()
        }
    };
    // TODO: Include method in config
    let color = Color::from_vote(config.vote_color, vote.as_ref(), &config.candidates);
    (color, vote)
}

pub struct Renderer<'a> {
    config: &'a ImageConfig,
    candidates: CandidatesState,
    steps: usize,
}

impl<'a> Renderer<'a> {
    // TODO: Include candidates and colors in config
    pub fn new(config: &'a ImageConfig) -> Self {
        let moving_candidates = config.candidate_state();
        Self { config, candidates: moving_candidates, steps: 0 }
    }
}

impl<'a> Iterator for Renderer<'a> {
    // TODO: We want to return references, to avoid allocation
    type Item = SampleResult;

    fn next(&mut self) -> Option<Self::Item> {
        if self.steps < self.config.frames {
            let mut res = get_image(self.candidates.candidates(), self.config);

            self.candidates.step(&self.config, &mut res);
            self.steps += 1;
            Some(res)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.config.frames - self.steps;
        (size, Some(size))
    }
}

impl<'a> ExactSizeIterator for Renderer<'a> {}
