use crate::world::Cell;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TileInstance {
	pub pos: u32,
	pub tile_flags: u32,
}

impl TileInstance {
	#[inline]
	pub fn new(x: u32, y: u32, cell: Cell) -> Self {
		Self {
			pos: x | (y << 16),
			tile_flags: (cell.block_id as u32) | ((cell.block_state as u32) << 16),
		}
	}
}