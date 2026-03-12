use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::Key;
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::window::{Window, WindowId};

mod state;
mod world;
mod midi_parse;
use state::State;

struct App {
    state: Option<State>,
}

impl App {
	fn new() -> Self {
		Self { state: None }
	}
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.state = Some(pollster::block_on(
			State::new(Arc::new(
				event_loop.create_window(Window::default_attributes().with_title("Midi Bounce")).unwrap()
			))
		));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
		if let Some(state) = &mut self.state {
			match event {
				WindowEvent::CloseRequested => event_loop.exit(),
				WindowEvent::Resized(size) => state.resize(size.width, size.height),
				WindowEvent::RedrawRequested => state.render(),
				WindowEvent::KeyboardInput { event, .. } => if !event.repeat && let Key::Character(character) = event.key_without_modifiers() && character.chars().count() == 1 {state.keypress(character.chars().next().unwrap() /* safe to use unwrap here because it is already checked */)}
				_ => (),
			}
		}
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}