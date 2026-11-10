use std::path::Path;
use std::sync::Arc;

use winit::window::Window;

use super::atlas::{Atlas, AtlasCpu};
use super::camera::{Camera2D, CameraUniform};
use super::instance::TileInstance;
use crate::world::World;

const SHADER: &str = include_str!("shader.wgsl");
const MAX_INSTANCES: usize = 65_536;

pub struct GraphicsEngine {
	surface: wgpu::Surface<'static>,
	device: wgpu::Device,
	queue: wgpu::Queue,
	config: wgpu::SurfaceConfiguration,

	pipeline: wgpu::RenderPipeline,
	camera_buffer: wgpu::Buffer,
	camera_bg: wgpu::BindGroup,

	atlas: Atlas,
	_atlas_cpu: AtlasCpu,

	instances: wgpu::Buffer,
	scratch: Vec<TileInstance>,

	pub camera: Camera2D,
	pub assets_dir: std::path::PathBuf,
}

impl GraphicsEngine {
	pub fn init(window: Arc<Window>, assets_dir: &Path) -> Self {
		pollster::block_on(Self::init_async(window, assets_dir))
	}

	async fn init_async(window: Arc<Window>, assets_dir: &Path) -> Self {
		let size = window.inner_size();
		let (width, height) = (size.width.max(1), size.height.max(1));

		let instance =
			wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
		let surface = instance.create_surface(window).expect("surface");

		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::HighPerformance,
				compatible_surface: Some(&surface),
				force_fallback_adapter: false,
				apply_limit_buckets: false,
			})
			.await
			.expect("adapter");

		let (device, queue) = adapter
			.request_device(&wgpu::DeviceDescriptor {
				label: None,
				required_features: wgpu::Features::empty(),
				required_limits: wgpu::Limits::default(),
				memory_hints: Default::default(),
				trace: wgpu::Trace::Off,
				experimental_features: Default::default(),
			})
			.await
			.expect("device");

		let caps = surface.get_capabilities(&adapter);
		let format = caps
			.formats
			.iter()
			.find(|f| f.is_srgb())
			.copied()
			.unwrap_or(caps.formats[0]);

		let config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			format,
			color_space: wgpu::SurfaceColorSpace::Auto,
			width,
			height,
			present_mode: caps.present_modes[0],
			alpha_mode: caps.alpha_modes[0],
			view_formats: vec![],
			desired_maximum_frame_latency: 2,
		};
		surface.configure(&device, &config);

		let atlas_cpu = AtlasCpu::load_from_dir(assets_dir);
		let atlas = Atlas::upload(&atlas_cpu, &device, &queue);

		let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label: Some("camera"),
			size: std::mem::size_of::<CameraUniform>() as u64,
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});

		let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("bgl"),
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Uniform,
						has_dynamic_offset: false,
						min_binding_size: None,
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					binding: 1,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Texture {
						sample_type: wgpu::TextureSampleType::Float { filterable: true },
						view_dimension: wgpu::TextureViewDimension::D2,
						multisampled: false,
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					binding: 2,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
					count: None,
				},
			],
		});

		let camera_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("camera_bg"),
			layout: &bgl,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: camera_buffer.as_entire_binding(),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::TextureView(&atlas.view),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::Sampler(&atlas.sampler),
				},
			],
		});

		let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: Some("tile_shader"),
			source: wgpu::ShaderSource::Wgsl(SHADER.into()),
		});

		let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: None,
			bind_group_layouts: &[Some(&bgl)],
			immediate_size: 0,
		});

		let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("tile_pipeline"),
			layout: Some(&layout),
			vertex: wgpu::VertexState {
				module: &shader,
				entry_point: Some("vs_main"),
				buffers: &[Some(wgpu::VertexBufferLayout {
					array_stride: std::mem::size_of::<TileInstance>() as u64,
					step_mode: wgpu::VertexStepMode::Instance,
					attributes: &[
						wgpu::VertexAttribute {
							format: wgpu::VertexFormat::Uint32,
							offset: 0,
							shader_location: 0,
						},
						wgpu::VertexAttribute {
							format: wgpu::VertexFormat::Uint32,
							offset: 4,
							shader_location: 1,
						},
					],
				})],
				compilation_options: Default::default(),
			},
			fragment: Some(wgpu::FragmentState {
				module: &shader,
				entry_point: Some("fs_main"),
				targets: &[Some(wgpu::ColorTargetState {
					format,
					blend: Some(wgpu::BlendState::ALPHA_BLENDING),
					write_mask: wgpu::ColorWrites::ALL,
				})],
				compilation_options: Default::default(),
			}),
			primitive: wgpu::PrimitiveState::default(),
			depth_stencil: None,
			multisample: wgpu::MultisampleState::default(),
			multiview_mask: None,
			cache: None,
		});

		let instances = device.create_buffer(&wgpu::BufferDescriptor {
			label: Some("instances"),
			size: (MAX_INSTANCES * std::mem::size_of::<TileInstance>()) as u64,
			usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});

		Self {
			surface,
			device,
			queue,
			config,
			pipeline,
			camera_buffer,
			camera_bg,
			atlas,
			_atlas_cpu: atlas_cpu,
			instances,
			scratch: Vec::with_capacity(MAX_INSTANCES),
			camera: Camera2D::default(),
			assets_dir: assets_dir.to_path_buf(),
		}
	}

	pub fn resize(&mut self, width: u32, height: u32) {
		if width == 0 || height == 0 {
			return;
		}
		self.config.width = width;
		self.config.height = height;
		self.surface.configure(&self.device, &self.config);
	}

	pub fn update(&mut self, world: &World) {
		let screen = [self.config.width as f32, self.config.height as f32];
		let uniform = self
			.camera
			.to_uniform(screen, [self.atlas.cols, self.atlas.rows]);
		self.queue
			.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));

		self.scratch.clear();
		let (x0, y0, x1, y1) = self.camera.visible_rect(screen[0], screen[1]);
		'outer: for y in y0..y1 {
			for x in x0..x1 {
				let cell = world.get_cell(y as usize, x as usize);
				if cell.block_id == 0 {
					continue;
				}
				if self.scratch.len() >= MAX_INSTANCES {
					break 'outer;
				}
				self.scratch.push(TileInstance::new(x, y, cell));
			}
		}

		if !self.scratch.is_empty() {
			self.queue
				.write_buffer(&self.instances, 0, bytemuck::cast_slice(&self.scratch));
		}

		let frame = match self.surface.get_current_texture() {
			wgpu::CurrentSurfaceTexture::Success(f) => f,
			wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
			_ => return,
		};

		let view = frame
			.texture
			.create_view(&wgpu::TextureViewDescriptor::default());
		let mut enc = self
			.device
			.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
		{
			let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: None,
				color_attachments: &[Some(wgpu::RenderPassColorAttachment {
					view: &view,
					depth_slice: None,
					resolve_target: None,
					ops: wgpu::Operations {
						load: wgpu::LoadOp::Clear(wgpu::Color {
							r: 0.1,
							g: 0.15,
							b: 0.2,
							a: 1.0,
						}),
						store: wgpu::StoreOp::Store,
					},
				})],
				depth_stencil_attachment: None,
				timestamp_writes: None,
				occlusion_query_set: None,
				multiview_mask: None,
			});
			if !self.scratch.is_empty() {
				pass.set_pipeline(&self.pipeline);
				pass.set_bind_group(0, &self.camera_bg, &[]);
				pass.set_vertex_buffer(0, self.instances.slice(..));
				pass.draw(0..6, 0..self.scratch.len() as u32);
			}
		}
		self.queue.submit(std::iter::once(enc.finish()));
		self.queue.present(frame);
	}
}