use std::sync::Arc;

use clap::Parser;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::Key;
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::window::{Window, WindowId};

mod midi_parse;
mod state;
mod world;
use state::State;

#[derive(clap::Parser, Clone)]
pub struct Args {
	#[arg(short, long)]
	midi_path: String,
	#[arg(short, long)]
	audio_path: String,
	#[arg(short, long, default_value_t = 0)]
	seed: u64,
	#[arg(short = 'q', long, default_value_t = 600.0)]
	square_speed: f32,
}

struct App {
	state: Option<State>,

	args: Args,
}

impl ApplicationHandler for App {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		if self.args.midi_path == "" || self.args.audio_path == "" {
			panic!("no midi or audio path chosen"); // should be unreachable
		} else {
			self.state = Some(pollster::block_on(State::new(
				Arc::new(event_loop.create_window(Window::default_attributes().with_title("Midi Bounce")).unwrap()),
				self.args.clone(),
			)));
		}
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
		if let Some(state) = &mut self.state {
			match event {
				WindowEvent::CloseRequested => event_loop.exit(),
				WindowEvent::Resized(size) => state.resize(size.width, size.height),
				WindowEvent::RedrawRequested => state.render(),
				WindowEvent::KeyboardInput { event, .. } => {
					if !event.repeat
						&& let Key::Character(character) = event.key_without_modifiers()
						&& character.chars().count() == 1
					{
						state.keypress(character.chars().next().unwrap() /* safe to use unwrap here because it is already checked */)
					}
				}
				_ => (),
			}
		}
	}
}

fn main() {
	let event_loop = EventLoop::new().unwrap();
	event_loop.set_control_flow(ControlFlow::Poll);
	event_loop.set_control_flow(ControlFlow::Wait);

	let args = Args::parse();

	// midi_path = rfd::FileDialog::new()
	// 	.set_title("Choose midi file")
	// 	.add_filter("Midi Files", &["mid", "midi"])
	// 	.pick_file()
	// 	.expect("You didn't choose a MIDI file!!!")
	// 	.to_str()
	// 	.unwrap()
	// 	.to_string();
	// audio_path = rfd::FileDialog::new()
	// 	.set_title("Choose audio file")
	// 	.add_filter("Audio Files", &["ogg", "flac", "wav", "mp3", "mp4"])
	// 	.pick_file()
	// 	.expect("You didn't choose an Audio file!!!")
	// 	.to_str()
	// 	.unwrap()
	// 	.to_string();

	let mut app = App { state: None, args };
	event_loop.run_app(&mut app).unwrap();
}
