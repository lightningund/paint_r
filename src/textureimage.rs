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

/// Creates a `ColorImage` with the given size and all transparent pixels
fn create_image(size: PixCoord) -> ColorImage {
	ColorImage::filled(size, Color32::TRANSPARENT)
}

/// An image that owns an array in memory of the pixel data, as well as a texture handle
///
/// Currently also stores edit history, but this may change
pub struct TextureImage {
	pub saved: bool,
	pub size: Rect,
	pub data: ColorImage,
	pub handle: TextureHandle,
	history: Vec<Box<dyn ImageEdit>>,
	redos: Vec<Box<dyn ImageEdit>>,
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

	/// Gets the nearest pixel when scaling
	///
	/// `coord` corresponds to the coordinates in the *target* image
	fn get_scaled_pixel(&self, coord: PixCoord, size: PixCoord) -> Color32 {
		// The coordinate on our image
		let scaled_coord = coord_map(coord, [0, 0], size, [0, 0], rect_to_size(self.size));
		let idx = coord_to_idx(scaled_coord, &self.data);
		self.data.pixels[idx]
	}

	/// Resizes using nearest-neighbor
	pub fn resize(&mut self, size: PixCoord) {
		let mut target = create_image(size);

		// TODO: make this a buffer creation iterator instead of repeated writes?
		for x in 0..size[0] {
			for y in 0..size[1] {
				let idx = coord_to_idx([x, y], &target);
				target.pixels[idx] = self.get_scaled_pixel([x, y], size);
			}
		}

		self.assign(target);
	}

	/// Undoes the last edit and pushes it to the redo history
	///
	/// Does nothing if there is no history
	///
	/// It is undefined behaviour to call this if edits have been made since the last time `save_state` was called
	pub fn undo(&mut self) {
		if let Some(edit) = self.history.pop() {
			let (redo, area) = edit.apply(&mut self.data);
			self.handle.set_partial(area.min(), self.data.region_by_pixels(area.min(), area.outer_size()), TEX_OPTS);
			self.redos.push(redo);
		}
	}

	/// Redoes the last undone edit
	///
	/// Does nothing if there is no redo history
	pub fn redo(&mut self) {
		if let Some(edit) = self.redos.pop() {
			let (undo, area) = edit.apply(&mut self.data);
			self.handle.set_partial(area.min(), self.data.region_by_pixels(area.min(), area.outer_size()), TEX_OPTS);
			self.history.push(undo);
		}

		self.save_state();
	}

	pub fn edit<E: ImageEdit + 'static>(&mut self, edit: E) {
		self.redos.clear();
		self.redos.push(Box::new(edit));
		self.redo();
		self.save_state();
	}

	pub fn copy(&self, rect: PixRect) -> ColorImage {
		// The actual size of rect that we can use
		let real = rect.limit(self.data.size);
		let size = real.outer_size();
		let min = real.min();
		self.data.region_by_pixels(min, size)
	}

	pub fn cut(&mut self, rect: PixRect) -> ColorImage {
		// The actual size of rect that we can use
		let real = rect.limit(self.data.size);
		let size = real.outer_size();
		let min = real.min();
		let data = self.data.region_by_pixels(min, size);
		self.edit(BlockEdit{
			old: ColorImage::filled(size, Color32::from_black_alpha(0)),
			coord: min,
		});
		data
	}

	pub fn paste(&mut self, pos: PixCoord, data: &ColorImage) {
		self.edit(BlockEdit{
			old: data.clone(),
			coord: pos,
		});
	}

	pub fn delete(&mut self, rect: PixRect) {
		// The actual size of rect that we can use
		let real = rect.limit(self.data.size);
		let size = real.outer_size();
		let min = real.min();
		self.edit(BlockEdit{
			old: ColorImage::filled(size, Color32::TRANSPARENT),
			coord: min,
		});
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