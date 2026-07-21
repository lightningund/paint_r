use eframe::egui;
use egui::{Color32, TextureHandle, ColorImage, Rect};

use crate::types::*;
use crate::edit::*;

static TEX_OPTS: egui::TextureOptions = egui::TextureOptions{
	magnification: egui::TextureFilter::Nearest,
	minification: egui::TextureFilter::Linear,
	mipmap_mode: None,
	wrap_mode: egui::TextureWrapMode::ClampToEdge,
};

/// An image that owns an array in memory of the pixel data, as well as a texture handle
///
/// Currently also stores edit history, but this may change
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

	/// Undoes the last edit and pushes it to the redo history
	///
	/// Does nothing if there is no history
	///
	/// It is undefined behaviour to call this if edits have been made since the last time `save_state` was called
	pub fn undo(&mut self) {
		if let Some(edit) = self.history.pop() {
			let (redo, area) = edit.apply(&mut self.data);
			self.handle.set_partial(area.min(), self.data.region_by_pixels(area.min(), area.size()), TEX_OPTS);
			self.redos.push(redo);
		}
	}

	/// Redoes the last undone edit
	///
	/// Does nothing if there is no redo history
	pub fn redo(&mut self) {
		if let Some(edit) = self.redos.pop() {
			let (undo, area) = edit.apply(&mut self.data);
			self.handle.set_partial(area.min(), self.data.region_by_pixels(area.min(), area.size()), TEX_OPTS);
			self.history.push(undo);
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
		let max = coord_max(rect.max(), self.data.size);
		self.data.region_by_pixels(min, coord_sub(max, min))
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