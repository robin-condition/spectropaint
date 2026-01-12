use std::error::Error;

use eframe::{AppCreator, NativeOptions};
use gui::app::SpectrogramApp;

fn main() -> Result<(), eframe::Error> {
    eframe::run_native(
        "Spectropaint",
        NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(SpectrogramApp::new(cc)))),
    )
}
