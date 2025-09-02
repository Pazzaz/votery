use std::{fs::File, io::BufWriter, path::Path};

use rayon::{
    iter::ParallelIterator,
    prelude::{IntoParallelIterator, ParallelDrainRange},
};

use png::Writer;
use rand::{distributions::Uniform, prelude::Distribution, thread_rng, Rng};
use votery::{generators::gaussian::{FuzzyType, Gaussian}, methods::{Borda, VotingMethod}, orders::tied::TiedI};

use crate::{candidates::OptimizingCandidates, color::{blend_colors, Color, VoteColorBlending}};

pub mod candidates;
pub mod color;

mod vector;

// We only support 2 dimensional images right now
pub const DIMENSIONS: usize = 2;

// Each image is contained in a box [0.0, 1.0] x [0.0, 1.0]
const MIN: f64 = 0.0;
const MAX: f64 = 1.0;

#[derive(PartialEq, Eq)]
enum Adaptive {
    Disable,
    Enable,
    Display,
}

enum Blending {
    Max,
    Average,
}

pub struct ImageConfig {
    points: usize,
    pub resolution: usize,
    pub frames: usize,
    pub candidates: usize,
    sample_size: usize,
    max_noise: f64,
    variance: f64,
    adapt_mode: Adaptive,
    around_size: usize,
    blending: Blending,
    vote_color: VoteColorBlending,
    fuzzy: FuzzyType,
}

impl Default for ImageConfig {
    fn default() -> Self {
        ImageConfig {
            points: 1000,
            resolution: 50,
            frames: 1000,
            candidates: 4,
            sample_size: 5,
            max_noise: 0.5,
            variance: 0.2,
            adapt_mode: Adaptive::Enable,
            around_size: 3,
            blending: Blending::Average,
            vote_color: VoteColorBlending::Harmonic,
            fuzzy: FuzzyType::Scaling(0.4),
        }
    }
}

// We have this big struct to store results from sampling an image, but we
// should use `Option`.
pub struct SampleResult {
    pub image: Vec<Vec<[u8; 3]>>,
    pub sample_count: Vec<Vec<usize>>,
    pub all_rankings: Vec<Vec<Vec<TiedI>>>,
}

// TODO: This should return the image and all calculated votes (if they are
// needed for other parts later)
pub fn render_image(
    name: &str,
    candidates: &[[f64; 2]],
    colors: &[Color],
    config: &ImageConfig,
) -> SampleResult {
    debug_assert!(candidates.len() == config.candidates);
    // Output file
    // TODO: This shouldn't be part of the library
    let mut writer = create_png_writer(&format!("{}.png", name), config.resolution);
    let writer_adaptive: Option<_> = if config.adapt_mode == Adaptive::Display {
        Some(create_png_writer(&format!("{}_bw.png", name), config.resolution))
    } else {
        None
    };

    debug_assert!(colors.len() == config.candidates);
    let SampleResult { mut image, sample_count, all_rankings } =
        get_image(candidates, colors, config);
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

fn create_png_writer(filename: &str, resolution: usize) -> Writer<BufWriter<File>> {
    println!("{}", filename);
    let path = Path::new(filename);
    let file = File::create(path).unwrap();
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, resolution as u32, resolution as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap()
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

fn sample_pixel<R: Rng>(
    g: &Gaussian,
    xi: usize,
    yi: usize,
    rng: &mut R,
    colors: &[Color],
    config: &ImageConfig,
) -> (Color, TiedI) {
    let x: f64 = (xi as f64) / (config.resolution as f64) * (MAX - MIN) + MIN;
    let y: f64 = (yi as f64) / (config.resolution as f64) * (MAX - MIN) + MIN;
    let votes = g.sample(rng, &[x, y]).into();
    let vote: TiedI = Borda::count(&votes).unwrap().as_vote();
    let color = Color::from_vote(config.vote_color, vote.as_ref(), colors);
    (color, vote)
}

pub fn random_candidates<R: Rng>(rng: &mut R, n: usize) -> Vec<[f64; DIMENSIONS]> {
    let dist = Uniform::new_inclusive(0.0, 1.0);
    (0..n).into_iter().map(|_| {
        let mut d = [0.0; DIMENSIONS];
        for i in 0..DIMENSIONS {
            d[i] = dist.sample(rng);
        }
        d
    }).collect()
}

pub fn render_animation(
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
        moving_candidates.step(v.as_ref());
        println!("{:?}", moving_candidates.candidates);
    }
}