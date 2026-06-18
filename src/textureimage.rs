/// An image that owns an array in memory of the pixel data, as well as a texture handle
///
/// Currently also stores edit history, but this may change

use eframe::egui;
use egui::{Color32, TextureHandle, ColorImage, Rect, Pos2};

use crate::edit::*;

static TEX_OPTS: egui::TextureOptions = egui::TextureOptions{
	magnification: egui::TextureFilter::Nearest,
	minification: egui::TextureFilter::Linear,
	mipmap_mode: None,
	wrap_mode: egui::TextureWrapMode::ClampToEdge,
};

pub type PixelCoord = [usize; 2];

fn size_to_rect(size: PixelCoord) -> Rect {
	Rect::from_two_pos(Pos2::ZERO, Pos2::new(size[0] as f32, size[1] as f32))
}

pub fn coord_to_idx(coord: PixelCoord, img: &ColorImage) -> usize {
	coord[0] + coord[1] * img.width()
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixRect {
	pub a: PixelCoord,
	pub b: PixelCoord,
}

impl From<PixRect> for Rect {
	fn from(value: PixRect) -> Self {
		let min = value.min();
		let max = value.max();
		Rect{
			min: egui::Pos2::new(min[0] as f32, min[1] as f32),
			max: egui::Pos2::new(max[0] as f32, max[1] as f32),
		}
	}
}

// TODO: This feels like probably bad practice?
impl From<&PixRect> for Rect {
	fn from(value: &PixRect) -> Self {
		value.clone().into()
	}
}

impl PixRect {
	fn min(&self) -> PixelCoord {
		[
			self.a[0].min(self.b[0]),
			self.a[1].min(self.b[1])
		]
	}

	fn max(&self) -> PixelCoord {
		[
			self.a[0].max(self.b[0]),
			self.a[1].max(self.b[1])
		]
	}
}

pub struct TextureImage {
	pub saved: bool,
	pub size: Rect,
	pub data: ColorImage,
	pub handle: TextureHandle,
	history: Vec<Edit>,
	redos: Vec<Edit>,
}

impl TextureImage {
	pub fn new(data: ColorImage, ctx: &egui::Context) -> Self {
		TextureImage{
			saved: true,
			size: size_to_rect(data.size),
			data: data.clone(),
			handle: ctx.load_texture("texture", data, TEX_OPTS),
			history: Default::default(),
			redos: Default::default(),
		}
	}

	pub fn assign(&mut self, data: ColorImage) {
		self.saved = true;
		self.size = size_to_rect(data.size);
		self.data = data.clone();
		self.handle.set(data, TEX_OPTS);
		self.history = Default::default();
		self.redos = Default::default();
	}

	/// Sets a portion of the image and updates the texture handle
	///
	/// Does not modify the history in any way
	fn set_edit(&mut self, edit: &PixelEdit) {
		let idx = coord_to_idx(edit.coord, &self.data);
		self.data.pixels[idx] = edit.oldcol;
		self.handle.set_partial(edit.coord, self.data.region_by_pixels(edit.coord, [1, 1]), TEX_OPTS);
	}

	/// Sets a portion of the image and updates the texture handle
	///
	/// Does not modify the history in any way
	///
	/// TODO: This crashes if it can't fit the entire image in the destination
	fn set_block(&mut self, block: &BlockEdit) {
		for src_y in 0..block.old.height() {
			let dest_y = block.coord[1] + src_y;
			for src_x in 0..block.old.width() {
				let dest_x = block.coord[0] + src_x;
				let src_idx = coord_to_idx([src_x, src_y], &block.old);
				let dest_idx = coord_to_idx([dest_x, dest_y], &self.data);
				if let Some(pixel) = self.data.pixels.get_mut(dest_idx) {
					*pixel = block.old.pixels[src_idx];
				}
			}
		}

		self.handle.set_partial(block.coord, self.data.region_by_pixels(block.coord, block.old.size), TEX_OPTS);
	}

	/// Undoes the last edit and pushes it to the redo history
	///
	/// Does nothing if there is no history
	///
	/// It is undefined behaviour to call this if edits have been made since the last time `save_state` was called
	pub fn undo(&mut self) {
		match self.history.pop() {
			Some(Edit::Pixels(edits)) => {
				println!("Undoing pixel edits");
				let mut redo: Vec<PixelEdit> = vec![];
				for edit in edits.iter().rev() {
					// We can unwrap here since these changes have already been done and so should be totally fine
					redo.push(PixelEdit::new(&self.data, edit.coord).unwrap());
					self.set_edit(edit);
				}
				self.redos.push(Edit::Pixels(redo));
			},
			Some(Edit::Block(block)) => {
				println!("Undoing block edits");
				let redo = BlockEdit::new(&self.data, &block.old, block.coord);
				self.redos.push(Edit::Block(redo));
				self.set_block(&block);
			},
			None => {}
		}
	}

	/// Redoes the last undone edit
	///
	/// Does nothing if there is no redo history
	pub fn redo(&mut self) {
		match self.redos.pop() {
			Some(Edit::Pixels(edits)) => {
				println!("Redoing pixel edits");
				let mut changes = vec![];
				for edit in edits {
					if let Some(change) = PixelEdit::new(&self.data, edit.coord) {
						changes.push(change); // add this edit to the current ongoing "Undo" edit
					}
					self.set_edit(&edit);
				}
				self.history.push(Edit::Pixels(changes));
			},
			Some(Edit::Block(block)) => {
				println!("Redoing block edits");
				let undo = BlockEdit::new(&self.data, &block.old, block.coord);
				self.history.push(Edit::Block(undo));
				self.set_block(&block);
			},
			None => {}
		}

		self.save_state();
	}

	/// Set a single pixel to a color
	///
	/// Does nothing if the coordinates are out of the bounds of the image
	pub fn edit(&mut self, color: Color32, coord: PixelCoord) {
		if coord[0] >= self.data.width() || coord[1] >= self.data.height() { return; }

		if self.saved {
			self.saved = false;
			self.history.push(Edit::Pixels(vec![]));
		}

		// If the last one isn't a pixels edit, then push one
		match self.history.last() {
			Some(Edit::Pixels(_)) => {},
			_ => { self.history.push(Edit::Pixels(vec![])); }
		}

		// We know this is going to be true, but whatever I guess
		// There's definitely a better way to do this
		if let Some(Edit::Pixels(edits)) = self.history.last_mut() {
			self.redos.clear();
			// We can unwrap here since we already did bounds checking on the top
			edits.push(PixelEdit::new(&self.data, coord).unwrap()); // add this edit to the current ongoing "Undo" edit
			self.set_edit(&PixelEdit{
				oldcol: color,
				coord,
			});
		}
	}

	pub fn copy(&self, rect: PixRect) -> ColorImage {
		let min = rect.min();
		let max = rect.max();
		self.data.region_by_pixels(min, [max[0] - min[0], max[1] - min[1]])
	}

	pub fn paste(&mut self, pos: PixelCoord, data: &ColorImage) {
		self.redos.clear();
		self.redos.push(Edit::Block(BlockEdit{
			old: data.clone(),
			coord: pos,
		}));
		self.redo();

		self.saved = true;
	}

	/// Mark the current edit as complete and push it to the history
	pub fn save_state(&mut self) {
		self.saved = true;
	}

	/// If there are edits to undo
	pub fn has_undo(&self) -> bool {
		!self.history.is_empty()
	}

	/// If there are edits to redo
	///
	/// Cleared whenever a manual edit is made
	pub fn has_redo(&self) -> bool {
		!self.redos.is_empty()
	}
}