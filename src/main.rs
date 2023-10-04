#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{
    egui::{self, RichText},
    epaint::Color32,
};
use midly::{self, num::u7};
use std::{collections::HashMap, fmt::*, fs, result::Result};
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

struct MyApp {
    picked_midi_path: Option<String>,
    midi_key_pairs: Vec<Option<MidiKeyPair>>,
    key_to_keyboard_mapping: HashMap<u8, String>,
}

type PlanckRows = Vec<Vec<String>>;
const MIDI_C_KEY: u8 = 60;

struct MidiKeyPair {
    midi_key: u7,
    keyboard_key: String,
}

fn default_planck_rows() -> PlanckRows {
    [
        [
            "TAB", "Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "BCK",
        ],
        ["ESC", "A", "S", "D", "F", "G", "H", "J", "K", "L", ";", "'"],
        [
            "SHF", "Z", "X", "C", "V", "B", "N", "M", ",", ".", "/", "ETR",
        ],
        [
            "", "CTRL", "ALT", "ORYX", "OS", "SHFDOWN", "SPACE", "SHFUP", "<-", "\\/", "/\\", "->",
        ],
    ]
    .map(|row| row.map(|key| key.to_owned()).into_iter().collect())
    .into_iter()
    .collect()
}

fn chromatic_planck_mapping(base_key: &str, rows: PlanckRows) -> HashMap<u8, String> {
    let mut base_index: i32 = 0;
    let mut found_base_key = false;
    'outer: for row in rows.iter() {
        for key in row {
            if key == base_key {
                found_base_key = true;
                break 'outer;
            }
            base_index += 1;
        }
    }
    if !found_base_key {
        panic!("Expected base key to exist")
    }

    let mut key_to_keyboard_mapping = HashMap::new();
    let mut index: i32 = 0;
    for row in rows.iter() {
        for keyboard_key in row {
            let midi_key_i32 = MIDI_C_KEY as i32 + (index - base_index);
            if let Ok(key_u8) = midi_key_i32.try_into() {
                key_to_keyboard_mapping.insert(key_u8, keyboard_key.clone());
            }
            index += 1;
        }
    }
    key_to_keyboard_mapping
}

impl Default for MyApp {
    fn default() -> Self {
        MyApp {
            picked_midi_path: None,
            midi_key_pairs: Vec::new(),
            key_to_keyboard_mapping: chromatic_planck_mapping("ESC", default_planck_rows()),
        }
    }
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
                    let _ = self.load_midi_file(path);
                }
            }
        });
    }

    // C  -> key = 60 + 0
    // C# -> key = 60 + 1
    // D  -> key = 60 + 2
    // D# -> key = 60 + 3
    // E  -> key = 60 + 4
    // F  -> key = 60 + 5
    // F# -> key = 60 + 6
    // G  -> key = 60 + 7
    // G# -> key = 60 + 8
    // A  -> key = 60 + 9
    // A# -> key = 60 + 10
    // B  -> key = 60 + 11

    fn load_midi_file(&mut self, path: String) -> Result<(), LoadMidiFileError> {
        self.picked_midi_path = Some(path.clone());
        let file = fs::read(path)?;
        let parsed_midi = midly::Smf::parse(&file)?;
        let first_track = parsed_midi
            .tracks
            .first()
            .ok_or(LoadMidiFileError::NoTrackError)?;

        self.midi_key_pairs.clear();
        for note in first_track {
            if let midly::TrackEventKind::Midi {
                channel: _,
                message,
            } = note.kind
            {
                if let midly::MidiMessage::NoteOn { key, vel: _ } = message {
                    let pair =
                        self.key_to_keyboard_mapping
                            .get(&key.into())
                            .and_then(|keyboard_key| {
                                Some(MidiKeyPair {
                                    midi_key: key,
                                    keyboard_key: keyboard_key.clone(),
                                })
                            });
                    self.midi_key_pairs.push(pair);
                }
            }
        }

        Ok(())
    }

    fn get_midi_keys_text(&self) -> String {
        let mut midi_keys_text = String::new();
        for opt_pair in self.midi_key_pairs.iter() {
            let pair_text = match opt_pair {
                Some(pair) => format!("\n{}  ({})", pair.keyboard_key, pair.midi_key),
                None => "\nERROR".to_owned(),
            };
            midi_keys_text += &pair_text;
        }
        midi_keys_text
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
                            let _ = self.load_midi_file(path.display().to_string());
                        }
                    }

                    if let Some(picked_midi_path) = &self.picked_midi_path {
                        ui.add_space(16.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Picked file:");
                            ui.monospace(picked_midi_path);
                        });
                    }

                    if self.midi_key_pairs.len() > 0 {
                        ui.add_space(16.0);
                        egui::ScrollArea::new([false, true]).show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Notes:");
                                ui.monospace(self.get_midi_keys_text());
                            });
                        });
                    }
                });
            });

        self.preview_hovering_files(ctx);
        self.collect_dropped_files(ctx);
    }
}
