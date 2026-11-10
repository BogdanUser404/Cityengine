// (C) Bogdan Yachmenev 2026
// License LGPL 3.0

pub const WORLD_CORD: usize = 5012;
pub const WORLD_SIZE: usize = WORLD_CORD * WORLD_CORD;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Cell {
	pub block_id: u16,
	pub block_state: u16,
}

#[repr(C)]
pub struct World {
	cells: Box<[Cell; WORLD_SIZE]>,
	tick: u64,
}

impl World {
	#[inline(always)]
	pub fn get_cell(&self, y: usize, x: usize) -> Cell {
		let index: usize = (y * WORLD_CORD) + x;
		self.cells[index]
	}

	#[inline(always)]
	pub fn set_cell(&mut self, y: usize, x: usize, new_cell: Cell) {
		let index: usize = (y * WORLD_CORD) + x;
		self.cells[index] = new_cell;
	}

	#[inline]
	pub fn cells(&self) -> &[Cell] {
		&self.cells[..]
	}

	#[inline]
	pub fn cells_mut(&mut self) -> &mut [Cell] {
		&mut self.cells[..]
	}

	#[inline]
	pub fn get_tick(&self) -> u64 {
		self.tick
	}

	#[inline]
	pub fn tick(&mut self) {
		if self.tick < u64::MAX {
			self.tick += 1;
		} else {
			panic!("Tick overflow!");
		}
	}
}

impl Default for World {
	fn default() -> Self {
		let vec = vec![Cell::default(); WORLD_SIZE];
		let boxed_slice = vec.into_boxed_slice();
		let cells: Box<[Cell; WORLD_SIZE]> = boxed_slice
			.try_into()
			.expect("world size mismatch");

		Self {
			cells,
			tick: 0,
		}
	}
}