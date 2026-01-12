use std::sync::Arc;

use realfft::ComplexToReal;
use rustfft::num_complex::{Complex, Complex32};

use crate::SpectrogramImage;

fn undo_to_real_no_changes(fft: &Arc<dyn ComplexToReal<f32>>, query: &mut [Complex32]) -> Vec<f32> {
    let mut outputs = fft.make_output_vec();
    /*
    if query[0].im != 0f32 || query[query.len() - 1].im != 0f32 {
        query[0] = Complex::ZERO;
        query[query.len() - 1] = Complex::ZERO;
    }
    */
    fft.process(query, &mut outputs).unwrap();
    outputs
}

pub fn inverse_st(spectrogram: &SpectrogramImage, window_size: usize) -> Vec<f32> {
    if window_size % 2 == 1 {
        panic!()
    }

    let hop_size = window_size / 2;
    let spectrum_size = window_size / 2 + 1;

    let total_sample_count = hop_size * spectrogram.width as usize + hop_size;

    let mut planner = realfft::RealFftPlanner::new();
    let ifft = planner.plan_fft_inverse(window_size);

    let img_height = spectrum_size;
    let img_width = spectrogram.width;

    let mut output_samples = Vec::new();
    output_samples.resize(total_sample_count, 0f32);

    let mut sample_start_ind = 0;

    println!("Beginning ifft");

    let mut spectrum = ifft.make_input_vec();

    for x in 0..spectrogram.width {
        spectrogram.get_column(x, &mut spectrum);

        let processed = undo_to_real_no_changes(&ifft, &mut spectrum);
        assert_eq!(processed.len(), window_size);
        for i in 0..processed.len() {
            output_samples[i + sample_start_ind] += processed[i];
        }

        sample_start_ind += hop_size;
    }

    println!("Ifft done");

    let len_recip = (window_size as f32).recip();

    for val in &mut output_samples {
        *val *= len_recip;
    }

    println!("Normalization done");

    output_samples
}
