use std::sync::Arc;

use realfft::ComplexToReal;
use rustfft::num_complex::{Complex, Complex32};

use crate::{SpectrogramImage, SpectrogramSettings};

use std::sync::mpsc;

fn undo_to_real_no_changes(
    fft: &Arc<dyn ComplexToReal<f32>>,
    query: &mut [Complex32],
    pad_amnt: usize,
    awful_hack: bool,
) -> Vec<f32> {
    let mut outputs = fft.make_output_vec();

    if awful_hack && (query[0].im != 0f32 || query[query.len() - 1].im != 0f32) {
        query[0] = Complex::ZERO;
        query[query.len() - 1] = Complex::ZERO;
    }

    fft.process(query, &mut outputs).unwrap();
    let halflen = (outputs.len() - pad_amnt) / 2;
    outputs.rotate_right(halflen);
    outputs.resize(outputs.len() - pad_amnt, 0f32);
    outputs
}

pub fn inverse_mt(
    spectrogram: &SpectrogramImage,
    settings: &SpectrogramSettings,
    thread_ct: usize,
    awful_hack: bool,
) -> Vec<f32> {
    let window_size = settings.window_size;
    let pad_amnt = settings.window_pad_amnt;

    if window_size % 2 == 1 {
        panic!()
    }

    let hop_size = window_size / 2;

    let total_sample_count = hop_size * spectrogram.width as usize + hop_size;

    let mut planner = realfft::RealFftPlanner::new();
    let ifft = planner.plan_fft_inverse(window_size + settings.window_pad_amnt);

    let spectrum_size = ifft.complex_len();

    let img_height = spectrum_size;
    let img_width = spectrogram.width;

    let mut output_samples = Vec::new();
    output_samples.resize(total_sample_count, 0f32);

    println!("Beginning ifft");

    let threadless_segs = spectrogram.width % thread_ct;
    let segs_per_thread_usually = spectrogram.width / thread_ct;

    let (sender, recvr) = std::sync::mpsc::channel();
    let static_sender = Arc::new(sender);

    std::thread::scope(|scop| {
        let mut threads = vec![];
        let mut starting_segment = 0;

        for t_id in 0..thread_ct {
            let segments_for_this_thread =
                segs_per_thread_usually + if t_id == 0 { threadless_segs } else { 0 };

            let starting_segment_for_this = starting_segment;

            let sender_arc = static_sender.clone();

            let spec_ref = &spectrogram;
            let cloned_fft = ifft.clone();

            threads.push(scop.spawn(move || {
                let mut sample_start_ind = starting_segment_for_this * hop_size;
                let mut spectrum = cloned_fft.make_input_vec();
                for seg_ind in 0..segments_for_this_thread {
                    let x = seg_ind + starting_segment_for_this;
                    spectrogram.get_column(x, &mut spectrum);

                    let processed =
                        undo_to_real_no_changes(&cloned_fft, &mut spectrum, pad_amnt, awful_hack);
                    assert_eq!(processed.len(), window_size);
                    sender_arc.send((sample_start_ind, processed)).unwrap();

                    sample_start_ind += hop_size;
                }
            }));

            starting_segment += segments_for_this_thread;
        }
        drop(static_sender);

        while let Ok((ind, proc)) = recvr.recv() {
            for i in 0..proc.len() {
                output_samples[i + ind] += proc[i];
            }
        }

        drop(threads);
    });

    println!("Ifft done");

    let len_recip = ((window_size + pad_amnt) as f32).recip();

    for val in &mut output_samples {
        *val *= len_recip / 2f32;
    }

    println!("Normalization done");

    output_samples
}
