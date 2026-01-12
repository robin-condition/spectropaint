use std::{
    f32::consts::{PI, TAU},
    sync::Arc,
};

use image::{ImageBuffer, Luma, Primitive, Rgb};
use rodio::Sample;
use rustfft::{
    Fft,
    num_complex::{Complex, Complex32},
};

// https://en.wikipedia.org/wiki/Hann_function
fn hann(n: usize, recip_len: f32) -> f32 {
    0.5f32 * (1f32 - (TAU * n as f32 * recip_len).cos())
}

fn analyze_with_hann_window(fft: &Arc<dyn Fft<f32>>, query: &[f32]) -> Vec<Complex32> {
    let recip_len = ((query.len() - 1) as f32).recip();
    let mut inputs: Vec<Complex<f32>> = query
        .into_iter()
        .enumerate()
        .map(|(i, f)| Complex::from(f * hann(i, recip_len)))
        .collect();
    fft.process(&mut inputs);
    inputs
}

pub trait UThing {
    fn as_frac(v: f32) -> Self;
}
impl UThing for u8 {
    fn as_frac(v: f32) -> Self {
        (v * u8::MAX as f32) as u8
    }
}
impl UThing for u16 {
    fn as_frac(v: f32) -> Self {
        (v * u16::MAX as f32) as u16
    }
}

pub fn analyze<T: UThing + Primitive>(
    query: &Vec<f32>,
    window_size: usize,
) -> Option<ImageBuffer<Luma<T>, Vec<T>>> {
    if window_size % 2 == 1 {
        panic!()
    }

    let hop_size = window_size / 2;

    let not_fit_in_window = query.len() % window_size;
    let to_pad_by = window_size + window_size - not_fit_in_window;
    let to_pad_by_on_left = to_pad_by / 2;
    let to_pad_by_on_right = to_pad_by - to_pad_by_on_left;

    let padded: Vec<f32> = std::iter::repeat_n(0f32, to_pad_by_on_left)
        .chain(query.iter().cloned())
        .chain(std::iter::repeat_n(0f32, to_pad_by_on_right))
        .collect();

    let new_total_len = padded.len();

    let mut my_fft = rustfft::FftPlanner::new();
    let fft = my_fft.plan_fft_forward(window_size);

    let seg_count = new_total_len / hop_size - 1;

    let end_width = seg_count;
    // Real-valued functions have symmetric spectra
    let end_height = window_size / 2;
    let mut results = Vec::new();
    results.resize(end_width * end_height, 0f32);

    let mut segment_start = 0usize;
    for i in 0..seg_count {
        let seg = &padded[segment_start..(segment_start + window_size)];

        let analyzed = analyze_with_hann_window(&fft, seg);
        let mags: Vec<f32> = analyzed.iter().map(|f| (f.norm() * 4f32).ln()).collect();
        //results[end_width * i] = 2f32;
        let start_ind = i;

        //results[100] = 1f32;

        for ind in 0..end_height {
            let x = i;
            let y = end_height - ind - 1;
            results[y * end_width + x] = mags[ind]; // + mags[window_size - 1 - ind];
        }

        segment_start += hop_size;
    }

    // Automatically determining maximum (log) amplitude.
    let maxd = results.iter().max_by(|a, b| a.total_cmp(b)).unwrap();
    // Manually setting the lower clipping point.
    let mind = -3f32;
    // Automatic is a no-go because of -inf
    let range = maxd - mind;
    let subpx: Vec<T> = results
        .iter()
        .map(|s| T::as_frac((s - mind) / range))
        .collect();

    image::ImageBuffer::from_vec(end_width as u32, end_height as u32, subpx)
}

pub struct SpectrogramResult {}

pub struct SpectrogramResultFrame {}
