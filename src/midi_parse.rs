use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

enum EventKind {
	Note,
	TempoChange(u32),
}

// Returns a list of seconds where note on events occured, and remove duplicates
pub fn parse_midi(bytes: &[u8]) -> Vec<f32> {
	let smf = Smf::parse(bytes).unwrap();

	// Merge tracks
	let mut events = Vec::new();
	for track in &smf.tracks {
		let mut now = 0;
		for event in track {
			now += event.delta.as_int();
			if let TrackEventKind::Midi{message: MidiMessage::NoteOn {vel, ..}, ..} = event.kind && vel > 0 {
				events.push((now, EventKind::Note));
			} else if let TrackEventKind::Meta(MetaMessage::Tempo(tempo)) = event.kind {
				events.push((now, EventKind::TempoChange(tempo.as_int())));
			}
		}
	}
	events.sort_by_key(|e|e.0);

	// Metrical is the format use by most midi files
	let Timing::Metrical(ticks_per_beat) = smf.header.timing else {panic!("Timecode midis not supported only metrical")};
	let ticks_per_beat = ticks_per_beat.as_int(); // Convert to a normal integer instead of a weird wrapper
	let mut times = Vec::new();
	let mut tempo: u32 = 500000; // 120 BPM is the default by midi standard
	let mut now: f32 = 0.0;
	let mut last_delta: u32 = 0; // Used to convert notes back to relative time before converting to absolute seconds

	for event in &events {
		if event.0 > last_delta || times.is_empty() { // ensure no duplicates
			now += (event.0 - last_delta) as f32 * (tempo as f32 * 1e-6 / ticks_per_beat as f32);
			last_delta = event.0;
			if let EventKind::Note = event.1 {
				times.push(now);
			}
		}
		if let EventKind::TempoChange(new_tempo) = event.1 {
			tempo = new_tempo;
		}
	}

	times
}