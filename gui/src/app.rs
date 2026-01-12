use std::{error::Error, path::PathBuf, thread};

use eframe::{App, CreationContext};
use egui::{TextureHandle, TextureOptions, Vec2, load::SizedTexture};
use egui_file_dialog::FileDialog;
use image::{EncodableLayout, ImageBuffer, Luma};
use rodio::{OutputStream, Source};

pub struct SpectrogramApp {
    file_dialog: FileDialog,
    stream: OutputStream,
    samples: Vec<f32>,
    image: TextureHandle,
    sized_tx: Option<SizedTexture>,
}

impl SpectrogramApp {
    pub fn new(cc: &CreationContext) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        Self {
            file_dialog: FileDialog::new(),
            stream: rodio::OutputStreamBuilder::open_default_stream().unwrap(),
            samples: vec![],
            image: cc.egui_ctx.load_texture(
                "hello",
                egui::ColorImage::example(),
                TextureOptions::NEAREST,
            ),
            sized_tx: None,
        }
    }

    fn read_file(&mut self, path: PathBuf) -> ImageBuffer<Luma<u8>, Vec<u8>> {
        let fs = std::fs::File::open(path).unwrap();
        let mut audio = rodio::Decoder::try_from(fs).unwrap();
        let channels = audio.channels();
        println!("{}", channels);
        let samples: Vec<_> = audio.step_by(channels as usize).collect();
        self.samples = samples;
        let res = spectrogram::analyze_mt::<u8>(&self.samples, 1500usize, 15).unwrap();
        res.save("dest.png").unwrap();
        res
    }
}

impl App for SpectrogramApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Select audio file!").clicked() {
                self.file_dialog.pick_file();
            }

            self.file_dialog.update(ctx);

            if let Some(path) = self.file_dialog.take_picked() {
                let imgsrc = self.read_file(path);
                let cimg = egui::ColorImage::new(
                    [imgsrc.width() as usize, imgsrc.height() as usize],
                    imgsrc
                        .as_bytes()
                        .iter()
                        .map(|&b| egui::Color32::from_rgb(b, b, b))
                        .collect(),
                );
                self.image.set(cimg, TextureOptions::NEAREST);
                let sized_texture = SizedTexture::new(
                    &self.image,
                    Vec2 {
                        x: imgsrc.width() as f32,
                        y: imgsrc.height() as f32,
                    },
                );
                self.sized_tx = Some(sized_texture);
                //egui::Image::new(imgsrc).show
            }

            if let Some(t) = &self.sized_tx {
                ui.image(*t);
            }
        });
    }
}
