use std::{
    fs::File,
    io::BufWriter,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

use color::{blend_colors, blend_colors_weighted, Color};
use png::Writer;
use rand::{distributions::Uniform, prelude::Distribution, seq::SliceRandom, thread_rng, Rng};
use rayon::{
    iter::ParallelIterator,
    prelude::{IntoParallelIterator, ParallelDrainRange},
};
use votery::{
    formats::{
        orders::{TiedVote, TiedVoteRef},
        toi::TiedOrdersIncomplete,
        Specific,
    },
    generators::gaussian::{FuzzyType, Gaussian},
    methods::{
        random_ballot::{RandomBallot, RandomBallotSingle},
        Borda, Fptp, RandomVotingMethod,
    },
    prelude::VotingMethod,
};

mod color;

#[derive(PartialEq, Eq)]
enum Adaptive {
    Disable,
    Enable,
    Display,
}

// We only support 2 dimensional images right now
const DIMENSIONS: usize = 2;

// Each image is contained in a box [0.0, 1.0] x [0.0, 1.0]
const MIN: f64 = 0.0;
const MAX: f64 = 1.0;

struct ImageConfig {
    points: usize,
    resolution: usize,
    frames: usize,
    candidates: usize,
    sample_size: usize,
    max_noise: f64,
    variance: f64,
    adapt_mode: Adaptive,
    around_size: usize,
    blending: Blending,
    vote_color: VoteColor,
    fuzzy: FuzzyType,
}

enum Blending {
    Max,
    Average,
}

#[derive(Clone, Copy)]
enum VoteColor {
    Winners,
    Harmonic,
}

impl Default for ImageConfig {
    fn default() -> Self {
        ImageConfig {
            points: 1000,
            resolution: 50,
            frames: 1000,
            candidates: 4,
            sample_size: 5,
            max_noise: 0.2,
            variance: 0.2,
            adapt_mode: Adaptive::Enable,
            around_size: 3,
            blending: Blending::Average,
            vote_color: VoteColor::Harmonic,
            fuzzy: FuzzyType::Scaling(0.4),
        }
    }
}

fn create_png_writer(filename: &str, resolution: usize) -> Writer<BufWriter<File>> {
    let path = Path::new(filename);
    let file = File::create(path).unwrap();
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, resolution as u32, resolution as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap()
}

// Turn a vote into a color.
fn vote_to_color(vote_color: VoteColor, vote: TiedVoteRef, colors: &[Color]) -> Color {
    match vote_color {
        VoteColor::Harmonic => {
            let mut mixes: Vec<Color> = Vec::new();
            let mut weights: Vec<f64> = Vec::new();
            for (gi, group) in vote.iter_groups().enumerate() {
                let mut hmm = Vec::new();
                for &i in group {
                    debug_assert!(i < colors.len());
                    hmm.push(colors[i]);
                }
                let new_c = blend_colors(hmm.iter());
                mixes.push(new_c);
                weights.push(1.0 / (gi + 1) as f64)
            }
            blend_colors_weighted(mixes.iter(), Some(&weights))
        }
        VoteColor::Winners => {
            let i_colors = vote.winners().iter().map(|&i| &colors[i]);
            blend_colors(i_colors)
        }
    }
}

fn sample_pixel<R: Rng>(
    g: &Gaussian,
    xi: usize,
    yi: usize,
    rng: &mut R,
    colors: &[Color],
    config: &ImageConfig,
) -> (Color, TiedVote) {
    let x: f64 = (xi as f64) / (config.resolution as f64) * (MAX - MIN) + MIN;
    let y: f64 = (yi as f64) / (config.resolution as f64) * (MAX - MIN) + MIN;
    let votes = g.sample(rng, &[x, y]).to_toi().unwrap();
    let vote: TiedVote = Borda::count(&votes).unwrap().as_vote();
    let color = vote_to_color(config.vote_color, vote.slice(), colors);
    (color, vote)
}

fn main() {
    let candidates = vec![[0.3, 0.7], [0.5, 0.84], [0.8, 0.2], [0.8, 0.7]];
    let mut directions = Vec::new();
    for [x, y] in &candidates {
        directions.push([y / 100.0, x / 100.0]);
    }
    let colors: Vec<Color> = vec![
        Color::new(255.0, 0.0, 0.0),
        Color::new(0.0, 255.0, 0.0),
        Color::new(0.0, 0.0, 255.0),
        Color::new(0.0, 0.0, 0.0),
    ];
    let config = ImageConfig::default();
    render_animation(candidates, directions, &colors, &config);
}

// A struct to represent a set of candidates which "bounce around" in the yee
// diagram.
struct BouncingCandidates {
    candidates: Vec<[f64; 2]>,
    directions: Vec<[f64; 2]>,
}

impl BouncingCandidates {
    fn new(candidates: Vec<[f64; 2]>, directions: Vec<[f64; 2]>) -> Self {
        debug_assert!(candidates.len() == directions.len());
        BouncingCandidates { candidates, directions }
    }

    // Create a new `BouncingCandidates` where each direction has been chosen
    // randomly. All candidates will move at the same `speed`.
    fn new_random_direction<R: Rng>(rng: &mut R, speed: f64, candidates: Vec<[f64; 2]>) -> Self {
        let circle_uniform = Uniform::new(0f64, std::f64::consts::TAU);
        let directions: Vec<[f64; 2]> = candidates
            .iter()
            .map(|_| {
                let v = circle_uniform.sample(rng);
                let (x, y) = v.sin_cos();
                [x * speed, y * speed]
            })
            .collect();
        BouncingCandidates::new(directions, candidates)
    }

    fn len(&self) -> usize {
        self.candidates.len()
    }

    fn step(&mut self) {
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

struct OptimizingCandidates {
    candidates: Vec<[f64; 2]>,
    speed: f64,
}

impl OptimizingCandidates {
    fn new(candidates: Vec<[f64; 2]>, speed: f64) -> Self {
        debug_assert!(0.0 < speed && speed <= 1.0);
        OptimizingCandidates { candidates, speed }
    }

    fn len(&self) -> usize {
        self.candidates.len()
    }

    fn step(&mut self, ranking: TiedVoteRef) {
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
                        v3.scaled(max_mul*self.speed)
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

struct Vector {
    x: f64,
    y: f64,
}

impl Vector {
    fn from_array(xy: [f64; 2]) -> Self {
        Vector { x: xy[0], y: xy[1] }
    }

    fn as_array(&self) -> [f64; 2] {
        [self.x, self.y]
    }

    fn sub(&self, b: &Vector) -> Vector {
        Vector { x: self.x - b.x, y: self.y - b.y }
    }

    fn add_assign(&mut self, b: &Vector) {
        self.x += b.x;
        self.y += b.y;
    }

    fn add(&self, b: &Vector) -> Vector {
        Vector {x: self.x + b.x, y: self.y + b.y }
    }

    fn div_assign_s(&mut self, s: f64) {
        self.x /= s;
        self.y /= s;
    }

    fn scaled(&self, s: f64) -> Vector {
        Vector { x: self.x * s, y: self.y * s }
    }

    fn len(&self) -> f64 {
        (self.x.powi(2) + self.y.powi(2)).sqrt()
    }

    fn dist(&self, b: &Vector) -> f64 {
        ((self.x - b.x).powi(2) + (self.y - b.y).powi(2)).sqrt()
    }

    fn clamp(&self, min: f64, max: f64) -> Vector {
        Vector { x: self.x.clamp(min, max), y: self.y.clamp(min, max) }
    }
}

fn render_animation(
    candidates: Vec<[f64; 2]>,
    directions: Vec<[f64; 2]>,
    colors: &[Color],
    config: &ImageConfig,
) {
    let mut moving_candidates = OptimizingCandidates::new(candidates, 0.1);
    for i in 0..config.frames {
        let SampleResult { mut all_rankings, .. } = render_image(
            &format!("animation/slow_borda_{}", i),
            &moving_candidates.candidates,
            colors,
            config,
        );
        let x = config.resolution / 4;
        let y = config.resolution / 2;
        let v = most_common(&mut all_rankings[y][x]);
        println!("{:?}, {:?}", moving_candidates.candidates, v);
        moving_candidates.step(v.slice());
        println!("{:?}", moving_candidates.candidates);
    }
}

// We have this big struct to store results from sampling an image, but we should use `Option`.
struct SampleResult {
    image: Vec<Vec<[u8; 3]>>,
    sample_count: Vec<Vec<usize>>,
    all_rankings: Vec<Vec<Vec<TiedVote>>>,
}

fn get_image(candidates: &[[f64; 2]], colors: &[Color], config: &ImageConfig) -> SampleResult {
    let mut g = Gaussian::new(DIMENSIONS, config.variance, config.points, config.fuzzy);
    for c in candidates {
        assert!(vector(c));
        g.add_candidate(c);
    }
    let mut iterations = 0;
    let mut all_samples: Vec<Vec<Vec<Color>>> =
        vec![vec![Vec::new(); config.resolution]; config.resolution];
    let mut needs_samples = vec![vec![true; config.resolution]; config.resolution];
    let mut queue = Vec::with_capacity(config.resolution * config.resolution);
    let mut sample_count: Vec<Vec<usize>> = vec![vec![0; config.resolution]; config.resolution];
    let mut all_rankings: Vec<Vec<Vec<TiedVote>>> = 
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
        let new_samples: Vec<(usize, usize, Vec<Color>, Vec<TiedVote>)> = queue
            .par_drain(..)
            .map(|(xi, yi)| {
                let mut rng = thread_rng();
                let mut new_samples1 = Vec::with_capacity(config.sample_size);
                let mut new_samples2 = Vec::with_capacity(config.sample_size);
                for _ in 0..config.sample_size {
                    let (color, vote) = sample_pixel(&g, xi, yi, &mut rng, &colors, &config);
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
            if old.len() == 0 || needs_samples[yi][xi] {
                needs_samples[yi][xi] = true;
                old.extend(new_colors);
                done = false;
                continue;
            }
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
                    d > config.max_noise
                }
            };
            if more_samples {
                done = false;
                let max_xi = xi.saturating_add(config.around_size).min(config.resolution - 1);
                let min_xi = xi.saturating_sub(config.around_size);
                let max_yi = yi.saturating_add(config.around_size).min(config.resolution - 1);
                let min_yi = yi.saturating_sub(config.around_size);
                for y in min_yi..=max_yi {
                    for x in min_xi..=max_xi {
                        needs_samples[y][x] = true;
                    }
                }
            }
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
    SampleResult {
        image,
        sample_count,
        all_rankings,
    }
}

// TODO: This should return the image and all calculated votes (if they are needed for other parts later)
fn render_image(name: &str, candidates: &[[f64; 2]], colors: &[Color], config: &ImageConfig) -> SampleResult {
    debug_assert!(candidates.len() == config.candidates);
    // Output file
    let mut writer = create_png_writer(&format!("{}.png", name), config.resolution);
    let writer_adaptive: Option<_> = if config.adapt_mode == Adaptive::Display {
        Some(create_png_writer(&format!("{}_bw.png", name), config.resolution))
    } else {
        None
    };

    debug_assert!(colors.len() == config.candidates);
    let SampleResult { mut image, sample_count, all_rankings } = get_image(candidates, colors, config);
    if config.adapt_mode == Adaptive::Display {
        let max_samples = sample_count.iter().map(|c| c.iter().max().unwrap()).max().unwrap();
        let adaptive_image: Vec<Vec<[u8; 3]>> = sample_count
            .iter()
            .map(|c| c.iter().map(|x| Color::bw(*x, *max_samples).quantize()).collect())
            .collect();
        let image_bytes: Vec<u8> = adaptive_image.iter().flatten().flatten().copied().collect();
        writer_adaptive.unwrap().write_image_data(&image_bytes).unwrap();
    }
    for c in 0..config.candidates {
        add_circle(&mut image, colors[c], &candidates[c], config.resolution);
    }
    let image_bytes: Vec<u8> = image.iter().flatten().flatten().copied().collect();
    writer.write_image_data(&image_bytes).unwrap();
    SampleResult { image, sample_count, all_rankings }
}

fn most_common<T>(v: &mut Vec<T>) -> T
where
    T: Default + PartialOrd + Clone,
{
    if v.len() == 0 {
        return T::default();
    }
    v.sort_by(|a, b| a.partial_cmp(&b).unwrap());
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
    color: Color,
    pos: &[f64; DIMENSIONS],
    resolution: usize,
) {
    let r = 0.02;
    let pi = std::f64::consts::PI;
    let mut angle: f64 = 0.0;
    while angle < 360.0 {
        let mut r_in = 0.0;
        while r_in < r {
            let x1 = r_in * f64::cos(angle * pi / 180.0);
            let y1 = r_in * f64::sin(angle * pi / 180.0);
            let x = pos[0] + x1;
            let y = pos[1] + y1;
            put_pixel(image, x, y, color, resolution);
            r_in += 0.001
        }
        angle += 0.1;
    }

    let mut angle: f64 = 0.0;
    while angle < 360.0 {
        let x1 = r * f64::cos(angle * pi / 180.0);
        let y1 = r * f64::sin(angle * pi / 180.0);
        let x = pos[0] + x1;
        let y = pos[1] + y1;
        put_pixel(image, x, y, color::BLACK, resolution);
        angle += 0.1;
    }
}

// maps [MIN, MAX) -> [0, RESOLUTION)
fn f64_to_coord(u: f64, resolution: usize) -> usize {
    let s = ((u - MIN) / (MAX - MIN) * resolution as f64) as usize;
    if s >= resolution {
        resolution - 1
    } else {
        s
    }
}

fn put_pixel(image: &mut Vec<Vec<[u8; 3]>>, x: f64, y: f64, color: Color, resolution: usize) {
    let xx = f64_to_coord(x, resolution);
    let yy = f64_to_coord(y, resolution);
    image[yy][xx] = color.quantize();
}

// void DrawCircle(int x, int y, int r, int color)
// {
//       static const double PI = 3.1415926535;
//       double i, angle, x1, y1;

//       for(i = 0; i < 360; i += 0.1)
//       {
//             angle = i;
//             x1 = r * cos(angle * PI / 180);
//             y1 = r * sin(angle * PI / 180);
//             putpixel(x + x1, y + y1, color);
//       }
// }

fn vector(n: &[f64]) -> bool {
    if n.len() != DIMENSIONS {
        return false;
    }
    for &i in n {
        if i < MIN || MAX < i {
            return false;
        }
    }
    true
}
