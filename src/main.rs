#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{
    egui::{self, RichText, Widget},
    epaint::Color32,
};
use midly;
use std::{fmt::*, fs, io, result::Result};
use thiserror::*;

// TODO: Add custom icon
// https://github.com/rust-windowing/winit/blob/master/examples/window_icon.rs

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        drag_and_drop_support: true,
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Planck Scribe",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

#[derive(Default)]
struct MyApp {
    picked_midi_path: Option<String>,
    midi_text: Option<String>,
}

impl MyApp {
    /// Preview hovering files:
    fn preview_hovering_files(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            let text = ctx.input(|i| {
                let mut text = "Dropping files:\n".to_owned();
                for file in &i.raw.hovered_files {
                    if let Some(path) = &file.path {
                        write!(text, "\n\n{}", path.display()).ok();
                    } else if !file.mime.is_empty() {
                        write!(text, "\n\n{}", file.mime).ok();
                    } else {
                        text += "\n\n???";
                    }
                }
                text
            });

            egui::CentralPanel::default()
                .frame(
                    egui::Frame::default()
                        .inner_margin(16.0)
                        .fill(Color32::from_black_alpha(192)),
                )
                .show(ctx, |ui| {
                    ui.label(text);
                });
        }
    }

    fn collect_dropped_files(&mut self, ctx: &egui::Context) {
        // Collect dropped files:
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                if let Some(path) = i
                    .raw
                    .dropped_files
                    .clone()
                    .into_iter()
                    .filter_map(|x| x.path.clone())
                    .filter_map(|path| path.to_str().and_then(|x| Some(x.to_string())))
                    .filter(|x| x.ends_with(".mid") || x.ends_with(".midi"))
                    .nth(0)
                {
                    self.load_midi_file(path);
                }
            }
        });
    }

    fn load_midi_file(&mut self, path: String) -> Result<(), LoadMidiFileError> {
        self.picked_midi_path = Some(path.clone());
        let midi_text = String::new();
        let file = fs::read(path)?;
        let parsed_midi = midly::Smf::parse(&file)?;
        let first_track = parsed_midi
            .tracks
            .first()
            .ok_or(LoadMidiFileError::NoTrackError)?;
        for note in first_track {
            match note.kind {
                midly::TrackEventKind::Midi { channel, message } => match message {
                    midly::MidiMessage::NoteOn { key, vel } => {} // TODO: Finish this
                },
                _ => {}
            }
        }
        self.midi_text = Some(midi_text);

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum LoadMidiFileError {
    #[error("io error: {0}")]
    IOError(std::io::Error),
    #[error("midly error: {0}")]
    MidlyError(midly::Error),
    #[error("no track exists")]
    NoTrackError,
}

impl From<std::io::Error> for LoadMidiFileError {
    fn from(value: std::io::Error) -> Self {
        LoadMidiFileError::IOError(value)
    }
}

impl From<midly::Error> for LoadMidiFileError {
    fn from(value: midly::Error) -> Self {
        LoadMidiFileError::MidlyError(value)
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::default().inner_margin(16.0))
            .show(ctx, |ui| {
                ui.style_mut().spacing.button_padding = egui::vec2(16.0, 8.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Planck Scribe 🎹").heading().size(32.0));
                    ui.add_space(16.0);
                    ui.label("Drag-and-drop MIDI files onto the window!");

                    if ui.button("Open MIDI file…").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("midi", &["mid", "midi"])
                            .pick_file()
                        {
                            self.load_midi_file(path.display().to_string());
                        }
                    }

                    if let Some(picked_midi_path) = &self.picked_midi_path {
                        ui.add_space(16.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Picked file:");
                            ui.monospace(picked_midi_path);
                        });
                    }

                    if let Some(midi_text) = &self.midi_text {
                        ui.add_space(16.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Notes:");
                            ui.monospace(midi_text);
                        });
                    }
                });
            });

        self.preview_hovering_files(ctx);
        self.collect_dropped_files(ctx);
    }
}
