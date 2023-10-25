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
    midi_key_tracks: Vec<MidiKeyTrack>,
    key_to_keyboard_mapping: HashMap<u8, String>,
    program_to_string_mapping: HashMap<u8, String>,
}

type PlanckRows = Vec<Vec<String>>;
const MIDI_C_KEY: u8 = 60;

struct MidiKeyTrack {
    name: String,
    midi_key_pairs: Vec<MidiKeyPair>,
}

impl MidiKeyTrack {
    fn new() -> Self {
        MidiKeyTrack {
            name: String::new(),
            midi_key_pairs: Vec::new(),
        }
    }
}

impl MidiKeyTrack {
    fn get_midi_keys_text(&self) -> String {
        let mut midi_keys_text = String::new();
        for pair in self.midi_key_pairs.iter() {
            let keyboard_key = match pair.keyboard_key.clone() {
                Some(key) => key,
                None => "NONE".to_owned(),
            };
            midi_keys_text += &format!("\n{}  ({})", pair.midi_key, keyboard_key);
        }
        midi_keys_text
    }
}

struct MidiKeyPair {
    midi_key: u7,
    keyboard_key: Option<String>,
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

fn program_to_string() -> HashMap<u8, String> {
    let mappings = [
        (0, "Piano"),
        (1, "Acoustic Grand Piano or Piano 1"),
        (2, "Bright Acoustic Piano or Piano 2"),
        (3, "Electric Grand Piano or Piano 3 (usually modeled after Yamaha CP70)"),
        (4, "Honky-tonk Piano"),
        (5, "Electric Piano 1 (usually a Rhodes piano)"),
        (6, "Electric Piano 2 (usually an FM piano patch)"),
        (7, "Harpsichord"),
        (8, "Clavinet"),
        // Chromatic Percussion
        (9, "Celesta"),
        (10, "Glockenspiel"),
        (11, "Music Box"),
        (12, "Vibraphone"),
        (13, "Marimba"),
        (14, "Xylophone"),
        (15, "Tubular Bells"),
        (16, "Dulcimer or Santoor"),
        // Organ
        (17, "Drawbar Organ or Organ 1"),
        (18, "Percussive Organ or Organ 2"),
        (19, "Rock Organ or Organ 3"),
        (20, "Church Organ"),
        (21, "Reed Organ"),
        (22, "Accordion"),
        (23, "Harmonica"),
        (24, "Bandoneon or Tango Accordion"),
        // Guitar
        (25, "Acoustic Guitar (nylon)"),
        (26, "Acoustic Guitar (steel)"),
        (27, "Electric Guitar (jazz)"),
        (28, "Electric Guitar (clean, usually resembling a Fender Stratocaster ran through a Roland Jazz Chorus amp)"),
        (29, "Electric Guitar (muted)"),
        (30, "Electric Guitar (overdriven)"),
        (31, "Electric Guitar (distortion)"),
        (32, "Electric Guitar (harmonics)"),
        // Bass
        (33, "Acoustic Bass"),
        (34, "Electric Bass (finger)"),
        (35, "Electric Bass (picked)"),
        (36, "Electric Bass (fretless)"),
        (37, "Slap Bass 1"),
        (38, "Slap Bass 2"),
        (39, "Synth Bass 1"),
        (40, "Synth Bass 2"),
        // Strings
        (41, "Violin"),
        (42, "Viola"),
        (43, "Cello"),
        (44, "Contrabass"),
        (45, "Tremolo Strings"),
        (46, "Pizzicato Strings"),
        (47, "Orchestral Harp"),
        (48, "Timpani"),
        // Ensemble
        (49, "String Ensemble 1"),
        (50, "String Ensemble 2"),
        (51, "Synth Strings 1"),
        (52, "Synth Strings 2"),
        (53, "Choir Aahs"),
        (54, "Voice Oohs (or Doos)"),
        (55, "Synth Voice or Synth Choir"),
        (56, "Orchestra Hit"),
        // Brass
        (57, "Trumpet"),
        (58, "Trombone"),
        (59, "Tuba"),
        (60, "Muted Trumpet"),
        (61, "French Horn"),
        (62, "Brass Section"),
        (63, "Synth Brass 1"),
        (64, "Synth Brass 2"),
        // Reed
        (65, "Soprano Sax"),
        (66, "Alto Sax"),
        (67, "Tenor Sax"),
        (68, "Baritone Sax"),
        (69, "Oboe"),
        (70, "English Horn"),
        (71, "Bassoon"),
        (72, "Clarinet"),
        // Pipe
        (73, "Piccolo"),
        (74, "Flute"),
        (75, "Recorder"),
        (76, "Pan Flute"),
        (77, "Blown bottle"),
        (78, "Shakuhachi"),
        (79, "Whistle"),
        (80, "Ocarina"),
        // Synth Lead
        (81, "Lead 1 (square, often chorused)"),
        (82, "Lead 2 (sawtooth, often chorused)"),
        (83, "Lead 3 (triangle, or calliope, usually resembling a woodwind)"),
        (84, "Lead 4 (sine, or chiff)"),
        (85, "Lead 5 (charang, a guitar-like lead)"),
        (86, "Lead 6 (voice)"),
        (87, "Lead 7 (fifths)"),
        (88, "Lead 8 (bass and lead or solo lead)"),
        // Synth Pad
        (89, "Pad 1 (new age, pad stacked with a bell, often derived from \"Fantasia\" patch from Roland D-50)"),
        (90, "Pad 2 (warm, a mellower saw pad)"),
        (91, "Pad 3 (polysynth or poly, a saw-like percussive pad resembling an early 1980s polyphonic synthesizer)"),
        (92, "Pad 4 (choir, similar to \"synth voice\")"),
        (93, "Pad 5 (bowed glass or bowed, a sound resembling a glass harmonica)"),
        (94, "Pad 6 (metallic, often created from a grand piano sample played with the attack removed)"),
        (95, "Pad 7 (halo, choir-like pad)"),
        (96, "Pad 8 (sweep, pad with a pronounced \"wah\" filter effect)"),
        // Synth Effects
        (97, "FX 1 (rain, a bright pluck with echoing pulses)"),
        (98, "FX 2 (soundtrack, a bright perfect fifth pad)"),
        (99, "FX 3 (crystal, a synthesized bell sound)"),
        (100, "FX 4 (atmosphere, usually a classical guitar-like sound)"),
        (101, "FX 5 (brightness, a fast-attack stacked pad with choir or bell)"),
        (102, "FX 6 (goblins, a slow-attack pad with chirping or murmuring sounds)"),
        (103, "FX 7 (echoes or echo drops, similar to \"rain\")"),
        (104, "FX 8 (sci-fi or star theme, usually an electric guitar-like pad)"),
        // Ethnic
        (105, "Sitar"),
        (106, "Banjo"),
        (107, "Shamisen"),
        (108, "Koto"),
        (109, "Kalimba"),
        (110, "Bag pipe"),
        (111, "Fiddle"),
        (112, "Shanai"),
        // Percussive
        (113, "Tinkle Bell"),
        (114, "Agogô or cowbell"),
        (115, "Steel Drums"),
        (116, "Woodblock"),
        (117, "Taiko Drum"),
        (118, "Melodic Tom or 808 Toms"),
        (119, "Synth Drum"),
        (120, "Reverse Cymbal"),
        // Sound Effects
        (121, "Guitar Fret Noise"),
        (122, "Breath Noise"),
        (123, "Seashore"),
        (124, "Bird Tweet"),
        (125, "Telephone Ring"),
        (126, "Helicopter"),
        (127, "Applause"),
        (128, "Gunshot"),
    ];
    mappings
        .into_iter()
        .map(|x| (x.0, x.1.to_string()))
        .collect()
}

impl Default for MyApp {
    fn default() -> Self {
        MyApp {
            program_to_string_mapping: program_to_string(),
            picked_midi_path: None,
            midi_key_tracks: Vec::new(),
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

        self.midi_key_tracks.clear();
        let mut channel_num: u32 = 1;
        for track in parsed_midi.tracks {
            let mut midi_key_track = MidiKeyTrack::new();
            midi_key_track.name = format!("Channel {}", channel_num);
            for note in track {
                if let midly::TrackEventKind::Midi {
                    channel: _,
                    message,
                } = note.kind
                {
                    match message {
                        midly::MidiMessage::NoteOn { key, vel: _ } => {
                            let keyboard_key = self
                                .key_to_keyboard_mapping
                                .get(&key.into())
                                .and_then(|key| Some(key.to_string()));
                            let pair = MidiKeyPair {
                                midi_key: key,
                                keyboard_key: keyboard_key.clone(),
                            };
                            midi_key_track.midi_key_pairs.push(pair);
                        }
                        midly::MidiMessage::ProgramChange { program } => {
                            if let Some(name) =
                                self.program_to_string_mapping.get(&program.as_int())
                            {
                                midi_key_track.name = name.clone()
                            }
                        }
                        _ => (),
                    }
                }
                channel_num += 1;
            }
            self.midi_key_tracks.push(midi_key_track)
        }

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

                    if self.midi_key_tracks.len() > 0 {
                        ui.add_space(16.0);
                        egui::ScrollArea::new([false, true])
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for track in self.midi_key_tracks.iter() {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(format!("{}:", track.name));
                                        ui.monospace(track.get_midi_keys_text());
                                    });
                                }
                            });
                    }
                });
            });

        self.preview_hovering_files(ctx);
        self.collect_dropped_files(ctx);
    }
}
