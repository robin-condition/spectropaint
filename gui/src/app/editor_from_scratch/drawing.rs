use egui::Vec2;
use rustfft::num_complex::Complex;
use spectrogram::SpectrogramImage;

pub trait Brush {
    fn update_with_scroll(&mut self, delta: f32);
    fn apply(&self, img: &mut SpectrogramImage, bin_range: [usize; 2], norm_pos: Vec2);
}

pub mod radius_brush;
pub mod solid_mag_brush;
