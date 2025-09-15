use std::{fs::File, io::BufWriter, path::Path};

use png::Writer;
use yee::{random_candidates, Adaptive, ImageConfig, Renderer};

fn main() {
    let config = ImageConfig::default();
    let candidates = random_candidates(&mut rand::rng(), config.candidates);
    let mut directions = Vec::new();
    for [x, y] in &candidates {
        directions.push([y / 100.0, x / 100.0]);
    }
    render_animation(candidates, &config);
}

// TODO: Just send in the type of candidates
fn render_animation(candidates: Vec<[f64; 2]>, config: &ImageConfig) {
    let renderer = Renderer::new(config, candidates);

    for (step, res) in renderer.enumerate() {
        let name = &format!("animation/slow_borda_{}", step);
        // Output file
        let mut writer = create_png_writer(&format!("{}.png", name), config.resolution);
        let writer_adaptive: Option<_> = match config.adapt_mode {
            Adaptive::Enable { display: true } => {
                Some(create_png_writer(&format!("{}_bw.png", name), config.resolution))
            }
            Adaptive::Disable | Adaptive::Enable { display: false } => None,
        };

        if let Some(adaptive_image) = &res.sample_heatmap {
            let image_bytes: Vec<u8> = adaptive_image.iter().flatten().flatten().copied().collect();
            writer_adaptive.unwrap().write_image_data(&image_bytes).unwrap();
        }
        let image_bytes: Vec<u8> = res.image.iter().flatten().flatten().copied().collect();
        writer.write_image_data(&image_bytes).unwrap();
    }
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
