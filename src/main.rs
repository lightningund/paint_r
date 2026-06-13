mod textureimage;
mod layermanager;

use std::path::{Path};
use eframe::egui::{self, Color32, Button};
use egui::{Ui, ColorImage, Rect};
use image::{ImageReader};

use crate::textureimage::*;
use crate::layermanager::*;

use egui::PointerButton::Primary as PRIMARY_CLICK;
use egui::PointerButton::Secondary as SECONDARY_CLICK;

fn main() -> eframe::Result {
	let args: Vec<String> = std::env::args().collect();

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
			let mut app = MyApp::default();
			if args.len() > 1 {
				let path = Path::new(&args[1]);
				let loaded = load_image_from_path(&path);
				match loaded {
					Ok(data) => app.assign_img(&cc.egui_ctx, data, &path),
					Err(err) => println!("Loading Failed: {}", err),
				}
			}
			Ok(Box::new(app))
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

// Works in all directions
fn bresenham_full(dx: i32, dy: i32) -> Vec<[i32; 2]> {
	// Works going down-right
	fn quad(dx: i32, dy: i32) -> Vec<[i32; 2]> {
		// Works going down-right and when dx > dy
		fn oct(dx: i32, dy: i32) -> Vec<[i32; 2]> {
			// From Wikipedia
			let mut points: Vec<[i32; 2]> = vec![];

			let mut d = 2*dy - dx;
			let mut y = 0;

			for x in 0..=dx {
				points.push([x, y]);
				if d > 0 {
					y += 1;
					d -= 2 * dx;
				}

				d += 2 * dy;
			}

			points
		}

		if dx < dy {
			let points = oct(dy, dx);
			return points.iter().map(|point| [point[1], point[0]]).collect();
		} else {
			let points = oct(dx, dy);
			return points;
		}
	}

	let flipped_x = dx < 0;
	let flipped_y = dy < 0;
	return quad(dx.abs(), dy.abs()).iter().map(|point| [
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

#[derive(Default, Debug)]
struct ImageCreator {
	width: String,
	height: String,
}

#[derive(PartialEq)]
enum Tool {
	Eyedropper,
	Pencil,
	Select,
	Paste,
}

struct MyApp {
	creating_img: Option<ImageCreator>, // If we currently have the create new image dialog up
	save_after_release: bool, // Whether to save the undo state after each pixel or only when you stop clicking
	color: Color32, // RGB 0-255
	secondary: Color32,
	scene_rect: Rect,
	layers: Vec<Layer>,
	curr_layer: usize,
	interacting: bool,
	last_coord: Option<PixelCoord>, // The coordinate of the last pixel we modified while dragging
	tool: Tool,
	selection: Option<PixRect>,
	clipboard: Option<ColorImage>,
	cursor_pos: Option<PixelCoord>,
}

impl Default for MyApp {
	fn default() -> Self {
		Self {
			creating_img: None,
			save_after_release: true,
			color: Color32::WHITE,
			secondary: Color32::BLACK,
			scene_rect: Rect::ZERO,
			layers: Default::default(),
			curr_layer: 0,
			interacting: false,
			last_coord: None,
			tool: Tool::Pencil,
			selection: Default::default(),
			clipboard: Default::default(),
			cursor_pos: Default::default(),
		}
	}
}

impl eframe::App for MyApp {
	// This is called every time the screen updates
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
		egui::Panel::top(ui.next_auto_id()).show_inside(ui, |ui| {
			ui.horizontal(|ui| {
				if ui.button("New").clicked() { self.creating_img = Some(Default::default()); }

				if ui.button("Open").clicked() && let Some(path) = rfd::FileDialog::new().pick_file() {
					if let Ok(image_data) = load_image_from_path(&path) {
						self.assign_img(ui.ctx(), image_data, &path);
					}
				}

				if ui.add_enabled(!self.layers.is_empty(), Button::new("Save")).clicked() {
					self.save_img();
				}

				ui.color_edit_button_srgba(&mut self.color);
				ui.label("/");
				ui.color_edit_button_srgba(&mut self.secondary);

				if let Some(layer) = self.layers.get_mut(self.curr_layer) {
					let img = &mut layer.image;
					if ui.add_enabled(img.has_undo(), Button::new("Undo")).clicked() {
						img.undo();
					}

					if ui.add_enabled(img.has_redo(), Button::new("Redo")).clicked() {
						img.redo();
					}

					if ui.add_enabled(self.selection.is_some(), Button::new("Copy")).clicked() {
						// We can unwrap here because the only way to click the button is if selection is some
						self.clipboard = Some(img.copy(self.selection.unwrap()));
					}

					ui.ctx().input(|i| {
						for evt in &i.events {
							use egui::Event::*;
							match evt {
								Copy => {
									if let Some(sel) = self.selection {
										self.clipboard = Some(img.copy(sel));
									}
								},
								_ => {}
							}
						}
					});
				}

				ui.checkbox(&mut self.save_after_release, "Save After Release")
					.on_hover_text("Whether to save the undo state after each pixel or only when you stop clicking");
			});

			// Tool selection
			ui.horizontal(|ui| {
				ui.selectable_value(&mut self.tool, Tool::Pencil, "Pencil");
				ui.selectable_value(&mut self.tool, Tool::Select, "Select");
				ui.selectable_value(&mut self.tool, Tool::Eyedropper, "Eyedropper");
				ui.add_enabled_ui(self.clipboard.is_some(), |ui| ui.selectable_value(&mut self.tool, Tool::Paste, "Paste"));
			});

			// Bresenham line algorithm test
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
		});

		// Show the image creation window if needed
		self.image_creator_window(ui);

		// Show the layer selection panel
		self.layers_panel(ui);

		// Show the info on the bottom
		self.status_bar(ui);

		egui::CentralPanel::default().show_inside(ui, |ui| self.image_zone(ui));
	}
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

	fn layers_panel(&mut self, ui: &mut Ui) {
		egui::Panel::right(ui.next_auto_id()).show_inside(ui, |ui| {
			ui.heading("Layers");
			if ui.button("New Layer").clicked() {
				let size = self.layers[0].image.data.size;
				self.layers.push(Layer{
					image: TextureImage::new(Path::new(""), ColorImage::filled(size, Color32::TRANSPARENT), ui.ctx()),
					enabled: true,
				});
			}

			for (idx, layer) in self.layers.iter_mut().enumerate() {
				ui.horizontal(|ui| {
					ui.checkbox(&mut layer.enabled, "");
					ui.selectable_value(&mut self.curr_layer, idx, format!("{}", idx));
				});
			}
		});
	}

	fn status_bar(&self, ui: &mut Ui) {
		egui::Panel::bottom(ui.next_auto_id()).show_inside(ui, |ui| {
			if let Some(pos) = self.cursor_pos {
				ui.label(format!("Pointer Pos: {:?}", pos));
			}
		});
	}

	/// The place where the actual image being edited is displayed
	fn image_zone(&mut self, ui: &mut Ui) {
		let scene = egui::Scene::new()
			.sense(egui::Sense::DRAG)
			.drag_pan_buttons(egui::DragPanButtons::MIDDLE)
			.zoom_range(0.0..=f32::INFINITY);

		let mut inner_rect = Rect::NAN;
		let response = scene.show(ui, &mut self.scene_rect, |ui| {
			let mut img_pos = ui.cursor();
			for layer in &mut self.layers {
				if !layer.enabled { continue; }

				img_pos.max = layer.image.size.max;
				ui.put(img_pos, egui::Image::new(&layer.image.handle));
			}

			if let Some(rect) = &self.selection {
			let painter = ui.painter();
				painter.rect_filled(rect.into(), 0, Color32::from_rgba_unmultiplied(0, 0, 255, 64));
			}

			inner_rect = ui.min_rect();
		}).response;

		// Reset the view to be exactly large enough to contain the contents
		if response.double_clicked() {
			self.scene_rect = inner_rect;
		}

		self.tool_process(response);
	}
}

// Functional stuff
impl MyApp {
	fn tool_process(&mut self, response: egui::Response) -> Option<()> {
		self.cursor_pos = None; // Reset it so that if the cursor is outside of the image, it stays None
		let img = &mut self.layers.get_mut(self.curr_layer)?.image;
		let pos = response.hover_pos()?;

		if pos.x < 0.0 || pos.y < 0.0 { return None; }
		if pos.x >= img.size.max.x || pos.y >= img.size.max.y { return None; }

		// Cast it to PixelCoord
		let coords = [pos.x as usize, pos.y as usize];
		self.cursor_pos = Some(coords);

		if response.drag_stopped() {
			self.last_coord = None;
			self.interacting = false;

			if self.save_after_release {
				img.save_state();
			}
		}

		let idx = coord_to_idx(coords, &img.data);
		let primary_down = response.dragged_by(PRIMARY_CLICK);
		let secondary_down = response.dragged_by(SECONDARY_CLICK);
		if primary_down || secondary_down {
			match self.tool {
				Tool::Pencil => {
					self.draw(coords, primary_down);
				},
				Tool::Eyedropper => {
					if primary_down {
						self.color = img.data.pixels[idx];
					} else if secondary_down {
						self.secondary = img.data.pixels[idx];
					}
				},
				Tool::Select => {
					if self.interacting && let Some(rect) = &mut self.selection {
						rect.b = coords;
					} else {
						self.selection = Some(PixRect{
							a: coords,
							b: coords
						});
					}
				},
				Tool::Paste => {
					// Only do it on the first frame
					if !self.interacting {
						let section = self.clipboard.as_ref()?;
						img.paste(coords, section);
					}
				},
			}

			self.interacting = true;
		}

		Some(())
	}

	fn draw(&mut self, coords: PixelCoord, primary: bool) {
		if self.last_coord.is_none_or(|last| coords != last) {
			if let Some(layer) = &mut self.layers.get_mut(self.curr_layer) {
				let img = &mut layer.image;
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
		// TODO: FIGURE OUT HOW TO SAVE MULTIPLE LAYERS
		// if let Some(img) = &self.img && let Some(path) = rfd::FileDialog::new()
		// 	.set_directory(img.path.parent().map(|p| p.to_path_buf()).unwrap_or(Default::default()))
		// 	.set_file_name(img.path.file_name().and_then(|f| f.to_str()).unwrap_or("image.png"))
		// 	.save_file() {
		// 	let buf_opt = image::ImageBuffer::<image::Rgba<u8>, _>::from_vec(
		// 		img.data.width() as u32,
		// 		img.data.height() as u32,
		// 		img.data.pixels.iter().flat_map(|col| col.to_array()).collect()
		// 	);
		// 	if let Some(buf) = buf_opt {
		// 		let res = buf.save(path);
		// 		if let Err(err) = res {
		// 			println!("Saving didn't work :( {}", err);
		// 		}
		// 	} else {
		// 		println!("Making the buffer didn't work :(");
		// 	}
		// }
	}

	fn assign_img(&mut self, ctx: &egui::Context, data: ColorImage, path: &Path) {
		self.layers.push(Layer{
			image: TextureImage::new(path, data, ctx),
			enabled: true,
		});

		// force a zoom reset
		self.scene_rect = Rect::NAN;
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
