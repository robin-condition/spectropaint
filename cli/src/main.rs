use std::{fs::File, time::Duration};

use image::{ImageBuffer, Luma};
use rodio::{Decoder, Source, buffer::SamplesBuffer};
use spectrogram::{SpectrogramImage, SpectrogramSettings};

fn main() {
    let args: Vec<_> = std::env::args().collect();

    let settings = SpectrogramSettings {
        window_size: 3000,
        window_pad_amnt: 1096,
    };
    println!("{:?}", args);
    let audio: Box<dyn Source>;
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

    let sane_reverse = spectrogram::inverse::inverse_mt(&res, &settings, 2, false);

    let mut orig = SamplesBuffer::new(1, sr, sane_reverse);
    rodio::output_to_wav(&mut orig, "results/original_reconstructed.wav").unwrap();

    println!("Spectrogram made");
    let view_bytes = res.create_intensity_bytes(-3f32, 10f32);
    let view_phase_bytes = res.create_relative_phase_bytes(0f32);

    let masked_phase_bytes = {
        let mut bytes = view_phase_bytes.clone();
        for ind in 0..bytes.len() {
            if view_bytes[ind] < 150u8 {
                bytes[ind] = 0;
            }
        }
        bytes
    };

    //cleanup_phase(&mut res);

    // Nuke phase
    res.eliminate_phase();
    //res.apply_random_phases();
    //res.apply_sinusoidal_phases(settings.window_size);

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

    let view_screwed_up_phase_bytes = res.create_relative_phase_bytes(0f32);
    ImageBuffer::<Luma<u8>, Vec<u8>>::from_vec(
        res.width as u32,
        res.height as u32,
        view_screwed_up_phase_bytes,
    )
    .unwrap()
    .save("results/bungled_phase.png")
    .unwrap();
}
