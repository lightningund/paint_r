use std::path::{Path, PathBuf};
use eframe::egui::{self, Color32, Pos2};
use egui::{Ui, TextureHandle, ColorImage, Rect, Widget as _};
use image::{ImageReader};

static TEX_OPTS: egui::TextureOptions = egui::TextureOptions{
	magnification: egui::TextureFilter::Nearest,
	minification: egui::TextureFilter::Linear,
	mipmap_mode: None,
	wrap_mode: egui::TextureWrapMode::ClampToEdge,
};

fn main() -> eframe::Result {
	let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default()
			.with_inner_size([500.0, 500.0])
			.with_drag_and_drop(true),
		..Default::default()
	};
	eframe::run_native(
		"My egui App",
		options,
		Box::new(|cc| {
			egui_extras::install_image_loaders(&cc.egui_ctx);
			Ok(Box::<MyApp>::default())
		}),
	)
}

// From https://docs.rs/egui/latest/egui/struct.ColorImage.html
fn load_image_from_path(path: &Path) -> Result<ColorImage, image::ImageError> {
	let image = ImageReader::open(path)?.decode()?;
	let size = [image.width() as _, image.height() as _];
	let image_buffer = image.to_rgba8();
	let pixels = image_buffer.as_flat_samples();
	Ok(ColorImage::from_rgba_unmultiplied(
		size,
		pixels.as_slice(),
	))
}

fn size_to_rect(size: [usize; 2]) -> Rect {
	Rect::from_two_pos(Pos2::ZERO, Pos2::new(size[0] as f32, size[1] as f32))
}

type PixelCoord = [usize; 2];

fn pixel_from_coord(coord: PixelCoord, img: &ColorImage) -> Color32 {
	img.pixels[coord[0] + coord[1] * img.width()]
}

