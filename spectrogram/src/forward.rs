use std::{f32::consts::TAU, sync::Arc, thread};

use image::{ImageBuffer, Luma, Primitive};
use realfft::RealToComplex;
use rustfft::{
    Fft,
    num_complex::{Complex, Complex32},
};

use crate::{SpectrogramImage, UThing};

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

fn analyze_real_with_hann_window(
    fft: &Arc<dyn RealToComplex<f32>>,
    query: &[f32],
) -> Vec<Complex32> {
    let recip_len = ((query.len() - 1) as f32).recip();
    let mut inputs: Vec<f32> = query
        .into_iter()
        .enumerate()
        .map(|(i, f)| f * hann(i, recip_len))
        .collect();
    let mut outputs = fft.make_output_vec();
    fft.process(&mut inputs, &mut outputs).unwrap();
    outputs
}

pub fn analyze_mt<T: UThing + Primitive>(
    query: &Vec<f32>,
    window_size: usize,
    thread_ct: usize,
) -> Option<SpectrogramImage> {
    if window_size % 2 == 1 {
        panic!()
    }

    let hop_size = window_size / 2;

    let not_fit_in_window = query.len() % window_size;
    let to_pad_by = window_size + window_size - not_fit_in_window;
    let to_pad_by_on_left = to_pad_by / 2;
    let to_pad_by_on_right = to_pad_by - to_pad_by_on_left;

    let spectrum_size = window_size / 2 + 1;

    let padded: Vec<f32> = std::iter::repeat_n(0f32, to_pad_by_on_left)
        .chain(query.iter().cloned())
        .chain(std::iter::repeat_n(0f32, to_pad_by_on_right))
        .collect();

    let padded_ref = Arc::new(padded);

    let new_total_len = padded_ref.len();

    let mut my_fft = realfft::RealFftPlanner::new();
    let fft = my_fft.plan_fft_forward(window_size);

    let seg_count = new_total_len / hop_size - 1;

    let mut spectrogram = SpectrogramImage::new_empty(seg_count, spectrum_size);

    let threadless_segs = seg_count % thread_ct;
    let segs_per_thread_usually = seg_count / thread_ct;

    let end_width = seg_count;
    // Real-valued functions have symmetric spectra
    let end_height = spectrum_size;

    let (sender, recvr) = std::sync::mpsc::channel();
    let static_sender = Arc::new(sender);

    let mut thread_handles = vec![];

    println!("Starting threads");

    let mut global_segment_start = 0usize;
    for thread_id in 0..thread_ct {
        let this_threads_seg_count = if thread_id == 0 {
            segs_per_thread_usually + threadless_segs
        } else {
            segs_per_thread_usually
        };
        let cloned_fft = fft.clone();
        let pref_clone = padded_ref.clone();
        let cloned_arc = static_sender.clone();
        thread_handles.push(thread::spawn(move || {
            let mut segment_start = global_segment_start * hop_size;
            for i in 0..this_threads_seg_count {
                let seg = &pref_clone[segment_start..(segment_start + window_size)];

                let analyzed = analyze_real_with_hann_window(&cloned_fft, seg);
                assert_eq!(analyzed.len(), spectrum_size);
                let mags: Vec<_> = analyzed.into_iter().map(|f| f * 2f32).collect();

                let x = i + global_segment_start;
                cloned_arc.send((x, mags)).unwrap();

                segment_start += hop_size;
            }
            drop(cloned_arc);
        }));
        global_segment_start += this_threads_seg_count;
    }

    drop(static_sender);

    let my_extra_last_thread = thread::spawn(move || {
        while let Ok(symb) = recvr.recv() {
            spectrogram.set_column(symb.0, &symb.1);
        }
        spectrogram
    });

    println!("Threads made");

    for thr in thread_handles {
        thr.join().unwrap();
    }
    spectrogram = my_extra_last_thread.join().unwrap();
    println!("Threads done.");

    Some(spectrogram)
}

pub fn analyze_st<T: UThing + Primitive>(
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

    println!("Analysis begun.");

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

    println!("FFTs done.");

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

    println!("Normalization done.");

    image::ImageBuffer::from_vec(end_width as u32, end_height as u32, subpx)
}
