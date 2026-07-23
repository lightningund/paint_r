use eframe::egui;
use egui::{Color32, ColorImage};

use crate::types::*;

fn pixel_from_coord(coord: PixCoord, img: &ColorImage) -> Option<Color32> {
	img.pixels.get(coord[0] + coord[1] * img.width()).cloned()
}

#[derive(Default, Debug, Clone, Copy)]
pub struct PixelEdit {
	pub oldcol: Color32,
	pub coord: PixCoord,
}

impl PixelEdit {
	pub fn new(data: &ColorImage, coord: PixCoord) -> Option<Self> {
		Some(PixelEdit {
			oldcol: pixel_from_coord(coord, data)?,
			coord
		})
	}
}

#[derive(Default, Debug, Clone)]
pub struct BlockEdit {
	pub old: ColorImage,
	pub coord: PixCoord,
}

impl BlockEdit {
	pub fn new(target: &ColorImage, data: &ColorImage, coord: PixCoord) -> Self {
		let block_end = coord_add(coord, data.size);
		let max_end = coord_min(block_end, target.size);
		let real_size = coord_sub(max_end, coord);
		BlockEdit {
			old: target.region_by_pixels(coord, real_size),
			coord
		}
	}

	pub fn apply(&self, target: &mut ColorImage) -> (BlockEdit, PixRect) {
		let redo = BlockEdit::new(target, &self.old, self.coord);
		for src_y in 0..self.old.height() {
			let dest_y = self.coord[1] + src_y;
			for src_x in 0..self.old.width() {
				let dest_x = self.coord[0] + src_x;
				let src_idx = coord_to_idx([src_x, src_y], &self.old);
				let dest_idx = coord_to_idx([dest_x, dest_y], &target);
				if let Some(pixel) = target.pixels.get_mut(dest_idx) {
					*pixel = self.old.pixels[src_idx];
				}
			}
		}

		let block_end = coord_add(self.coord, self.old.size);
		let max_end = coord_min(block_end, coord_sub(target.size, [1, 1]));
		let real_size = coord_sub(max_end, self.coord);
		println!("Theoretical end: {:?}, Max Size: {:?}, Calc max: {:?}", block_end, target.size, real_size);
		let area = PixRect{ a: self.coord, b: max_end };
		(redo, area)
	}
}

pub enum Edit {
	Pixels(Vec<PixelEdit>),
	Block(BlockEdit),
}

/// Sets a portion of the image and updates the texture handle
fn apply_edit(edits: &Vec<PixelEdit>, target: &mut ColorImage) -> (Vec<PixelEdit>, PixRect)  {
	let mut redo: Vec<PixelEdit> = vec![];
	let mut area = PixRect {
		a: edits[0].coord,
		b: edits[0].coord,
	};
	for edit in edits.iter().rev() {
		// We can unwrap here since these changes have already been done and so should be totally fine
		redo.push(PixelEdit::new(target, edit.coord).unwrap());
		let idx = coord_to_idx(edit.coord, &target);
		target.pixels[idx] = edit.oldcol;
		area = area.include(edit.coord);
	}

	println!("{:?}", area);
	(redo, area)
}

impl Edit {
	/// Apply the edit to the image
	///
	/// Returns the reverse of the edit, as well as the changed region
	pub fn apply(&self, target: &mut ColorImage) -> (Edit, PixRect) {
		match self {
			Edit::Pixels(edits) => {
				println!("Applying pixel edits");
				let (reverse, area) = apply_edit(&edits, target);
				(Edit::Pixels(reverse), area)
			},
			Edit::Block(block) => {
				println!("Applying block edits");
				let (reverse, area) = block.apply(target);
				(Edit::Block(reverse), area)
			},
		}
	}
}