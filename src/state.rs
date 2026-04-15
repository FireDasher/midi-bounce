use std::{fs::File, sync::Arc, time::Instant};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::world::World;

pub struct Mesh {
	vertices: Vec<Vertex>,
	indicies: Vec<u16>,
}
impl Mesh {
	pub fn new() -> Self {
		Self { vertices: Vec::new(), indicies: Vec::new() }
	}

	pub fn add_vertex(&mut self, x: f32, y: f32, shade: f32) {
		self.vertices.push(Vertex { x, y, shade });
	}
	pub fn add_triangle(&mut self, a: u16, b: u16, c: u16) {
		self.indicies.push(a);
		self.indicies.push(b);
		self.indicies.push(c);
	}

	pub fn get_num_vertices(&self) -> usize {
		self.vertices.len()
	}

	fn create_buffers(&self, device: &wgpu::Device) -> MeshBuffers {
		let vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: None,
			contents: bytemuck::cast_slice(&self.vertices),
			usage: wgpu::BufferUsages::VERTEX,
		});
		let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: None,
			contents: bytemuck::cast_slice(&self.indicies),
			usage: wgpu::BufferUsages::INDEX,
		});
		MeshBuffers { vertex, index, len: self.indicies.len() as u32 }
	}
}

struct MeshBuffers {
	vertex: wgpu::Buffer,
	index: wgpu::Buffer,
	len: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
	x: f32, y: f32,
	shade: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Default)]
struct Uniforms {
	matrix: [f32; 8],
	color: [f32; 4],
}

impl Vertex {
	const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32];
	fn desc() -> wgpu::VertexBufferLayout<'static> {
		use std::mem;
		wgpu::VertexBufferLayout {
			array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Vertex,
			attributes: &Self::ATTRIBS,
		}
	}
}

struct UniformBuffers {
	buffer: wgpu::Buffer,
	bg: wgpu::BindGroup
}

pub struct State {
	window: Arc<Window>,
	surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
	render_pipeline: wgpu::RenderPipeline,

	map_buffer: MeshBuffers,
	square_buffer: MeshBuffers,

	map_uniforms: UniformBuffers,
	square_uniforms: UniformBuffers,

	world: World,
	last_time: Instant,

	_audio_stream: rodio::MixerDeviceSink,
	sink: rodio::Player, // sink is needed to stop audio

	midi_path: String, audio_path: String
}

impl State {
	pub async fn new(window: Arc<Window>) -> Self {
		// Command line arguments
		let arguments: Vec<String> = std::env::args().collect();
		if arguments.len() != 3 {
			panic!("\n//// ERROR: You need exactly two arguments, Midi File path and Audio path ////\n");
		}
		let midi_path = arguments[1].clone();
		let audio_path = arguments[2].clone();

		// Creating the surface
		let size = window.inner_size();

		let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { // why doesn't this implement Default anymore
			backends: wgpu::Backends::PRIMARY,
			flags: Default::default(),
			memory_budget_thresholds: Default::default(),
			backend_options: Default::default(),
			display: None,
		});
		let surface = instance.create_surface(window.clone()).unwrap();
		let adapter = instance.request_adapter(&wgpu::RequestAdapterOptionsBase {
			power_preference: wgpu::PowerPreference::default(),
			compatible_surface: Some(&surface),
			force_fallback_adapter: false,
		}).await.unwrap();

