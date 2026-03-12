use std::{collections::HashSet, fs::File, io::Read};

use glam::{Vec2, vec2};
use rand::Rng;

use crate::{midi_parse::parse_midi, state::Mesh};

#[derive(Debug)]
struct Rect {
	min: Vec2, max: Vec2
}
impl Rect {
	fn union(&self, other: &Self) -> Self {
		Self { min: self.min.min(other.min), max: self.max.max(other.max) }
	}
	
	// fn intersects(&self, other: &Self) -> bool {
	// 	self.min.x < other.max.x && self.max.x > other.min.x && self.min.y < other.max.y && self.max.y > other.min.y
	// }

	fn centered(x: f32, y: f32, width: f32, height: f32) -> Self {
		Self { min: vec2(x-width*0.5, y-height*0.5), max: vec2(x+width*0.5, y+height*0.5) }
	}

	fn sweep(&self, velocity: Vec2, other: &Rect) -> bool {
		let entry = (Vec2::select(velocity.cmpgt(Vec2::ZERO), other.min - self.max, other.max - self.min) / velocity).max_element();
		let exit = (Vec2::select(velocity.cmpgt(Vec2::ZERO), other.max - self.min, other.min - self.max) / velocity).min_element();
		entry < exit && entry < 1.0 && exit > 0.0
	}
}

#[derive(Debug)]
struct Bounce {
	pos: Vec2,
	dir: Vec2,
	time: f32,
	vertical: bool,
}

impl Bounce {
	fn get_rect(&self) -> Rect {
		if self.vertical {
			if self.dir.x < 0.0 {
				Rect::centered(self.pos.x+20.0, self.pos.y, 8.0, 32.0)
			} else {
				Rect::centered(self.pos.x-20.0, self.pos.y, 8.0, 32.0)
			}
		} else {
			if self.dir.y < 0.0 {
				Rect::centered(self.pos.x, self.pos.y+20.0, 32.0, 8.0)
			} else {
				Rect::centered(self.pos.x, self.pos.y-20.0, 32.0, 8.0)
			}
		}
	}
	fn get_square_rect(&self) -> Rect {
		Rect::centered(self.pos.x, self.pos.y, 32.0, 32.0)
	}
}
impl Default for Bounce {
	fn default() -> Self {
		Self { pos: Vec2::ZERO, dir: Vec2::ONE, time: 0.0, vertical: false }
	}
}

#[derive(Debug)]
pub struct Square {
	pub pos: Vec2,
	pub dir: Vec2,
	pub squish: Vec2, // only display size multiplayer
	squish_velocity: Vec2,
	time: f32,
	pub next_note: usize,
}

impl Square {
	fn get_rect(&self) -> Rect {
		Rect::centered(self.pos.x, self.pos.y, 32.0, 32.0)
	}
}
impl Default for Square {
	fn default() -> Self {
		Self { pos: Vec2::ZERO, dir: Vec2::ONE, squish: Vec2::ONE, squish_velocity: Vec2::ZERO, time: 0.0, next_note: 0 }
	}
}

pub struct World {
	pub camera: Vec2,
	pub square: Square,
	bounces: Vec<Bounce>,
	areas: Vec<Rect>,
	pub started: bool,
}

impl World {
	pub fn generate_from_times(times: &[f32]) -> Self {
		let mut rng = rand::rng();

		let mut bounces: Vec<Bounce> = Vec::new();

		let mut square = Square::default();
		let mut vertical = false;

		let change_dir_chance: f32 = 0.3;
		let square_speed: f32 = 600.0;

		let mut backtrace = false;
		let mut backtraced_indexes: HashSet<usize> = HashSet::new();

		'outer: while square.next_note < times.len() {
			// println!("{:?}, Vertical: {}, Len: {}, Backtrace: {backtrace}", square, vertical, bounces.len());
			if backtrace {
				// prevents getting stuck in an infinite loop
				while !bounces.is_empty() && backtraced_indexes.contains(&(bounces.len()-1)) {
					backtraced_indexes.remove(&(bounces.len()-1));
					bounces.pop();
					square.next_note -= 1;
				}

				// remove the last bounce to re-generate it in the new orientation
				let last = bounces.pop().unwrap_or_default();
				square.pos = last.pos;
				square.dir = last.dir;
				square.time = last.time;
				vertical = !last.vertical; // Flip the direction to try new path

				// I added this
				if !vertical {square.dir.x = -square.dir.x}
				else {square.dir.y = -square.dir.y}

				backtrace = false;
				backtraced_indexes.insert(bounces.len());
				square.next_note -= 1;
			}

			let time = times[square.next_note];

			if !backtraced_indexes.contains(&bounces.len()) && rng.random::<f32>() < change_dir_chance { vertical = !vertical }
			
			let velocity = square.dir * square_speed * (time - square.time);
			// ensure the square will not pass through a bounce
			for bounce in &bounces {
				if square.get_rect().sweep(velocity, &bounce.get_rect()) {
					backtrace = true;
					continue 'outer;
				}
			}
			square.pos += velocity;
			square.time = time;

			if vertical {square.dir.x = -square.dir.x}
			else {square.dir.y = -square.dir.y}

			let bounce = Bounce { pos: square.pos, dir: square.dir, time: time, vertical };
			// ensure the square hasn't passed through the bounce
			for bounce_index in 0..bounces.len() {
				let first = if bounce_index == 0 {&Bounce::default()} else {&bounces[bounce_index - 1]};
				let last = &bounces[bounce_index];
				if first.get_square_rect().sweep(last.pos - first.pos, &bounce.get_rect()) {
					backtrace = true;
					continue 'outer;
				}
			}
			bounces.push(bounce);

			square.next_note += 1;
		}

