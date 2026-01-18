use egui::Vec2;
use rustfft::num_complex::Complex;
use spectrogram::SpectrogramImage;

use crate::app::editor_from_scratch::drawing::Brush;

#[derive(Clone)]
pub struct RadiusBrush {
    pub brightness: f32,
    pub radius: f32,
}

impl RadiusBrush {
    pub fn new(brightness: f32, radius: f32) -> Self {
        Self { brightness, radius }
    }
}

impl Brush for RadiusBrush {
    fn update_with_scroll(&mut self, delta: f32) {
        self.brightness *= (delta / 20f32).exp();
    }

    fn apply(&self, img: &mut SpectrogramImage, bin_range: [usize; 2], norm_pos: Vec2) {
        let bin_diff = bin_range[1] - bin_range[0];
        let img_center = [
            (norm_pos.x * img.width as f32),
            (norm_pos.y * bin_diff as f32) + bin_range[0] as f32,
        ];
        let rounded_center = [img_center[0] as usize, img_center[1] as usize];
        let rounded_rad = self.radius.ceil() as i32;
        let diam = rounded_rad * 2 + 1;
        for x in -rounded_rad..rounded_rad + 1 {
            for y in -rounded_rad..rounded_rad + 1 {
                let translated_x = x + rounded_center[0] as i32;
                let translated_y = y + rounded_center[1] as i32;
                if translated_x >= 0
                    && translated_y >= 0
                    && (translated_x as usize) < img.width
                    && (translated_y as usize) < bin_range[1]
                {
                    let square = [translated_x as usize, translated_y as usize];
                    let dx = square[0] as f32 - img_center[0];
                    let dy = square[1] as f32 - img_center[1];
                    let dist = (dx * dx + dy * dy); //.sqrt();
                    if dist <= self.radius * self.radius {
                        let bgt = self.brightness * (1f32 - dist / (self.radius * self.radius));
                        let num = img.mut_get_at(square[0], square[1]);
                        if num.norm_sqr() < bgt * bgt {
                            *num = Complex::from(bgt);
                        }
                    }
                }
            }
        }
    }
}
