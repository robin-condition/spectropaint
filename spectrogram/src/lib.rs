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

    pub fn to_phase_bytes(&self, buffer: &mut [u8]) {
        for x in 0..self.width {
            for y in 0..self.height {
                buffer[(self.height - 1 - y) * self.width + x] =
                    u8::as_frac(self.get_at(x, y).arg() / TAU + 0.5f32);
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

pub struct SpectrogramResult {}

pub struct SpectrogramResultFrame {}
