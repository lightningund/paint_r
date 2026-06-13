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

#[derive(Default, Clone, Copy, PartialEq, Eq)]
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
	fn min(self) -> PixelCoord {
		[
			self.a[0].min(self.b[0]),
			self.a[1].min(self.b[1])
		]
	}

	fn max(self) -> PixelCoord {
		[
			self.a[0].max(self.b[0]),
			self.a[1].max(self.b[1])
		]
	}
}

#[derive(Default, Debug, Clone, Copy)]
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

#[derive(Default, Debug, Clone)]
struct BlockEdit {
	old: ColorImage,
	coord: PixelCoord,
}

impl BlockEdit {
	fn new(target: &ColorImage, data: &ColorImage, coord: PixelCoord) -> Self {
		BlockEdit {
			old: target.region_by_pixels(coord, data.size),
			coord
		}
	}
}

enum Edit {
	Pixels(Vec<PixelEdit>),
	Block(BlockEdit),
}

pub struct TextureImage {
	pub saved: bool,
	pub path: PathBuf,
	pub size: Rect,
	pub data: ColorImage,
	pub handle: TextureHandle,
	history: Vec<Edit>,
	redos: Vec<Edit>,
}

impl TextureImage {
	pub fn new(path: &Path, data: ColorImage, ctx: &egui::Context) -> Self {
		TextureImage{
			saved: true,
			path: path.to_path_buf(),
			size: size_to_rect(data.size),
			data: data.clone(),
			handle: ctx.load_texture("texture", data, TEX_OPTS),
			history: Default::default(),
			redos: Default::default(),
		}
	}

	pub fn assign(&mut self, path: &Path, data: ColorImage) {
		self.saved = true;
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
	///
	/// It is undefined behaviour to call this if edits have been made since the last time `save_state` was called
	pub fn undo(&mut self) {
		match self.history.pop() {
			Some(Edit::Pixels(edits)) => {
				println!("Undoing pixel edits");
				let mut redo: Vec<PixelEdit> = vec![];
				for edit in edits.iter().rev() {
					redo.push(PixelEdit::new(&self.data, edit.coord));
					self.set_edit(edit);
				}
				self.redos.push(Edit::Pixels(redo));
			},
			Some(Edit::Block(block)) => {
				println!("Undoing block edits");
				let redo = BlockEdit::new(&self.data, &block.old, block.coord);
				self.handle.set_partial(block.coord, self.data.region_by_pixels(block.coord, block.old.size), TEX_OPTS);
				self.redos.push(Edit::Block(redo));

				let src_range = (0..block.old.height()).flat_map(|y| {
					let y_idx = y * block.old.width();
					y_idx..(y_idx + block.old.width())
				});
				// Has to be a collected vec so that we actually evaluate it and can use self.data again
				let dest_indices = (0..block.old.height()).flat_map(|y| {
					let y_idx = (y + block.coord[1]) * self.data.width();
					let start = y_idx + block.coord[0];
					start..(start + block.old.width())
				}).collect::<Vec<_>>();

				for (src_idx, dest_idx) in src_range.zip(dest_indices.iter()) {
					self.data.pixels[*dest_idx] = block.old.pixels[src_idx];
				}

				// for src_y in 0..block.old.height() {
				// 	let dest_y = block.coord[1] + src_y;
				// 	for src_x in 0..block.old.width() {
				// 		let dest_x = block.coord[0] + src_x;
				// 		let src_idx = coord_to_idx([src_x, src_y], &block.old);
				// 		let dest_idx = coord_to_idx([dest_x, dest_y], &self.data);
				// 		self.data.pixels[dest_idx] = block.old.pixels[src_idx];
				// 	}
				// }
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
					changes.push(PixelEdit::new(&self.data, edit.coord)); // add this edit to the current ongoing "Undo" edit
					self.set_edit(&edit);
				}
				self.history.push(Edit::Pixels(changes));
			},
			Some(Edit::Block(block)) => {
				println!("Redoing block edits");
				let undo = BlockEdit::new(&self.data, &block.old, block.coord);

				for src_y in 0..block.old.height() {
					// let src_range = (src_y * block.old.size[0])..((src_y + 1) * block.old.size[0]);

					let dest_y = block.coord[1] + src_y;
					// let dest_range = (dest_y * self.data.size[0])..(dest_y * self.data.size[0] + block.old.size[0]);

					// self.data.pixels[dest_range].copy_from_slice(&block.old.pixels[src_range]);
					for src_x in 0..block.old.width() {
						let dest_x = block.coord[0] + src_x;
						let src_idx = coord_to_idx([src_x, src_y], &block.old);
						let dest_idx = coord_to_idx([dest_x, dest_y], &self.data);
						self.data.pixels[dest_idx] = block.old.pixels[src_idx];
					}
				}

				self.handle.set_partial(block.coord, block.old, TEX_OPTS);
				self.history.push(Edit::Block(undo));
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
			edits.push(PixelEdit::new(&self.data, coord)); // add this edit to the current ongoing "Undo" edit
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
		// for src_y in 0..data.height() {
		// 	let dest_y = pos[1] + src_y;
		// 	for src_x in 0..data.width() {
		// 		let dest_x = pos[0] + src_x;
		// 		let coord: PixelCoord = [dest_x, dest_y];
		// 		// TODO: There are definitely things I could do to optimize this for doing it repeatedly
		// 		self.edit(pixel_from_coord([src_x, src_y], data), coord);
		// 	}
		// }

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