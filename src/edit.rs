use eframe::egui;
use egui::{Color32, ColorImage};

use crate::types::*;

/// Returns `None` if out of bounds
fn pixel_from_coord(coord: PixCoord, img: &ColorImage) -> Option<Color32> {
	img.pixels.get(coord_to_idx(coord, img)).cloned()
}

/// Some kind of edit that can be applied to an image, once, and later undone if needed
pub trait ImageEdit {
	/// Modifies the target and returns the inverse action and the affected area
	fn apply(&self, target: &mut ColorImage) -> (Box<dyn ImageEdit>, PixRect);
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

pub type PixelEdits = Vec<PixelEdit>;

/// Returns `None` if the list of coordinates is empty
pub fn create_pixel_edits(col: Color32, coords: &Vec<PixCoord>) -> Option<PixelEdits> {
	if coords.len() == 0 { return None; }

	Some(coords.iter().map(|pos| {
		PixelEdit {
			oldcol: col,
			coord: *pos,
		}
	}).collect())
}

impl ImageEdit for PixelEdits {
	fn apply(&self, target: &mut ColorImage) -> (Box<dyn ImageEdit>, PixRect) {
		let mut redo: Vec<PixelEdit> = vec![];
		let mut area = PixRect {
			a: self[0].coord,
			b: self[0].coord,
		};

		for edit in self.iter() {
			// We can unwrap here since these changes have already been done and so should be totally fine
			redo.push(PixelEdit::new(target, edit.coord).unwrap());
			let idx = coord_to_idx(edit.coord, &target);
			target.pixels[idx] = edit.oldcol;
			area = area.include(edit.coord);
		}

		println!("{:?}", area);
		redo.reverse();
		(Box::new(redo), area)
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
}

impl ImageEdit for BlockEdit {
	fn apply(&self, target: &mut ColorImage) -> (Box<dyn ImageEdit>, PixRect) {
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
		(Box::new(redo), area)
	}
}