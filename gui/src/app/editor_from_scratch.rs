use egui::{Color32, Image, Sense, TextureHandle, TextureOptions, Vec2, load::SizedTexture, vec2};
use rodio::{OutputStream, buffer::SamplesBuffer};
use rustfft::num_complex::{Complex, Complex32};
use spectrogram::{SpectrogramImage, SpectrogramSettings};

pub struct MyEditor {
    image: TextureHandle,
    sized_tx: Option<SizedTexture>,
    spectrogram: SpectrogramImage,
    samples: Option<Vec<f32>>,
    width: usize,
    window_len: usize,
    height: usize,
    stream: OutputStream,
}

impl MyEditor {
    pub fn new(
        cc: &eframe::CreationContext,
        width: usize,
        height: usize,
        window_len: usize,
    ) -> Self {
        Self {
            image: cc.egui_ctx.load_texture(
                "hello",
                egui::ColorImage::new([width, height], vec![Color32::BLACK; width * height]),
                TextureOptions::NEAREST,
            ),
            spectrogram: SpectrogramImage {
                width,
                height,
                data: vec![Complex32::ZERO; width * height],
            },
            sized_tx: None,
            width,
            height,
            window_len,
            stream: rodio::OutputStreamBuilder::open_default_stream().unwrap(),
            samples: None,
        }
    }

    fn reset_img(&mut self) {
        let colors = self
            .spectrogram
            .create_intensity_bytes(-3f32, 10f32)
            .iter()
            .map(|f| Color32::from_rgb(*f, *f, *f))
            .collect();

        let img = egui::ColorImage::new([self.width, self.height], colors);
        self.image.set(img, TextureOptions::NEAREST);
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        if ui.button("Clear").clicked() {
            self.spectrogram.data = vec![Complex::ZERO; self.width * self.height];
            self.samples = None;
            self.sized_tx = None;
            self.reset_img();
        }
        if ui.button("Play").clicked() {
            self.play();
        }
        egui::containers::ScrollArea::both().show(ui, |ui| {
            if self.sized_tx.is_none() {
                self.sized_tx = Some(SizedTexture::new(
                    &self.image,
                    vec2(self.width as f32 * 10f32, self.height as f32 * 5f32),
                ));
            }

            let resp = ui.add(Image::new(self.sized_tx.unwrap()).sense(Sense::drag()));
            let mut changed = false;
            if resp.dragged() {
                if let Some(p) = resp.interact_pointer_pos() {
                    let norm = (p - resp.rect.min) / resp.rect.size();
                    if norm.x >= 0f32 && norm.x < 1f32 && norm.y >= 0f32 && norm.y < 1f32 {
                        let to_px = [
                            (norm.x * self.width as f32) as usize,
                            self.height - (norm.y * self.height as f32) as usize,
                        ];
                        *self.spectrogram.mut_get_at(to_px[0], to_px[1]) = Complex32::from(100f32);
                        changed = true;
                    }
                }
            }

            if changed {
                self.samples = None;
                self.sized_tx = None;
                self.reset_img();
            }
        });
    }

    pub fn play(&mut self) {
        if self.samples.is_none() {
            self.samples = Some(spectrogram::inverse::inverse_mt(
                &self.spectrogram,
                &SpectrogramSettings {
                    window_size: self.window_len,
                    window_pad_amnt: 0,
                },
                4,
                true,
            ));
        }

        let dat = self.samples.clone().unwrap();

        let buffer = SamplesBuffer::new(1, 10000, dat);
        self.stream.mixer().add(buffer);
    }
}
