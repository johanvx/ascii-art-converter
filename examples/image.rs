use image::{open, GenericImageView, ImageBuffer, Pixel};
use imageproc::drawing::{draw_text_mut, text_size};
use imageproc::filter::gaussian_blur_f32;
use rand::distributions::{Distribution, Uniform};
use rand::rngs::SmallRng;
use rand::SeedableRng;
use rusttype::{Font, Scale};
use std::path::PathBuf;

static SEED: u64 = 0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up coin generator
    let coin = Uniform::new_inclusive(0, 1);

    // Set up RNG
    let mut rng = SmallRng::seed_from_u64(SEED);

    // Load font
    let font = Vec::from(include_bytes!("../resources/NotoSansMono-Regular.ttf") as &[u8]);
    let font = Font::try_from_vec(font).unwrap();

    // Set up scale
    let intended_char_height = 128.0;
    let scale = Scale::uniform(intended_char_height);
    let (char_width, _) = text_size(scale, &font, "0");
    let vmetrics = font.v_metrics(scale);
    let char_height = (vmetrics.ascent + vmetrics.descent).round() as i32;

    // Set up start cursor
    let start_x = 0;
    let start_y = vmetrics.descent.round() as i32;

    // I/O paths
    let input = PathBuf::from("input.png");
    let output = PathBuf::from("output.png");

    // Load image
    let image = open(&input)?;
    let (width, height) = image.dimensions();

    // Calculate the dimension of 1/0 bitmap
    let text_width = (width / char_width as u32) as usize;
    let text_height = (height / char_height as u32) as usize;

    // Build canvas
    let canvas = ImageBuffer::from_par_fn(width, height, |x, y| {
        image
            .get_pixel(x, y)
            .map(|p| (p as f32 * 0.7).round() as u8)
    });
    let mut canvas = gaussian_blur_f32(&canvas, 8.0);

    // Build 1/0 render layout
    let mut bitmap = vec![vec![0; text_width]; text_height];
    for row in 0..text_height {
        for col in 0..text_width {
            bitmap[row][col] = coin.sample(&mut rng);
        }
    }

    // Draw text on canvas
    let mut x = start_x;
    let mut y = start_y;
    for line in bitmap {
        for bit in line {
            let s = bit.to_string();
            let text = s.as_str();
            let pixel = image.get_pixel((x + char_width / 2) as u32, (y + char_height / 2) as u32);
            draw_text_mut(&mut canvas, pixel, x, y, scale, &font, text);
            x += char_width;
        }
        x = 0;
        y += char_height;
    }

    canvas.save(&output)?;

    Ok(())
}
