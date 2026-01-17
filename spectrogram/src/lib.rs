use std::{
    f32::consts::{PI, TAU},
    sync::Arc,
    thread,
};

use image::{EncodableLayout, ImageBuffer, Luma, Primitive, Rgb};
use realfft::{ComplexToReal, RealToComplex};
use rodio::Sample;
use rustfft::{
    Fft,
    num_complex::{Complex, Complex32},
    num_traits::ConstZero,
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

#[derive(Clone, Copy)]
pub struct SpectrogramSettings {
    pub window_size: usize,
    pub window_pad_amnt: usize,
}

#[derive(Clone, Copy)]
pub struct SpectrogramIntensityPlotSettings {
    pub bin_range: [usize; 2],
    pub intensity_range: [f32; 2],
}

#[derive(Clone, Copy)]
pub struct SpectrogramPhasePlotSettings {
    pub bin_range: [usize; 2],
    pub lower_seam: f32,
}

trait PhaselessAmplitudeApplier {
    fn apply_intensity(img: &mut SpectrogramImage, x: usize, y: usize, intensity: f32);
}

struct OverrideAmplitudeApplier;
impl PhaselessAmplitudeApplier for OverrideAmplitudeApplier {
    fn apply_intensity(img: &mut SpectrogramImage, x: usize, y: usize, intensity: f32) {
        *img.mut_get_at(x, y) = Complex::from(intensity);
    }
}

struct MultiplyByAmplitudeApplier;
impl PhaselessAmplitudeApplier for MultiplyByAmplitudeApplier {
    fn apply_intensity(img: &mut SpectrogramImage, x: usize, y: usize, intensity: f32) {
        *img.mut_get_at(x, y) *= intensity;
    }
}

trait ZeroingBehavior {
    fn zero_outside_range(
        img: &mut SpectrogramImage,
        x: usize,
        first_bin: usize,
        last_bin_plus_one: usize,
    );
}

struct ZeroOutsideRange;
impl ZeroingBehavior for ZeroOutsideRange {
    fn zero_outside_range(
        img: &mut SpectrogramImage,
        x: usize,
        first_bin: usize,
        last_bin_plus_one: usize,
    ) {
        for y in 0..first_bin {
            *img.mut_get_at(x, y) = Complex::ZERO;
        }
        for y in last_bin_plus_one..img.height {
            *img.mut_get_at(x, y) = Complex::ZERO;
        }
    }
}

struct NoZeroing;
impl ZeroingBehavior for NoZeroing {
    fn zero_outside_range(
        img: &mut SpectrogramImage,
        x: usize,
        first_bin: usize,
        last_bin_plus_one: usize,
    ) {
    }
}

impl SpectrogramImage {
    pub fn compute_bin_number(window_size: usize, sample_rate: usize, frequency: f32) -> usize {
        (frequency / sample_rate as f32 * window_size as f32) as usize
    }

    pub fn get_at(&self, x: usize, y: usize) -> Complex32 {
        self.data[y * self.width + x]
    }

    pub fn get_index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn mut_get_at(&mut self, x: usize, y: usize) -> &mut Complex32 {
        &mut self.data[y * self.width + x]
    }

    pub fn phaseless_from_intensity_bytes(
        &mut self,
        settings: &SpectrogramIntensityPlotSettings,
        buffer: &[u8],
        zero_outside: bool,
    ) {
        if zero_outside {
            self.internal_apply_intensity_bytes::<OverrideAmplitudeApplier, ZeroOutsideRange>(
                settings, buffer,
            );
        } else {
            self.internal_apply_intensity_bytes::<OverrideAmplitudeApplier, NoZeroing>(
                settings, buffer,
            );
        }
    }

    fn internal_apply_intensity_bytes<Appl: PhaselessAmplitudeApplier, Zeroing: ZeroingBehavior>(
        &mut self,
        settings: &SpectrogramIntensityPlotSettings,
        buffer: &[u8],
    ) {
        let range = settings.intensity_range[1] - settings.intensity_range[0];
        for x in 0..self.width {
            for y in settings.bin_range[0]..settings.bin_range[1] {
                let buf_y = y - settings.bin_range[0];
                let byte_val = buffer[(settings.bin_range[1] - 1 - buf_y) * self.width + x];
                let as_float = if byte_val == 0 {
                    f32::NEG_INFINITY
                } else {
                    byte_val.to_frac()
                };
                let un_normalized = as_float * range + settings.intensity_range[0];
                let un_log = un_normalized.exp();
                let intensity = un_log;
                Appl::apply_intensity(self, x, y, intensity);
            }

            Zeroing::zero_outside_range(self, x, settings.bin_range[0], settings.bin_range[1]);
        }
    }

    pub fn apply_intensity_bytes(
        &mut self,
        settings: &SpectrogramIntensityPlotSettings,
        buffer: &[u8],
    ) {
        self.internal_apply_intensity_bytes::<MultiplyByAmplitudeApplier, NoZeroing>(
            settings, buffer,
        );
    }

    pub fn normalize_magnitudes_no_nans(&mut self) {
        for c in &mut self.data {
            *c = Complex::from_polar(1f32, c.arg());
        }
    }

    pub fn normalize_magnitudes_with_norm(&mut self) {
        for c in &mut self.data {
            *c /= c.norm();
        }
    }

    pub fn apply_phase_bytes(&mut self, lower_seam: f32, buffer: &[u8], relative: bool) {
        for y in 0..self.height {
            let mut phased = Complex::from(1f32);
            for x in 0..self.width {
                let curr_byte = buffer[y * self.width];
                let phase = curr_byte.to_frac() * TAU + lower_seam;
                let curr_phase = (Complex::i() * phase).exp();
                if relative {
                    phased *= curr_phase;
                } else {
                    phased = curr_phase;
                }

                *self.mut_get_at(x, y) *= phased;
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

                let this_frequency_hz = y as f32 / assume_window_size as f32 * 48000f32;

                let radians_per_sec = this_frequency_hz * TAU;
                let sec_per_hop = hop_size as f32 / 48000f32;

                let radians_per_hop = sec_per_hop * radians_per_sec;

                let radians_per_hop = -TAU / 4f32; // 600 HZ

                let radians_per_hop = 0f32; // 2400 HZ

                // Guess:
                let radians_per_hop = -TAU * this_frequency_hz / 2400f32;
                //let radians_per_hop = 0.75f32 * TAU * this_frequency_hz / 2400f32;

                // Guess #2:
                let samples_per_cycle = 48000f32 / this_frequency_hz;
                let leftover_samples = hop_size as f32 % samples_per_cycle;
                let leftover_cycles = leftover_samples / samples_per_cycle;
                //let radians_per_hop = leftover_cycles * TAU;

                //let this_frequency = y * PI * x;
                *self.mut_get_at(x, y) *= (Complex::i() *
                    //(this_frequency * samples_before_this as f32 + offset) as f32)
                    //((y as f32) * x as f32 * PI + if y % 2 == 0 { 0f32 } else {PI}))
                    if (y) % 2 == 1 {-0.498958833217f32 * x as f32 + PI} else {1.07799747917f32 * x as f32})
                //radians_per_hop * x as f32)
                //PI / 2f32)
                .exp();
            }
        }
    }

    fn arg_seamed_at(complex: Complex32, lower_seam: f32) -> f32 {
        let raw_arg = complex.arg();
        let arg_0_tau = (raw_arg + TAU) % TAU;
        let arg_0_tau_shifted = ((arg_0_tau - lower_seam) + TAU) % TAU;
        arg_0_tau_shifted
    }

    pub fn to_absolute_phase_bytes(
        &self,
        settings: &SpectrogramPhasePlotSettings,
        buffer: &mut [u8],
    ) {
        for x in 0..self.width {
            for y in settings.bin_range[0]..settings.bin_range[1] {
                let buf_y = y - settings.bin_range[0];
                buffer[(settings.bin_range[1] - 1 - buf_y) * self.width + x] =
                    u8::as_frac(Self::arg_seamed_at(self.get_at(x, y), settings.lower_seam) / TAU);
            }
        }
    }

    pub fn to_relative_phase_bytes(
        &self,
        settings: &SpectrogramPhasePlotSettings,
        buffer: &mut [u8],
    ) {
        for x in 0..self.width {
            for y in settings.bin_range[0]..settings.bin_range[1] {
                let buf_y = y - settings.bin_range[0];
                if x > 0 {
                    buffer[(settings.bin_range[1] - 1 - buf_y) * self.width + x] = u8::as_frac(
                        Self::arg_seamed_at(
                            self.get_at(x, y) / self.get_at(x - 1, y),
                            settings.lower_seam,
                        ) / TAU,
                    );
                } else {
                    buffer[(settings.bin_range[1] - 1 - buf_y) * self.width + x] =
                        u8::as_frac(Self::arg_seamed_at(self.get_at(x, y), settings.lower_seam));
                }
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

    pub fn to_intensity_bytes(
        &self,
        settings: &SpectrogramIntensityPlotSettings,
        buffer: &mut [u8],
    ) {
        let range = settings.intensity_range[1] - settings.intensity_range[0];
        for x in 0..self.width {
            for y in settings.bin_range[0]..settings.bin_range[1] {
                let buf_y = y - settings.bin_range[0];
                buffer[(settings.bin_range[1] - 1 - buf_y) * self.width + x] = u8::as_frac(
                    (self.get_at(x, y).norm_sqr().ln() * 0.5f32 - settings.intensity_range[0])
                        / range,
                );
            }
        }
    }

    pub fn create_intensity_bytes(&self, settings: &SpectrogramIntensityPlotSettings) -> Vec<u8> {
        let mut myvec = Vec::new();
        myvec.resize(
            self.width * (settings.bin_range[1] - settings.bin_range[0]),
            0,
        );
        self.to_intensity_bytes(settings, &mut myvec);
        myvec
    }

    pub fn create_phase_bytes(&self, settings: &SpectrogramPhasePlotSettings) -> Vec<u8> {
        let mut myvec = Vec::new();
        myvec.resize(
            self.width * (settings.bin_range[1] - settings.bin_range[0]),
            0,
        );
        self.to_absolute_phase_bytes(settings, &mut myvec);
        myvec
    }

    pub fn create_relative_phase_bytes(&self, settings: &SpectrogramPhasePlotSettings) -> Vec<u8> {
        let mut myvec = Vec::new();
        myvec.resize(
            self.width * (settings.bin_range[1] - settings.bin_range[0]),
            0,
        );
        self.to_relative_phase_bytes(settings, &mut myvec);
        myvec
    }

    pub fn get_column(&self, x: usize, spectrum: &mut [Complex32]) {
        for y in 0..self.height {
            spectrum[y] = self.get_at(x, y);
        }
        for y in self.height..spectrum.len() {
            spectrum[y] = Complex::ZERO;
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
