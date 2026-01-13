use std::{error::Error, path::PathBuf, thread, time::Duration};

use eframe::{App, CreationContext};
use egui::{TextureHandle, TextureOptions, Vec2, load::SizedTexture};
use egui_file_dialog::FileDialog;
use image::{EncodableLayout, ImageBuffer, Luma};
use rodio::{OutputStream, Source, buffer::SamplesBuffer};

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

        let window_size = 3000;
        //let mut audio = rodio::Decoder::try_from(fs).unwrap();
        let mut audio = rodio::source::SineWave::new(440f32).take_duration(Duration::new(100, 0));
        let channels = audio.channels();
        let sr = audio.sample_rate();
        println!("{}", channels);
        let samples: Vec<_> = audio.step_by(channels as usize).collect();
        self.samples = samples;
        let mut res =
            spectrogram::forward::analyze_mt::<u8>(&self.samples, window_size, 15).unwrap();
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

        let img_buffer =
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
        img_buffer
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