		let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
				label: None,
				required_features: wgpu::Features::empty(),
				experimental_features: wgpu::ExperimentalFeatures::disabled(),
				required_limits: wgpu::Limits::default(),
				memory_hints: Default::default(),
				trace: wgpu::Trace::Off,
		}).await.unwrap();

		let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter().find(|f| !f.is_srgb() /* srgb makes it look too bright */).copied().unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

		// bind group crap
		let uniforms_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: None,
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStages::VERTEX,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Uniform,
						has_dynamic_offset: false,
						min_binding_size: None,
					},
					count: None,
				}
			],
		});
		let uniform_buffer = device.create_buffer_init(
			&wgpu::util::BufferInitDescriptor {
				label: None,
				contents: bytemuck::cast_slice(&[Uniforms::default()]),
				usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
			}
		);
		let uniform_buffer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: None,
			layout: &uniforms_bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: uniform_buffer.as_entire_binding(),
				}
			],
		});
		let map_uniforms = UniformBuffers {buffer: uniform_buffer, bg: uniform_buffer_bind_group};
		let uniform_buffer = device.create_buffer_init(
			&wgpu::util::BufferInitDescriptor {
				label: None,
				contents: bytemuck::cast_slice(&[Uniforms::default()]),
				usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
			}
		);
		let uniform_buffer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: None,
			layout: &uniforms_bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: uniform_buffer.as_entire_binding(),
				}
			],
		});
		let square_uniforms = UniformBuffers {buffer: uniform_buffer, bg: uniform_buffer_bind_group};

		// Creating the pipeline
		let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
		let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: None,
			layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: None,
				bind_group_layouts: &[Some(&uniforms_bind_group_layout)],
				immediate_size: 0,
			})),
			vertex: wgpu::VertexState {
				module: &shader,
				entry_point: None,
				buffers: &[ Vertex::desc() ],
				compilation_options: wgpu::PipelineCompilationOptions::default()
			},
			fragment: Some(wgpu::FragmentState {
				module: &shader,
				entry_point: None,
				compilation_options: wgpu::PipelineCompilationOptions::default(),
				targets: &[Some(wgpu::ColorTargetState {
					format: config.format,
					blend: Some(wgpu::BlendState::REPLACE),
					write_mask: wgpu::ColorWrites::ALL
				})]
			}),
			primitive: wgpu::PrimitiveState {
				topology: wgpu::PrimitiveTopology::TriangleList,
				strip_index_format: None,
				front_face: wgpu::FrontFace::Ccw,
				cull_mode: None,
				polygon_mode: wgpu::PolygonMode::Fill,
				unclipped_depth: false,
				conservative: false
			},
			depth_stencil: None,
			multisample: wgpu::MultisampleState::default(),
			multiview_mask: None,
			cache: None
		});

		// World
		// let world = World::generate_from_times(&[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]);
		let world = World::generate_from_floats_file(&midi_path);

		// Creating the buffer
		let world_mesh = world.create_mesh();
		let map_buffer = world_mesh.create_buffers(&device);

		let mut square_mesh = Mesh::new();
		square_mesh.add_vertex(-16.0, 16.0, 0.0);
		square_mesh.add_vertex(-16.0, -16.0, 0.0);
		square_mesh.add_vertex(16.0, -16.0, 0.0);
		square_mesh.add_vertex(16.0, 16.0, 0.0);
		square_mesh.add_triangle(0, 1, 2);
		square_mesh.add_triangle(2, 3, 0);
		square_mesh.add_vertex(-12.0, 12.0, 1.0);
		square_mesh.add_vertex(-12.0, -12.0, 1.0);
		square_mesh.add_vertex(12.0, -12.0, 1.0);
		square_mesh.add_vertex(12.0, 12.0, 1.0);
		square_mesh.add_triangle(4, 5, 6);
		square_mesh.add_triangle(6, 7, 4);
		let square_buffer = square_mesh.create_buffers(&device);

		// audio
		let mut audio_stream = rodio::DeviceSinkBuilder::open_default_sink().expect("Failed to get audio handle, make sure you have headphones or speakers");
		audio_stream.log_on_drop(false);

		// finally create the thing
		Self {
			window, surface, device, queue, config, is_surface_configured: false, render_pipeline,
			map_buffer, square_buffer, map_uniforms, square_uniforms,
			world, last_time: Instant::now(),
			sink: rodio::Player::connect_new(&audio_stream.mixer()), _audio_stream: audio_stream,
			midi_path, audio_path
		}
	}
	pub fn resize(&mut self, width: u32, height: u32) {
		if width > 0 && height > 0 {
			self.config.width = width;
			self.config.height = height;
			self.surface.configure(&self.device, &self.config);
			self.is_surface_configured = true;
		}
	}
	pub fn keypress(&mut self, key: char) {
		match key {
			'r' => {self.world.reset(); self.sink.stop();}, // resets the square
			'g' => {self.world = World::generate_from_floats_file(&self.midi_path); self.map_buffer = self.world.create_mesh().create_buffers(&self.device); self.sink.stop();}, // regenerates the world
			_ => ()
		}
	}
	pub fn render(&mut self) {
		self.window.request_redraw();

		// update //
		let now = Instant::now();
		let delta = now.duration_since(self.last_time).as_secs_f32();
		self.world.update(delta);
		self.last_time = now;

		// start
		if !self.world.started && self.world.camera.distance_squared(self.world.square.pos) < 1.0 {
			let source = rodio::Decoder::try_from(File::open(&self.audio_path).unwrap()).unwrap();
			self.sink.append(source);
			self.world.started = true;
		}

		// set uniforms //
		const SCALE_FACTOR: f32 = 1.0 / 540.0;
		let aspect = self.config.height as f32 / self.config.width as f32;

		let map_uniform = Uniforms {
				matrix: [
								aspect * SCALE_FACTOR         ,                0.0                  ,
										0.0                   ,            SCALE_FACTOR             ,
					-self.world.camera.x * aspect * SCALE_FACTOR , -self.world.camera.y * SCALE_FACTOR ,
					0.0, 0.0 // I hate padding
				],
				color: [1.0, 1.0, 1.0, 1.0],
			};

			self.queue.write_buffer(&self.map_uniforms.buffer, 0, bytemuck::cast_slice(&[map_uniform]));

		const COLORS: [[f32; 4]; 4] = [[1.0, 0.0, 0.0, 1.0], [1.0, 1.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0]]; // I hate padding (pretend the padding is alpha)
		let square_uniform = Uniforms {
				matrix: [
								aspect * SCALE_FACTOR * self.world.square.squish.x         ,                0.0                  ,
										0.0                   ,            SCALE_FACTOR * self.world.square.squish.y             ,
					(self.world.square.pos.x-self.world.camera.x) * aspect * SCALE_FACTOR , (self.world.square.pos.y-self.world.camera.y) * SCALE_FACTOR ,
					0.0, 0.0 // I hate padding
				],
				color: COLORS[self.world.square.next_note % 4],
			};

			self.queue.write_buffer(&self.square_uniforms.buffer, 0, bytemuck::cast_slice(&[square_uniform]));

		//////////////////////// Render //////////////////////////////////
		if !self.is_surface_configured {return}

		// they made this way more complicated than it used to be
		let output = match self.surface.get_current_texture() {
			wgpu::CurrentSurfaceTexture::Success(texture) => texture,
			wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => { self.surface.configure(&self.device, &self.config); surface_texture },
			wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Validation | wgpu::CurrentSurfaceTexture::Lost => { return },
			wgpu::CurrentSurfaceTexture::Outdated => { self.surface.configure(&self.device, &self.config); return },
		};

		let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
		let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

		{
			let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: None,
				color_attachments: &[Some(wgpu::RenderPassColorAttachment {
					view: &view,
					resolve_target: None,
					depth_slice: None,
					ops: wgpu::Operations {
						load: wgpu::LoadOp::Clear(wgpu::Color {
							r: 0.25, g: 0.25, b: 0.25, a: 1.0,
						}),
						store: wgpu::StoreOp::Store
					}
				})],
				..Default::default()
			});
			render_pass.set_pipeline(&self.render_pipeline);

			render_pass.set_bind_group(0, &self.map_uniforms.bg, &[]);

			render_pass.set_vertex_buffer(0, self.map_buffer.vertex.slice(..));
			render_pass.set_index_buffer(self.map_buffer.index.slice(..), wgpu::IndexFormat::Uint16);
			render_pass.draw_indexed(0..self.map_buffer.len, 0, 0..1);

			render_pass.set_bind_group(0, &self.square_uniforms.bg, &[]);

			render_pass.set_vertex_buffer(0, self.square_buffer.vertex.slice(..));
			render_pass.set_index_buffer(self.square_buffer.index.slice(..), wgpu::IndexFormat::Uint16);
			render_pass.draw_indexed(0..self.square_buffer.len, 0, 0..1);
		}

		self.queue.submit(std::iter::once(encoder.finish()));
		output.present();
	}
}