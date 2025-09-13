use yee::{random_candidates, render_animation, ImageConfig};

fn main() {
    let config = ImageConfig::default();
    let candidates = random_candidates(&mut rand::rng(), config.candidates);
    let mut directions = Vec::new();
    for [x, y] in &candidates {
        directions.push([y / 100.0, x / 100.0]);
    }
    render_animation(candidates, &config);
}
