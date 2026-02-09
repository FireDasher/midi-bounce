// use std::fs;

// use midly::{Smf, Timing};

// pub fn parse_midi(path: &str) -> Vec<f32> {
// 	let smf = Smf::parse(&fs::read(path).unwrap()).unwrap();
// 	let ticks_per_beat = match smf.header.timing {
// 		Timing::Metrical(t) => t.as_int() as u32,
// 		_ => unreachable!(),
// 	};
// 	let mut tempos: Vec<(u32, u32)> = Vec::new();
// 	for track in &smf.tracks {
// 		let mut ticks: u32 = 0;

// 	}
// }