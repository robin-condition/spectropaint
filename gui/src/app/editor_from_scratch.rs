use std::{fs::File, io::Read, sync::Arc};

use egui::{
    Color32, Image, ImageSource, Sense, TextureHandle, TextureOptions, Vec2,
    load::{ImagePoll, SizedTexture},
    scroll_area::ScrollSource,
    vec2,
};
use egui_file_dialog::FileDialog;
use rodio::{OutputStream, buffer::SamplesBuffer};
use rustfft::num_complex::{Complex, Complex32};
use spectrogram::{SpectrogramImage, SpectrogramIntensityPlotSettings, SpectrogramSettings};

pub struct MyEditor {
    image: TextureHandle,
    sized_tx: Option<SizedTexture>,
    spectrogram: SpectrogramImage,
    samples: Option<Vec<f32>>,
    width: usize,
    window_len: usize,
    img_height: usize,
    stream: OutputStream,

    layout_img: Option<egui::load::Bytes>,

    file_picker: FileDialog,

    scale: Vec2,

    cursor_brightness: f32,

    intensity_settings: SpectrogramIntensityPlotSettings,

    sample_rate: usize,
}

impl MyEditor {
    pub fn new(
        cc: &eframe::CreationContext,
        width: usize,
        window_len: usize,
        sample_rate: usize,
        max_freq: f32,
    ) -> Self {
        let height = window_len / 2 + 1;
        let max_bin = SpectrogramImage::compute_bin_number(window_len, sample_rate, max_freq);
        let img_height = max_bin;
        Self {
            image: cc.egui_ctx.load_texture(
                "hello",
                egui::ColorImage::new(
                    [width, img_height],
                    vec![Color32::BLACK; width * img_height],
                ),
                TextureOptions::NEAREST,
            ),
            spectrogram: SpectrogramImage {
                width,
                height,
                data: vec![Complex32::ZERO; width * height],
            },
            intensity_settings: SpectrogramIntensityPlotSettings {
                bin_range: [0, img_height],
                intensity_range: [0f32, 10f32],
            },
            cursor_brightness: 20f32,
            layout_img: None,
            sized_tx: None,
            width,
            sample_rate,
            img_height,
            file_picker: FileDialog::new(),
            window_len,
            stream: rodio::OutputStreamBuilder::open_default_stream().unwrap(),
            samples: None,
            scale: vec2(15f32, 15f32),
        }
    }

    fn reset_img(&mut self) {
        let colors = self
            .spectrogram
            .create_intensity_bytes(&self.intensity_settings)
            .iter()
            .map(|f| Color32::from_rgb(*f, *f, *f))
            .collect();

        let img = egui::ColorImage::new([self.width, self.img_height], colors);
        self.image.set(img, TextureOptions::NEAREST);
    }

    fn draw_img_and_let_changes_affect_spectrogram<'a>(
        &mut self,
        img: Image<'a>,
        ui: &mut egui::Ui,
        changed: &mut bool,
    ) {
        let resp = ui.add(img.sense(Sense::drag()));
        if resp.dragged() {
            if let Some(p) = resp.interact_pointer_pos() {
                let norm = (p - resp.rect.min) / resp.rect.size();
                if norm.x >= 0f32 && norm.x < 1f32 && norm.y >= 0f32 && norm.y < 1f32 {
                    let to_img_px = [
                        (norm.x * self.width as f32) as usize,
                        self.img_height - 1 - (norm.y * self.img_height as f32) as usize,
                    ];

                    let brightness = if resp.dragged_by(egui::PointerButton::Secondary) {
                        0f32
                    } else {
                        self.cursor_brightness
                    };
                    *self.spectrogram.mut_get_at(to_img_px[0], to_img_px[1]) =
                        Complex32::from(brightness);
                    *changed = true;
                }
            }
        }

        if resp.contains_pointer() && ui.input(|inp| inp.modifiers.ctrl) {
            let scrolled = ui.input(|inp| inp.raw_scroll_delta).y;
            if scrolled != 0f32 {
                self.cursor_brightness *= (scrolled / 20f32).exp();
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        if ui.button("Play").clicked() {
            self.play();
        }
        if ui.button("Choose overlay").clicked() {
            self.file_picker.pick_file();
        }
        egui::containers::ScrollArea::both()
            .scroll_source(ScrollSource::SCROLL_BAR | ScrollSource::MOUSE_WHEEL)
            .show(ui, |ui| {
                if self.sized_tx.is_none() {
                    self.sized_tx = Some(SizedTexture::new(
                        &self.image,
                        vec2(
                            self.width as f32 * self.scale.x,
                            self.img_height as f32 * self.scale.y,
                        ),
                    ));
                }

                let changed = &mut false;
                ui.horizontal(|ui| {
                    self.draw_img_and_let_changes_affect_spectrogram(
                        Image::new(self.sized_tx.unwrap()),
                        ui,
                        changed,
                    );

                    if let Some(overlay) = &self.layout_img {
                        self.draw_img_and_let_changes_affect_spectrogram(
                            Image::new(ImageSource::Bytes {
                                uri: std::borrow::Cow::Owned("Hi".to_string()),
                                bytes: overlay.clone(),
                            })
                            .fit_to_exact_size(self.sized_tx.unwrap().size),
                            ui,
                            changed,
                        );
                    }
                });

                if *changed {
                    self.samples = None;
                    self.sized_tx = None;
                    self.reset_img();
                }
            });
        self.file_picker.update(ui.ctx());

        if let Some(path) = self.file_picker.take_picked() {
            // https://stackoverflow.com/questions/75728074/simplest-way-to-display-an-image-from-a-filepath
            let mut buf = vec![];
            File::open(path).unwrap().read_to_end(&mut buf).unwrap();
            self.layout_img = Some(egui::load::Bytes::Shared(buf.into()));
        }

        if ui.button("Clear").clicked() {
            self.spectrogram.data = vec![Complex::ZERO; self.width * self.spectrogram.height];
            self.samples = None;
            self.sized_tx = None;
            self.cursor_brightness = 20f32;
            self.reset_img();
        }
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

        let buffer = SamplesBuffer::new(1, self.sample_rate as u32, dat);
        self.stream.mixer().add(buffer);
    }
}