		let mut areas: Vec<Rect> = Vec::new();
		for bounce_index in 0..bounces.len() {
			let first = if bounce_index == 0 {&Bounce::default()} else {&bounces[bounce_index - 1]};
			let last = &bounces[bounce_index];
			areas.push(first.get_square_rect().union(&last.get_square_rect()));
		}

		Self { areas, square: Square::default(), camera: bounces.last().unwrap().pos, bounces, started: false }
	}
	pub fn generate_from_floats_file(file_path: &str) -> Self {
		let mut file = File::open(file_path).unwrap();
		let mut buffer = Vec::new();
		file.read_to_end(&mut buffer).unwrap();
		let times: &[f32] =
		if file_path.ends_with(".mid") || file_path.ends_with(".midi") {
			&parse_midi(&buffer)
		} else if file_path.ends_with(".bin") {
			bytemuck::cast_slice(&buffer)
		} else {
			panic!("Unsupported file type")
		};
		Self::generate_from_times(times)
	}

	pub fn create_mesh(&self) -> Mesh {
		let mut mesh = Mesh::new();
		for rect in &self.areas {
			let base = mesh.get_num_vertices() as u16;
			mesh.add_vertex(rect.min.x, rect.max.y, 0.5);
			mesh.add_vertex(rect.min.x, rect.min.y, 0.5);
			mesh.add_vertex(rect.max.x, rect.min.y, 0.5);
			mesh.add_vertex(rect.max.x, rect.max.y, 0.5);
			mesh.add_triangle(base+0, base+1, base+2);
			mesh.add_triangle(base+2, base+3, base+0);
		}
		for bounce in &self.bounces {
			let rect = bounce.get_rect();
			let base = mesh.get_num_vertices() as u16;
			mesh.add_vertex(rect.min.x, rect.max.y, 0.25);
			mesh.add_vertex(rect.min.x, rect.min.y, 0.25);
			mesh.add_vertex(rect.max.x, rect.min.y, 0.25);
			mesh.add_vertex(rect.max.x, rect.max.y, 0.25);
			mesh.add_triangle(base+0, base+1, base+2);
			mesh.add_triangle(base+2, base+3, base+0);
		}
		mesh
	}

	pub fn update(&mut self, delta: f32) {
		self.square.squish_velocity += (Vec2::ONE - self.square.squish) * 600.0 * delta;
		self.square.squish_velocity *= (-6.0 * delta).exp();
		self.square.squish += self.square.squish_velocity * delta;

		if self.started && self.square.next_note < self.bounces.len() {
			self.square.pos  += self.square.dir * 600.0 * delta;
			self.square.time += delta;
			let next_bounce = &self.bounces[self.square.next_note];
			if self.square.time >= next_bounce.time {
				self.square.pos = next_bounce.pos;
				self.square.dir = next_bounce.dir;
				self.square.next_note += 1;
				if next_bounce.vertical {
					self.square.squish_velocity += vec2(-8.0, 6.0);
				} else {
					self.square.squish_velocity += vec2(6.0, -8.0);
				}
			}
		}

		self.camera += (self.square.pos - self.camera) * 3.0 * delta;
	}

	pub fn reset(&mut self) {
		// self.camera = self.bounces.last().unwrap().pos;
		self.square = Square::default();
		self.started = false;
	}
}