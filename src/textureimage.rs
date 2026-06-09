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

fn size_to_rect(size: [usize; 2]) -> Rect {
	Rect::from_two_pos(Pos2::ZERO, Pos2::new(size[0] as f32, size[1] as f32))
}

pub fn pixel_from_coord(coord: PixelCoord, img: &ColorImage) -> Color32 {
	img.pixels[coord[0] + coord[1] * img.width()]
}

pub fn coord_to_idx(coord: PixelCoord, img: &ColorImage) -> usize {
	coord[0] + coord[1] * img.width()
}

pub struct PixelEdit {
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
	pub history: Vec<Vec<PixelEdit>>,
	pub redos: Vec<Vec<PixelEdit>>,
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

	pub fn undo(&mut self) {
		if let Some(edits) = self.history.pop() {
			let mut redo: Vec<PixelEdit> = vec![];
			for edit in edits.iter().rev() {
				redo.push(PixelEdit::new(&self.data, edit.coord));
				let idx = coord_to_idx(edit.coord, &self.data);
				self.data.pixels[idx] = edit.oldcol;
				self.handle.set_partial(edit.coord, self.data.region_by_pixels(edit.coord, [1, 1]), TEX_OPTS);
			}
			self.redos.push(redo);
		}
	}

	fn mini_redo(&mut self, edit: PixelEdit) {
		if self.history.is_empty() { self.history.push(vec![]); }
		let top = self.history.last_mut().unwrap(); // we can unwrap here since we know there's at least one
		top.push(PixelEdit::new(&self.data, edit.coord)); // add this edit to the current ongoing "Undo" edit
		let idx = coord_to_idx(edit.coord, &self.data);
		self.data.pixels[idx] = edit.oldcol;
		self.handle.set_partial(edit.coord, self.data.region_by_pixels(edit.coord, [1, 1]), TEX_OPTS);
	}

	pub fn redo(&mut self) {
		if let Some(edits) = self.redos.pop() {
			for edit in edits {
				self.mini_redo(edit);
			}

			self.save_state();
		}
	}

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

	pub fn save_state(&mut self) {
		self.saved = true;
	}
}