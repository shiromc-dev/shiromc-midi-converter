use clap::Parser;
use midi_toolkit::{
	events::{Event, NoteOnEvent},
	io::MIDIFile,
	pipe,
	sequence::{
		event::{cancel_tempo_events, merge_events_array, scale_event_time},
		to_vec, unwrap_items, TimeCaster,
	},
};
use serde::ser::SerializeStruct;
use serde::Serialize;
use std::path::Path;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Serialize)]
struct SongData {
	maxtick: u32,
	pitchint: u32,
	title: String,
	data: BTreeMap<u32, Vec<NoteOnSerializer>>,
}

impl Serialize for NoteOnSerializer {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let mut state = serializer.serialize_struct("NoteOnSerializer", 3)?;
		state.serialize_field("channel", &self.inner.channel)?;
		state.serialize_field("note", &self.inner.key)?;
		state.serialize_field("velocity", &self.inner.velocity)?;
		state.end()
	}
}

struct NoteOnSerializer {
	inner: NoteOnEvent,
}

#[derive(Parser)]
#[command(name = "MIDI to JSON Converter")]
struct Args {
	/// Path to the MIDI file to convert
	#[arg(value_name = "MIDI_FILE", required = true)]
	midi_file: PathBuf,
	/// File name of converted json
	#[arg(short = 't', long = "title", required = true)]
	title: String,
}

fn main() {
	// Parse command-line arguments into the Args struct
	let args = Args::parse();

	let path = &args.midi_file;

	let midi = MIDIFile::open(path, None).unwrap();

	let ppq = midi.ppq();
	let merged = pipe!(
		midi.iter_all_tracks()
		|>to_vec()
		|>merge_events_array()
		|>TimeCaster::<f64>::cast_event_delta()
		|>cancel_tempo_events(250000)
		|>scale_event_time(1.0 / ppq as f64)
		|>unwrap_items()
	);

	let mut time = 0.0;

	// Extract filename without extension from the path
	let filename = Path::new(path).file_stem().unwrap().to_str().unwrap();

	let mut song_data = SongData {
		maxtick: 0,
		pitchint: 0,
		title: args.title,
		data: BTreeMap::new(),
	};

	for e in merged {
		if e.delta != 0.0 {
			time += e.delta;
		}

		// Convert time to Minecraft ticks (20 ticks = 1 second)
		let tick_key = (time * 20.0).round() as u32; // Multiply by 20 and round to nearest Minecraft tick

		// Process NoteOn events and store them in the event map
		match e.event {
			Event::NoteOn(evt) => {
				let serializer = NoteOnSerializer { inner: evt };
				song_data
					.data
					.entry(tick_key)
					.or_insert_with(Vec::new)
					.push(serializer);
			}
			_ => {}
		}
	}

	song_data.maxtick = (time * 20.0).round() as u32;

	// Conditional compilation for pretty printing based on build profile
	#[cfg(debug_assertions)]
	let json_data = serde_json::to_string_pretty(&song_data).unwrap();

	#[cfg(not(debug_assertions))]
	let json_data = serde_json::to_string(&song_data).unwrap();

	// Save to file
	std::fs::write(
		format!(
			"C:/Users/kazu/Desktop/File/Rust/shiromc-midi-converter/{}.json",
			filename
		),
		json_data,
	)
	.expect("Unable to write file");
}
