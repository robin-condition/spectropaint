use std::{
    f32::consts::{PI, TAU},
    os::linux::raw::stat,
    sync::Arc,
    thread,
};

use image::{EncodableLayout, ImageBuffer, Luma, Primitive, Rgb};
use realfft::{ComplexToReal, RealToComplex};
use rodio::Sample;
use rustfft::{
    Fft,
    num_complex::{Complex, Complex32},
};

pub trait UThing {
    fn as_frac(v: f32) -> Self;
    fn to_frac(self) -> f32;
}
impl UThing for u8 {
    fn as_frac(v: f32) -> Self {
        (v * u8::MAX as f32) as u8
    }

    fn to_frac(self) -> f32 {
        self as f32 / u8::MAX as f32
    }
}
impl UThing for u16 {
    fn as_frac(v: f32) -> Self {
        (v * u16::MAX as f32) as u16
    }

    fn to_frac(self) -> f32 {
        self as f32 / u16::MAX as f32
    }
}

pub struct SpectrogramImage {
    pub width: usize,
    pub height: usize,
    pub data: Vec<Complex32>,
}

impl SpectrogramImage {
    pub fn get_at(&self, x: usize, y: usize) -> Complex32 {
        self.data[y * self.width + x]
    }

    pub fn mut_get_at(&mut self, x: usize, y: usize) -> &mut Complex32 {
        &mut self.data[y * self.width + x]
    }

    pub fn phaseless_from_intensity_bytes(&mut self, min_val: f32, max_val: f32, buffer: &[u8]) {
        let range = max_val - min_val;
        for x in 0..self.width {
            for y in 0..self.height {
                let byte_val = buffer[(self.height - 1 - y) * self.width + x];
                let as_float = if byte_val == 0 {
                    f32::NEG_INFINITY
                } else {
                    byte_val.to_frac()
                };
                let un_normalized = as_float * range + min_val;
                let un_log = un_normalized.exp();
                let intensity = un_log;
                *self.mut_get_at(x, y) = Complex::from(intensity);
            }
        }
    }

    pub fn apply_random_phases(&mut self) {
        for x in 0..self.width {
            for y in 0..self.height {
                *self.mut_get_at(x, y) *= (Complex::i() * rand::random_range(0f32..TAU)).exp();
            }
        }
    }

    pub fn apply_sinusoidal_phases(&mut self, assume_window_size: usize) {
        let hop_size = assume_window_size / 2;

        for x in 0..self.width {
            for y in 0..self.height {
                let samples_before_this = x * hop_size;
                let actual_y = y; //self.height as i32 / 2 - y as i32 / 2 - 1;
                // The 2 * pi and self.height * 2 cancel a 2. Though maybe it should be /assume_window_size * TAU with not quite cancellation. idk.
                //let this_frequency = actual_y as f32 / self.height as f32 * PI;
                //let this_frequency = actual_y as f32 / assume_window_size as f32 * TAU;
                //let offset = 0f32;

                let offset = if y < 27 || y == 28 { PI } else { 0f32 };

                let this_frequency = 440f32 * TAU / 48000f32;

                //let this_frequency = y * PI * x;
                *self.mut_get_at(x, y) *= (Complex::i() *
                //(this_frequency * samples_before_this as f32 + offset) as f32)
                ((27.5f32) * x as f32 * PI + if y % 2 == 0 { 0f32 } else {PI}))
                .exp();
            }
        }
    }

    pub fn to_phase_bytes(&self, buffer: &mut [u8]) {
        for x in 0..self.width {
            for y in 0..self.height {
                buffer[(self.height - 1 - y) * self.width + x] =
                    u8::as_frac(self.get_at(x, y).arg() / TAU + 0.5f32);
            }
        }
    }

    pub fn eliminate_phase(&mut self) {
        for x in 0..self.width {
            for y in 0..self.height {
                *self.mut_get_at(x, y) = self.get_at(x, y).norm().into();
            }
        }
    }

    pub fn to_intensity_bytes(&self, min_val: f32, max_val: f32, buffer: &mut [u8]) {
        let range = max_val - min_val;
        for x in 0..self.width {
            for y in 0..self.height {
                buffer[(self.height - 1 - y) * self.width + x] =
                    u8::as_frac((self.get_at(x, y).norm_sqr().ln() * 0.5f32 - min_val) / range);
            }
        }
    }

    pub fn create_intensity_bytes(&self, min: f32, max: f32) -> Vec<u8> {
        let mut myvec = Vec::new();
        myvec.resize(self.width * self.height, 0);
        self.to_intensity_bytes(min, max, &mut myvec);
        myvec
    }

    pub fn create_phase_bytes(&self) -> Vec<u8> {
        let mut myvec = Vec::new();
        myvec.resize(self.width * self.height, 0);
        self.to_phase_bytes(&mut myvec);
        myvec
    }

    pub fn get_column(&self, x: usize, spectrum: &mut [Complex32]) {
        for y in 0..self.height {
            spectrum[y] = self.get_at(x, y);
        }
    }

    pub fn set_column(&mut self, x: usize, spectrum: &[Complex32]) {
        for y in 0..self.height {
            *self.mut_get_at(x, y) = spectrum[y];
        }
    }

    pub fn new_empty(width: usize, height: usize) -> Self {
        let mut data = Vec::new();
        data.resize(width * height, Complex32::ZERO);
        Self {
            width,
            height,
            data,
        }
    }
}

pub mod forward;

pub mod inverse;
