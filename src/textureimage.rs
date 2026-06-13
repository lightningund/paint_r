use std::path::{Path, PathBuf};
use eframe::egui::{self, Color32};
use egui::{TextureHandle, ColorImage, Rect, Pos2};

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

fn pixel_from_coord(coord: PixelCoord, img: &ColorImage) -> Color32 {
	img.pixels[coord[0] + coord[1] * img.width()]
}

pub fn coord_to_idx(coord: PixelCoord, img: &ColorImage) -> usize {
	coord[0] + coord[1] * img.width()
}

pub struct PixRect {
	min: PixelCoord,
	max: PixelCoord,
}

struct PixelEdit {
	oldcol: Color32,
	coord: PixelCoord,
}

impl PixelEdit {
	fn new(data: &ColorImage, coord: PixelCoord) -> Self {
		PixelEdit {
			oldcol: pixel_from_coord(coord, data),
			coord
		}
	}
}

pub struct TextureImage {
	pub saved: bool,
	pub path: PathBuf,
	pub size: Rect,
	pub data: ColorImage,
	pub handle: TextureHandle,
	history: Vec<Vec<PixelEdit>>,
	redos: Vec<Vec<PixelEdit>>,
}

impl TextureImage {
	pub fn new(path: &Path, data: ColorImage, ctx: &egui::Context) -> Self {
		TextureImage{
			saved: false,
			path: path.to_path_buf(),
			size: size_to_rect(data.size),
			data: data.clone(),
			handle: ctx.load_texture("texture", data, TEX_OPTS),
			history: Default::default(),
			redos: Default::default(),
		}
	}

	pub fn assign(&mut self, path: &Path, data: ColorImage) {
		self.saved = false;
		self.path = path.to_path_buf();
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
	pub fn undo(&mut self) {
		if let Some(edits) = self.history.pop() {
			let mut redo: Vec<PixelEdit> = vec![];
			for edit in edits.iter().rev() {
				redo.push(PixelEdit::new(&self.data, edit.coord));
				self.set_edit(edit);
			}
			self.redos.push(redo);
		}
	}

	fn mini_redo(&mut self, edit: PixelEdit) {
		if self.history.is_empty() { self.history.push(vec![]); }
		let top = self.history.last_mut().unwrap(); // we can unwrap here since we know there's at least one
		top.push(PixelEdit::new(&self.data, edit.coord)); // add this edit to the current ongoing "Undo" edit
		self.set_edit(&edit);
	}

	/// Redoes the last undone edit
	///
	/// Does nothing if there is no redo history
	pub fn redo(&mut self) {
		if let Some(edits) = self.redos.pop() {
			for edit in edits {
				self.mini_redo(edit);
			}

			self.save_state();
		}
	}

	/// Set a single pixel to a color
	///
	/// Does nothing if the coordinates are out of the bounds of the image
	pub fn edit(&mut self, color: Color32, coord: PixelCoord) {
		if coord[0] >= self.data.width() || coord[1] >= self.data.height() { return; }

		if self.saved {
			self.saved = false;
			self.history.push(vec![]);
		}

		// Just make a new redo object and immediately apply it
		self.redos.clear();
		self.mini_redo(PixelEdit{
			oldcol: color,
			coord,
		});
	}

	pub fn copy(&self, min: PixelCoord, max: PixelCoord) -> ColorImage {
		self.data.region_by_pixels(min, [max[0] - min[0], max[1] - min[1]])
	}

	pub fn paste(&mut self, pos: PixelCoord, data: &ColorImage) {

	}

	/// Mark the current edit as complete and push it to the history
	pub fn save_state(&mut self) {
		self.saved = true;
	}

	/// If there are edits to undo
	///
	/// It is undefined behaviour to call this if edits have been made since the last time `save_state` was called
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