use egui::Vec2;
use rustfft::num_complex::Complex;
use spectrogram::SpectrogramImage;

use crate::app::editor_from_scratch::drawing::Brush;

#[derive(Clone)]
pub struct SolidMagBrush {
    pub brightness: f32,
}

impl SolidMagBrush {
    pub fn new(start: f32) -> Self {
        Self { brightness: start }
    }
}

impl Brush for SolidMagBrush {
    fn update_with_scroll(&mut self, delta: f32) {
        self.brightness *= (delta / 20f32).exp();
    }

    fn apply(&self, img: &mut SpectrogramImage, bin_range: [usize; 2], norm_pos: Vec2) {
        let bin_diff = bin_range[1] - bin_range[0];
        let img_coord = [
            (norm_pos.x * img.width as f32) as usize,
            (norm_pos.y * bin_diff as f32) as usize + bin_range[0],
        ];
        *img.mut_get_at(img_coord[0], img_coord[1]) = Complex::from(self.brightness);
    }
}
