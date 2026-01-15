use std::{fs::File, time::Duration};

use image::{ImageBuffer, Luma};
use realfft::RealFftPlanner;
use rodio::{Decoder, Source, buffer::SamplesBuffer};
use rustfft::{FftPlanner, num_complex::Complex, num_traits::ConstZero};
use spectrogram::{SpectrogramImage, SpectrogramSettings};

fn cleanup_phase(spec: &mut SpectrogramImage) {
    for Y in 0..spec.height {
        if Y % 2 == 0 {
            continue;
        }
        for x in 0..spec.width {
            //let mut res = vec![Complex::ZERO; spec.height];
            //spec.get_column(x, &mut res);
            //for y in 1..(spec.height - 1) {
            //    *spec.mut_get_at(x, y) -= res[y - 1] + res[y + 1];
            // }
            *spec.mut_get_at(x, Y) *= -1f32;
        }
    }
}

fn update_image(width: usize, height: usize, v: &mut [u8], mut spec: SpectrogramImage) {
    let mag_bytes = spec.create_intensity_bytes(-3f32, 10f32);
    //cleanup_phase(&mut spec);
    let phase_bytes = spec.create_relative_phase_bytes();
    for x in 0..width {
        for y in 0..height {
            let ind = spec.get_index(x, y);
            if mag_bytes[ind] < 100u8 {
                continue;
            }
            //let prev = v[ind];
            //let cur_phase_byte = phase_bytes[ind];
            //let now_val = ((prev as f32 + cur_phase_byte as f32) * 0.5f32) as u8;
            let now_val = phase_bytes[ind];
            v[ind] = now_val;
        }
    }
}

fn create_phase_delta_stuff_map() {
    let settings = SpectrogramSettings {
        window_size: 3000,
        window_pad_amnt: 0,
    };
    let mut width = 0;
    let mut height = 0;

    let mut data = Vec::new();
    for f in 1..1000 {
        let freq = f as f32;
        let audio = rodio::source::SineWave::new(freq).take_duration(Duration::from_secs(10));

        let channels = audio.channels();
        let sr = audio.sample_rate();
        let samples: Vec<_> = audio.step_by(channels as usize).collect();

        let res = spectrogram::forward::analyze_mt(&samples, &settings, 15).unwrap();
        data.resize(res.height * res.width, 0);
        width = res.width;
        height = res.height;
        update_image(res.width, res.height, &mut data, res);
        println!("Completed frequency {}", f);
    }

    ImageBuffer::<Luma<u8>, Vec<u8>>::from_vec(width as u32, height as u32, data)
        .unwrap()
        .save("results/weird_phase_test.png")
        .unwrap();
}

fn main() {
    let args: Vec<_> = std::env::args().collect();

    //create_phase_delta_stuff_map();

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
        audio = Box::new(
            rodio::source::SineWave::new(
                4037f32, //432f32,
            )
            .take_duration(Duration::new(10, 0)),
        );
    }
    //let mut audio = rodio::Decoder::try_from(fs).unwrap();
    //let audio = rodio::source::SineWave::new(1200f32).take_duration(Duration::new(100, 0));
    //rodio::output_to_wav(&mut audio.clone(), "results/original.wav").unwrap();
    let channels = audio.channels();
    let sr = audio.sample_rate();
    println!("{}", channels);
    let samples: Vec<_> = audio.step_by(channels as usize).collect();

    let mut orig_orig = SamplesBuffer::new(1, sr, samples.clone());
    rodio::output_to_wav(&mut orig_orig, "results/singlechannel_orig.wav").unwrap();

    //

    let mut res = spectrogram::forward::analyze_mt(&samples, &settings, 15).unwrap();

    let sane_reverse = spectrogram::inverse::inverse_mt(&res, &settings, 15, false);

    let mut orig = SamplesBuffer::new(1, sr, sane_reverse);
    rodio::output_to_wav(&mut orig, "results/original_reconstructed.wav").unwrap();

    println!("Spectrogram made");
    let view_bytes = res.create_intensity_bytes(-3f32, 10f32);
    let view_phase_bytes = res.create_phase_bytes();

    let masked_phase_bytes = {
        let mut bytes = view_phase_bytes.clone();
        for ind in 0..bytes.len() {
            if view_bytes[ind] < 100u8 {
                bytes[ind] = 0;
            }
        }
        bytes
    };

    //cleanup_phase(&mut res);

    // Nuke phase
    res.eliminate_phase();
    //res.apply_random_phases();
    res.apply_sinusoidal_phases(settings.window_size);

    let reverse = spectrogram::inverse::inverse_mt(&res, &settings, 15, true);
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

    ImageBuffer::<Luma<u8>, Vec<u8>>::from_vec(
        res.width as u32,
        res.height as u32,
        masked_phase_bytes,
    )
    .unwrap()
    .save("results/masked_phase.png")
    .unwrap();

    let view_screwed_up_phase_bytes = res.create_relative_phase_bytes();
    ImageBuffer::<Luma<u8>, Vec<u8>>::from_vec(
        res.width as u32,
        res.height as u32,
        view_screwed_up_phase_bytes,
    )
    .unwrap()
    .save("results/bungled_phase.png")
    .unwrap();
}