fn coord_to_idx(coord: PixelCoord, img: &ColorImage) -> usize {
	coord[0] + coord[1] * img.width()
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

struct TextureImage {
	saved: bool,
	path: PathBuf,
	size: Rect,
	data: ColorImage,
	handle: TextureHandle,
	history: Vec<Vec<PixelEdit>>,
	redos: Vec<Vec<PixelEdit>>,
}

impl TextureImage {
	fn mini_redo(&mut self, edit: PixelEdit) {
		if self.history.is_empty() { self.history.push(vec![]); }
		let top = self.history.last_mut().unwrap(); // we can unwrap here since we know there's at least one
		top.push(PixelEdit::new(&self.data, edit.coord));
		let idx = coord_to_idx(edit.coord, &self.data);
		self.data.pixels[idx] = edit.oldcol;
		self.handle.set_partial(edit.coord, self.data.region_by_pixels(edit.coord, [1, 1]), TEX_OPTS);
	}

	fn undo(&mut self) {
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

	fn redo(&mut self) {
		if let Some(edits) = self.redos.pop() {
			for edit in edits {
				self.mini_redo(edit);
			}

			self.save_state();
		}
	}

	fn edit(&mut self, color: Color32, coord: PixelCoord) {
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

	fn save_state(&mut self) {
		self.saved = true;
	}
}

#[derive(Default, Debug)]
struct ImageCreator {
	width: String,
	height: String,
}

struct MyApp {
	creating_img: Option<ImageCreator>, // If we currently have the create new image dialog up
	save_after_release: bool, // Whether to save the undo state after each pixel or only when you stop clicking
	color: Color32, // RGB 0-255
	secondary: Color32,
	scene_rect: Rect,
	img: Option<TextureImage>,
	last_coord: Option<[usize; 2]>, // The coordinate of the last pixel we modified while dragging
}

impl Default for MyApp {
	fn default() -> Self {
		Self {
			creating_img: None,
			save_after_release: false,
			color: Color32::WHITE,
			secondary: Color32::BLACK,
			scene_rect: Rect::ZERO,
			img: Default::default(),
			last_coord: None,
		}
	}
}

impl eframe::App for MyApp {
	// This is called every time the screen updates
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show_inside(ui, |ui| {
			let mut opening = false;
			let mut saving = false;
			ui.horizontal(|ui| {
				if ui.button("New").clicked() { self.creating_img = Some(Default::default()); }
				opening = ui.button("Open").clicked();
				saving = ui.add_enabled(self.img.is_some(), egui::Button::new("Save")).clicked();
				ui.color_edit_button_srgba(&mut self.color);
				ui.label("/");
				ui.color_edit_button_srgba(&mut self.secondary);

				if let Some(img) = &mut self.img {
					if ui.add_enabled(!img.history.is_empty(), egui::Button::new("Undo")).clicked() {
						img.undo();
					}

					if ui.add_enabled(!img.redos.is_empty(), egui::Button::new("Redo")).clicked() {
						img.redo();
					}
				}

				ui.checkbox(&mut self.save_after_release, "Save After Release")
					.on_hover_text("(Currently TODO) Whether to save the undo state after each pixel or only when you stop clicking");
			});

			self.image_creator_window(ui);

			// Bresenham line test
			if ui.button("Test Lines").clicked() {
				self.assign_img(ui.ctx(), ColorImage::filled([1000, 1000], Color32::WHITE), Path::new(""));
				let mut quad_test = |dx: bool, dy: bool| {
					let a = if dx { 500 + 462 } else { 500 - 462 };
					let b = if dy { 500 + 191 } else { 500 - 191 };
					self.last_coord = Some([500, 500]);
					self.draw([a, b], false);
					self.last_coord = Some([500, 500]);
					self.draw([b, a], false);
				};

				quad_test(true, true);
				quad_test(true, false);
				quad_test(false, true);
				quad_test(false, false);
			}

			// Create a button, and check if it was clicked. Then, with short circuiting,
			// if it was clicked we create a file dialog and assign it to path based on what the user clicks
			// This also won't execute inside the block if the user cancels and there is no path
			if opening && let Some(path) = rfd::FileDialog::new().pick_file() {
				if let Ok(image_data) = load_image_from_path(&path) {
					self.assign_img(ui.ctx(), image_data, &path);

					// force a zoom reset
					self.scene_rect = Rect::NAN;
				}
			}

			if saving { self.save_img(); }

			if let Some(img) = &self.img {
				// Add a label and the path itself on the same line
				ui.horizontal(|ui| {
					ui.label("Picked File:");
					ui.monospace(img.path.display().to_string());
				});
			}

			self.scene_surface(ui);
		});
	}
}

// From Wikipedia
// Works going down-right and when dx > dy
fn bresenham_oct(dx: i32, dy: i32) -> Vec<[i32; 2]> {
	let mut points: Vec<[i32; 2]> = vec![];

	let mut d = 2*dy - dx;
	let mut y = 0;

	for x in 0..=dx {
		points.push([x, y]);
		if d > 0 {
			y += 1;
			d += 2 * (dy - dx);
		} else {
			d += 2 * dy;
		}
	}

	points
}

// Works going down-right
fn bresenham_quad(dx: i32, dy: i32) -> Vec<[i32; 2]> {
	if dx < dy {
		let points = bresenham_oct(dy, dx);
		return points.iter().map(|point| [point[1], point[0]]).collect();
	} else {
		let points = bresenham_oct(dx, dy);
		return points;
	}
}

// Works in all directions
fn bresenham_full(dx: i32, dy: i32) -> Vec<[i32; 2]> {
	let flipped_x = dx < 0;
	let flipped_y = dy < 0;
	return bresenham_quad(dx.abs(), dy.abs()).iter().map(|point| [
		if flipped_x { -point[0] } else { point[0] },
		if flipped_y { -point[1] } else { point[1] }
	]).collect();
}

fn bresenham(start: PixelCoord, end: PixelCoord) -> Vec<PixelCoord> {
	let x0 = start[0] as i32;
	let y0 = start[1] as i32;
	let x1 = end[0] as i32;
	let y1 = end[1] as i32;

	let dx = x1 - x0;
	let dy = y1 - y0;

	bresenham_full(dx, dy).iter()
		.skip(1) // skip the first one since it's the one from last frame
		.map(|point| [(point[0] + x0) as usize, (point[1] + y0) as usize])
		.collect()
}

// UI Elements
impl MyApp {
	fn image_creator_window(&mut self, ui: &mut Ui) {
		if let Some(mut creator) = self.creating_img.take() {
			let mut created = false;
			egui::Window::new("Create New Image")
				.order(egui::Order::Foreground)
				.collapsible(false)
				.show(ui.ctx(), |ui| {
				let wlabel = ui.label("Width:");
				ui.text_edit_singleline(&mut creator.width).labelled_by(wlabel.id);
				let hlabel = ui.label("Height:");
				ui.text_edit_singleline(&mut creator.height).labelled_by(hlabel.id);

				if ui.button("Create").clicked() {
					if let Ok(w) = creator.width.parse() && let Ok(h) = creator.height.parse() {
						self.assign_img(ui.ctx(), ColorImage::filled([w, h], Color32::WHITE), Path::new(""));
						created = true;
					} else {
						println!("Please enter only numbers");
					}
				}

				if ui.button("Cancel").clicked() {
					created = true;
				}
			});

			// If we didn't actually make the image this frame, put it back
			if !created { self.creating_img = Some(creator); }
		}
	}

	fn scene_surface(&mut self, ui: &mut Ui) {
		ui.label(format!("Scene rect: {:#?}", self.scene_rect));
		ui.separator();

		let scene = egui::Scene::new()
			.sense(egui::Sense::DRAG)
			.drag_pan_buttons(egui::DragPanButtons::MIDDLE)
			.zoom_range(0.0..=f32::INFINITY);

		let mut inner_rect = Rect::NAN;
		let response = scene
			.show(ui, &mut self.scene_rect, |ui| {
				if let Some(img) = &self.img {
					egui::Image::new(&img.handle).ui(ui);
				}

				inner_rect = ui.min_rect();
			})
			.response;

		// Reset the view to be exactly large enough to contain the contents
		if response.double_clicked() {
			self.scene_rect = inner_rect;
		}

		if response.contains_pointer() && let Some(pos) = response.hover_pos() {
			// The position readout works on hover
			let coords = [pos.x as i32, pos.y as i32];
			if coords[0] >= 0 && coords[1] >= 0 {
				// TODO: Figure out how to actually place this next to the other info
				ui.put(size_to_rect([750, 115]), egui::Label::new(format!("Pointer Pos: {:?}", coords)));
			}

			// The drawing on drag
			if response.dragged_by(egui::PointerButton::Primary) || response.dragged_by(egui::PointerButton::Secondary) {
				let coords = [pos.x as usize, pos.y as usize];
				self.draw(coords, response.dragged_by(egui::PointerButton::Primary));
			} else {
				self.last_coord = None;

				if self.save_after_release && let Some(img) = &mut self.img {
					img.save_state();
				}
			}
		}
	}
}

// Functional stuff
impl MyApp {
	fn draw(&mut self, coords: PixelCoord, primary: bool) {
		if self.last_coord.is_none_or(|last| coords != last) {
			if let Some(img) = &mut self.img && coords[0] < img.data.width() && coords[1] < img.data.height() {
				let color = if primary { self.color } else { self.secondary };

				if let Some(last) = self.last_coord {
					for point in bresenham(last, coords) {
						img.edit(color, point);
						if !self.save_after_release {
							img.save_state();
						}
					}
				} else {
					img.edit(color, coords);
					if !self.save_after_release {
						img.save_state();
					}
				}

				self.last_coord = Some(coords);
			}
		}
	}

	fn save_img(&self) {
		if let Some(img) = &self.img && let Some(path) = rfd::FileDialog::new()
			.set_directory(img.path.parent().map(|p| p.to_path_buf()).unwrap_or(Default::default()))
			.set_file_name(img.path.file_name().and_then(|f| f.to_str()).unwrap_or("image.png"))
			.save_file() {
			let buf_opt = image::ImageBuffer::<image::Rgba<u8>, _>::from_vec(
				img.data.width() as u32,
				img.data.height() as u32,
				img.data.pixels.iter().flat_map(|col| col.to_array()).collect()
			);
			if let Some(buf) = buf_opt {
				let res = buf.save(path);
				if let Err(err) = res {
					println!("Saving didn't work :( {}", err);
				}
			} else {
				println!("Making the buffer didn't work :(");
			}
		}
	}

	fn assign_img(&mut self, ctx: &egui::Context, data: ColorImage, path: &Path) {
		// If we have an image already, just update it
		if let Some(img) = &mut self.img {
			img.path = path.to_path_buf();
			img.size = size_to_rect(data.size);
			img.data = data.clone();
			img.handle.set(data, TEX_OPTS);
			img.history = Default::default();
			img.redos = Default::default();
		} else {
			self.img = Some(TextureImage{
				saved: false,
				path: path.to_path_buf(),
				size: size_to_rect(data.size),
				data: data.clone(),
				handle: ctx.load_texture("screenshot_demo", data, TEX_OPTS),
				history: Default::default(),
				redos: Default::default(),
			});
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_bresenham() {
		println!("to {:?}: {:?}", [6, 3], bresenham_full(6, 3));
		println!("to {:?}: {:?}", [3, 6], bresenham_full(3, 6));
		println!("to {:?}: {:?}", [6, -3], bresenham_full(6, -3));
		println!("to {:?}: {:?}", [-3, 6], bresenham_full(-3, 6));
		println!("to {:?}: {:?}", [3, -6], bresenham_full(3, -6));
		println!("to {:?}: {:?}", [-6, 3], bresenham_full(-6, 3));
		println!("to {:?}: {:?}", [-6, -3], bresenham_full(-6, -3));
		println!("to {:?}: {:?}", [-3, -6], bresenham_full(-3, -6));
	}
}
