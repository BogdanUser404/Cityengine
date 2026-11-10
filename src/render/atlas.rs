use std::collections::HashMap;
use std::path::Path;

pub const TILE: u32 = 64;

// CPU version
pub struct AtlasCpu {
	pub pixels: Vec<u8>,
	pub cols: u32,
	pub rows: u32,
}

impl AtlasCpu {
	pub fn load_from_dir(dir: &Path) -> Self {
		let mut blocks: HashMap<u16, image::RgbaImage> = HashMap::new();
		let mut max_id: u16 = 0;

		if let Ok(entries) = std::fs::read_dir(dir) {
			for entry in entries.flatten() {
				let path = entry.path();
				if path.extension().and_then(|e| e.to_str()) != Some("png") {
					continue;
				}
				let Some(id) = path
					.file_stem()
					.and_then(|s| s.to_str())
					.and_then(|s| s.parse::<u16>().ok())
				else {
					continue;
				};

				let img = match image::open(&path) {
					Ok(i) => i.to_rgba8(),
					Err(e) => {
						eprintln!("skip {}: {e}", path.display());
						continue;
					}
				};
				let img = if img.width() != TILE || img.height() != TILE {
					eprintln!("warn: {} not {}x{}, resizing", path.display(), TILE, TILE);
					image::imageops::resize(
						&img,
						TILE,
						TILE,
						image::imageops::FilterType::Nearest,
					)
				} else {
					img
				};

				max_id = max_id.max(id);
				blocks.insert(id, img);
			}
		}

		if blocks.is_empty() {
			eprintln!("warn: no png in {}, empty atlas", dir.display());
		}

		let needed = max_id as u32 + 1;
		let cols = (needed as f32).sqrt().ceil().max(1.0) as u32;
		let rows = ((needed + cols - 1) / cols).max(1);

		let mut atlas = image::RgbaImage::new(cols * TILE, rows * TILE);
		for (id, img) in &blocks {
			let cx = (id % cols as u16) as u32;
			let cy = (id / cols as u16) as u32;
			image::imageops::overlay(&mut atlas, img, (cx * TILE) as i64, (cy * TILE) as i64);
		}

		Self { pixels: atlas.into_raw(), cols, rows }
	}

	#[inline]
	pub fn dims_px(&self) -> (u32, u32) {
		(self.cols * TILE, self.rows * TILE)
	}
}

// GPU Version
pub struct Atlas {
	pub texture: wgpu::Texture,
	pub view:    wgpu::TextureView,
	pub sampler: wgpu::Sampler,
	pub cols:    u32,
	pub rows:    u32,
}

impl Atlas {
	pub fn upload(cpu: &AtlasCpu, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
		let (w, h) = cpu.dims_px();
		let size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
		let texture = device.create_texture(&wgpu::TextureDescriptor {
			label: Some("atlas"),
			size,
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: wgpu::TextureFormat::Rgba8UnormSrgb,
			usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
			view_formats: &[],
		});
		queue.write_texture(
			wgpu::TexelCopyTextureInfo {
				texture: &texture,
				mip_level: 0,
				origin: wgpu::Origin3d::ZERO,
				aspect: wgpu::TextureAspect::All,
			},
			&cpu.pixels,
			wgpu::TexelCopyBufferLayout {
				offset: 0,
				bytes_per_row: Some(4 * w),
				rows_per_image: Some(h),
			},
			size,
		);
		let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
		let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			mag_filter: wgpu::FilterMode::Nearest,
			min_filter: wgpu::FilterMode::Nearest,
			..Default::default()
		});
		Self { texture, view, sampler, cols: cpu.cols, rows: cpu.rows }
	}
}