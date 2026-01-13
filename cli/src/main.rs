use std::time::Duration;

use image::{ImageBuffer, Luma};
use rodio::{Source, buffer::SamplesBuffer};

fn main() {
    let window_size = 3000usize;
    //let mut audio = rodio::Decoder::try_from(fs).unwrap();
    let mut audio = rodio::source::SineWave::new(1230f32).take_duration(Duration::new(100, 0));
    let channels = audio.channels();
    let sr = audio.sample_rate();
    println!("{}", channels);
    let samples: Vec<_> = audio.step_by(channels as usize).collect();
    let mut res = spectrogram::forward::analyze_mt::<u8>(&samples, window_size, 15).unwrap();
    println!("Spectrogram made");
    let view_bytes = res.create_intensity_bytes(-3f32, 10f32);
    let view_phase_bytes = res.create_phase_bytes();

    let sane_reverse = spectrogram::inverse::inverse_st(&res, window_size);

    // Nuke phase
    res.eliminate_phase();
    //res.apply_random_phases();
    res.apply_sinusoidal_phases(window_size);

    let reverse = spectrogram::inverse::inverse_st(&res, window_size);
    let mut aud = SamplesBuffer::new(1, sr, reverse);
    rodio::output_to_wav(&mut aud, "results/mywav.wav").unwrap();

    let mut orig = SamplesBuffer::new(1, sr, sane_reverse);
    rodio::output_to_wav(&mut orig, "results/original_reconstructed.wav").unwrap();

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
