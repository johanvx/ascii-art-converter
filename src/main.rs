extern crate ffmpeg_next as ffmpeg;

use image::{ImageBuffer, Pixel, Rgb};
use imageproc::drawing::{draw_text_mut, text_size};
use imageproc::filter::gaussian_blur_f32;
use indicatif::{ProgressBar, ProgressStyle};
use ndarray::Array3;
use rand::distributions::{Distribution, Uniform};
use rand::rngs::SmallRng;
use rand::SeedableRng;
use rusttype::{Font, Scale};
use std::path::PathBuf;
use video_rs::{self, Decoder, Encoder, EncoderSettings};

static SEED: u64 = 0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize ffmpeg
    video_rs::init()?;

    // Set up coin generator
    let coin = Uniform::new_inclusive(0, 1);

    // Set up RNG
    let mut rng = SmallRng::seed_from_u64(SEED);

    // Load font
    let font = Vec::from(include_bytes!("../resources/NotoSansMono-Regular.ttf") as &[u8]);
    let font = Font::try_from_vec(font).unwrap();

    // Set up scale
    let intended_char_height = 24.0;
    let scale = Scale::uniform(intended_char_height);
    let (char_width, _) = text_size(scale, &font, "0");
    let vmetrics = font.v_metrics(scale);
    let char_height = (vmetrics.ascent + vmetrics.descent).round() as i32;

    // Set up start cursor
    let start_x = 0;
    let start_y = vmetrics.descent.round() as i32;

    // I/O paths
    let input = PathBuf::from("input.mp4");
    let output = PathBuf::from("output.mp4");

    // Get number of frames
    let context = ffmpeg::format::input(&input)?;
    let frame_count = context
        .streams()
        .best(ffmpeg::media::Type::Video)
        .unwrap()
        .frames();

    // Load video
    let mut decoder = Decoder::new(&input.into()).expect("failed to create decoder");
    let (width, height) = decoder.size();

    // Calculate the dimension of 1/0 bitmap
    let text_width = (width / char_width as u32) as usize;
    let text_height = (height / char_height as u32) as usize;

    // Set up encoder
    let settings = EncoderSettings::for_h264_yuv420p(width as usize, height as usize, false);
    let mut encoder = Encoder::new(&output.into(), settings).expect("failed to create encoder");

    // Set up progress bar
    let pb = ProgressBar::new(frame_count as u64);
    pb.set_style(ProgressStyle::with_template(
        "[{elapsed_precise}] {wide_bar} {pos:>7}/{len:7}",
    )?);

    // Process frames
    for (ts, frame) in decoder
        .decode_iter()
        .take_while(Result::is_ok)
        .map(Result::unwrap)
    {
        // Convert frame to image
        let image = frame.into_raw_vec();
        let image = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_vec(width, height, image)
            .expect("failed to convert frame to image");

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
                let pixel =
                    image.get_pixel((x + char_width / 2) as u32, (y + char_height / 2) as u32);
                draw_text_mut(&mut canvas, *pixel, x, y, scale, &font, text);
                x += char_width;
            }
            x = 0;
            y += char_height;
        }

        // Convert canvas to frame
        let frame = Array3::from_shape_vec((height as usize, width as usize, 3), canvas.into_vec())
            .expect("unable to build frame");

        // Encode frame
        encoder.encode(&frame, &ts).expect("failed to encode frame");
        pb.inc(1);
    }

    // Finish encoding
    encoder.finish().expect("failed to finish encoder");
    pb.finish();

    Ok(())
}
