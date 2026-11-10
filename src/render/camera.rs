use crate::world::WORLD_CORD;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
	pub offset: [f32; 2],
	pub zoom:   f32,
	pub _pad0:  f32,
	pub screen: [f32; 2],
	pub atlas:  [u32; 2],
}

#[derive(Clone, Copy)]
pub struct Camera2D {
	pub x: f32,
	pub y: f32,
	pub zoom: f32,
}

impl Default for Camera2D {
	fn default() -> Self {
		Self { x: 0.0, y: 0.0, zoom: 32.0 }
	}
}

impl Camera2D {
	pub const MIN_ZOOM: f32 = 8.0;
	pub const MAX_ZOOM: f32 = 64.0;

	pub fn clamp_zoom(&mut self) {
		self.zoom = self.zoom.clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
	}

	/// Сдвиг в пикселях экрана (например, от мыши) → в мировых тайлах.
	pub fn pan_pixels(&mut self, dx: f32, dy: f32) {
		self.x = (self.x + dx / self.zoom).max(0.0);
		self.y = (self.y + dy / self.zoom).max(0.0);
	}

	/// (x0, y0, x1, y1) — эксклюзивные границы видимого прямоугольника клеток.
	pub fn visible_rect(&self, screen_w: f32, screen_h: f32) -> (u32, u32, u32, u32) {
		let w = (screen_w / self.zoom).ceil() as u32 + 1;
		let h = (screen_h / self.zoom).ceil() as u32 + 1;
		let x0 = self.x.floor().max(0.0) as u32;
		let y0 = self.y.floor().max(0.0) as u32;
		let x1 = (x0 + w).min(WORLD_CORD as u32);
		let y1 = (y0 + h).min(WORLD_CORD as u32);
		(x0, y0, x1, y1)
	}

	pub fn to_uniform(&self, screen: [f32; 2], atlas: [u32; 2]) -> CameraUniform {
		CameraUniform {
			offset: [self.x, self.y],
			zoom: self.zoom,
			_pad0: 0.0,
			screen,
			atlas,
		}
	}
}