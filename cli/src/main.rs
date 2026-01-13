use std::{fs::File, time::Duration};

use image::{ImageBuffer, Luma};
use realfft::RealFftPlanner;
use rodio::{Decoder, Source, buffer::SamplesBuffer};
use rustfft::{FftPlanner, num_complex::Complex};
use spectrogram::SpectrogramSettings;

fn main() {
    let args: Vec<_> = std::env::args().collect();

    let settings = SpectrogramSettings {
        window_size: 3000,
        window_pad_amnt: 0,
    };
    println!("{:?}", args);
    let mut audio: Box<dyn Source>;
    if args.len() >= 2 {
        let fs = File::open(&args[1]).unwrap();
        audio = Box::new(rodio::Decoder::try_from(fs).unwrap());
    } else {
        audio =
            Box::new(rodio::source::SineWave::new(1200f32).take_duration(Duration::new(100, 0)));
    }
    //let mut audio = rodio::Decoder::try_from(fs).unwrap();
    //let audio = rodio::source::SineWave::new(1200f32).take_duration(Duration::new(100, 0));
    //rodio::output_to_wav(&mut audio.clone(), "results/original.wav").unwrap();
    let channels = audio.channels();
    let sr = audio.sample_rate();
    println!("{}", channels);
    let samples: Vec<_> = audio.step_by(channels as usize).collect();

    // //

    // let mut mutable_samples: Vec<_> = samples
    //     .iter()
    //     .cloned()
    //     .map(|f| Complex::from(f as f64))
    //     .collect();

    // let mut planner = FftPlanner::new();
    // let fft = planner.plan_fft_forward(samples.len());

    // let mut scratch = vec![Complex::ZERO; fft.get_outofplace_scratch_len()];

    // let mut output = vec![Complex::ZERO; samples.len()];

    // fft.process_outofplace_with_scratch(&mut mutable_samples, &mut output, &mut scratch);

    // let ifft = planner.plan_fft_inverse(samples.len());
    // ifft.process_outofplace_with_scratch(&mut output, &mut mutable_samples, &mut scratch);

    // println!("Roundtrip done.");

    // let mut total_i_component = 0f64;
    // let mut total_neg = 0f64;
    // let mut total_pos = 0f64;
    // for i in &mutable_samples {
    //     total_i_component += i.im.abs();
    //     if i.re < 0f64 {
    //         total_neg -= i.re;
    //     } else {
    //         total_pos += i.re;
    //     }
    // }
    // println!("Test: {}, {}, {}", total_i_component, total_neg, total_pos);

    // let mut total_diff = 0f64;
    // for i in mutable_samples.iter().zip(samples.iter()) {
    //     total_diff += (i.0.re - *i.1 as f64).abs();
    // }

    // let len = mutable_samples.len();

    // for i in &mut mutable_samples {
    //     *i /= len as f64;
    // }

    // println!("Total diff: {}", total_diff);

    // let mut orig_orig = SamplesBuffer::new(
    //     1,
    //     sr,
    //     mutable_samples
    //         .into_iter()
    //         .map(|f| f.re as f32)
    //         .collect::<Vec<_>>(),
    // );
    // rodio::output_to_wav(&mut orig_orig, "results/singlechannel_orig_1fft.wav").unwrap();

    let mut orig_orig = SamplesBuffer::new(1, sr, samples.clone());
    rodio::output_to_wav(&mut orig_orig, "results/singlechannel_orig.wav").unwrap();

    //

    let mut res = spectrogram::forward::analyze_mt::<u8>(&samples, &settings, 15).unwrap();

    let sane_reverse = spectrogram::inverse::inverse_st(&res, &settings, false);

    let mut orig = SamplesBuffer::new(1, sr, sane_reverse);
    rodio::output_to_wav(&mut orig, "results/original_reconstructed.wav").unwrap();

    println!("Spectrogram made");
    let view_bytes = res.create_intensity_bytes(-3f32, 10f32);
    let view_phase_bytes = res.create_phase_bytes();

    // Nuke phase
    res.eliminate_phase();
    //res.apply_random_phases();
    res.apply_sinusoidal_phases(settings.window_size);

    let reverse = spectrogram::inverse::inverse_st(&res, &settings, true);
    let mut aud = SamplesBuffer::new(1, sr, reverse);
    rodio::output_to_wav(&mut aud, "results/mywav.wav").unwrap();

    let img_buffer: ImageBuffer<Luma<u8>, Vec<_>> =
        ImageBuffer::from_vec(res.width as u32, res.height as u32, view_bytes).unwrap();
    img_buffer.save("results/dest.png").unwrap();
    ImageBuffer::<Luma<u8>, Vec<u8>>::from_vec(
        res.width as u32,
        res.height as u32,
        view_phase_bytes,
    )
    .unwrap()
    .save("results/phase.png")
    .unwrap();

    let view_screwed_up_phase_bytes = res.create_phase_bytes();
    ImageBuffer::<Luma<u8>, Vec<u8>>::from_vec(
        res.width as u32,
        res.height as u32,
        view_screwed_up_phase_bytes,
    )
    .unwrap()
    .save("results/bungled_phase.png")
    .unwrap();
}
