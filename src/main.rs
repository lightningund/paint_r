mod types;
mod textureimage;
mod layermanager;
mod bresenham;
mod edit;

use std::ops::Deref;
use std::path::{Path, PathBuf};
use eframe::egui;
use egui::{Color32, Button, Ui, ColorImage, Rect};
use image::{ImageReader};

use crate::textureimage::*;
use crate::layermanager::*;
use crate::types::*;

use egui::PointerButton::Primary as PRIMARY_CLICK;
use egui::PointerButton::Secondary as SECONDARY_CLICK;

/// The minimum number of screen pixels per image pixel to still draw the gridlines
static MIN_GRID_SIZE: f32 = 5.0;

fn main() -> eframe::Result {
	let args: Vec<String> = std::env::args().collect();

	let icon = eframe::icon_data::from_png_bytes(include_bytes!("../icon.png")).expect("Couldn't Load Icon");

	let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default()
			.with_icon(std::sync::Arc::new(icon))
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

#[derive(Default, Debug)]
struct ImageCreator {
	width: String,
	height: String,
}

#[derive(PartialEq)]
enum Tool {
	Eyedropper,
	Pencil,
	Rect,
	Select,
	Paste,
	Line,
}

struct MyApp {
	creating_img: Option<ImageCreator>, // If we currently have the create new image dialog up
	save_after_release: bool, // Whether to save the undo state after each pixel or only when you stop clicking
	show_grid: bool, // Whether to show gridlines around the pixels
	color: Color32,
	secondary: Color32,
	scene_rect: Rect,
	interacting: bool,
	layers: LayerManager,
	last_coord: Option<PixCoord>, // The coordinate of the last pixel we modified while dragging
	tool: Tool,
	selection: Option<PixRect>,
	clipboard: Option<ColorImage>,
	cursor_pos: Option<PixCoord>,
	path: Option<PathBuf>,
}

impl Default for MyApp {
	fn default() -> Self {
		Self {
			creating_img: None,
			save_after_release: true,
			show_grid: false,
			color: Color32::WHITE,
			secondary: Color32::BLACK,
			scene_rect: Rect::ZERO,
			layers: Default::default(),
			interacting: false,
			last_coord: None,
			tool: Tool::Pencil,
			selection: Default::default(),
			clipboard: Default::default(),
			cursor_pos: Default::default(),
			path: Default::default(),
		}
	}
}

impl eframe::App for MyApp {
	// This is called every time the screen updates
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
		// Show the image creation window if needed
		self.image_creator_window(ui);

		// The main settings
		egui::Panel::top(ui.next_auto_id()).show_inside(ui, |ui| self.top_bar(ui));

		// The status info bar
		egui::Panel::bottom(ui.next_auto_id()).show_inside(ui, |ui| self.status_bar(ui));

		// The tool selection panel
		egui::Panel::left(ui.next_auto_id()).show_inside(ui, |ui| self.tool_panel(ui));

		// The layer selection/edit panel
		egui::Panel::right(ui.next_auto_id()).show_inside(ui, |ui| self.layers.draw_panel(ui));

		// The main canvas
		egui::CentralPanel::default().show_inside(ui, |ui| self.image_zone(ui));
	}
}

/// Returns true if a given key was pressed this frame
///
/// Includes key-repeat events
fn pressed(ctx: &egui::Context, key: egui::Key) -> bool {
	ctx.input(|i| i.key_pressed(key))
}

/// Sets a value if a given key was pressed this frame
///
/// Includes key-repeat events
fn set_if_key<T>(ctx: &egui::Context, key: egui::Key, target: &mut T, val: T) {
	if ctx.input(|i| i.key_pressed(key)) {
		*target = val;
	}
}

// UI Elements
impl MyApp {
	/// Popup window used for creating images
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

	/// All of the main settings
	fn top_bar(&mut self, ui: &mut Ui) {
		ui.horizontal(|ui| {
			if ui.button("New").clicked() {
				self.creating_img = Some(Default::default());
			}

			if ui.button("Open").clicked() {
				self.open(ui.ctx());
			}

			if ui.add_enabled(!self.layers.is_empty() && self.path.is_some(), Button::new("Save")).clicked() {
				self.save_img();
			}

			if ui.add_enabled(!self.layers.is_empty(), Button::new("Save As")).clicked() {
				self.save_as();
			}

			ui.color_edit_button_srgba(&mut self.color);
			ui.label("/");
			ui.color_edit_button_srgba(&mut self.secondary);

			if let Some(layer) = self.layers.get_active_mut() {
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
			}

			ui.checkbox(&mut self.save_after_release, "Save After Release")
				.on_hover_text("Whether to save the undo state after each pixel or only when you stop clicking");

			ui.checkbox(&mut self.show_grid, "Pixel Grid")
				.on_hover_text("Whether to show gridlines around the pixels");
		});
	}

	/// Bar displaying some stats
	fn status_bar(&self, ui: &mut Ui) {
		if let Some(pos) = self.cursor_pos {
			ui.label(format!("Pointer Pos: {:?}", pos));
		}
	}

	/// Tool selection
	fn tool_panel(&mut self, ui: &mut Ui) {
		ui.selectable_value(&mut self.tool, Tool::Pencil, "Pencil");
		ui.selectable_value(&mut self.tool, Tool::Rect, "Rect");
		ui.selectable_value(&mut self.tool, Tool::Line, "Line");
		ui.selectable_value(&mut self.tool, Tool::Select, "Select");
		ui.selectable_value(&mut self.tool, Tool::Eyedropper, "Eyedropper");
		ui.add_enabled_ui(self.clipboard.is_some(), |ui| ui.selectable_value(&mut self.tool, Tool::Paste, "Paste"));

		// Also check for shortcuts
		self.shortcuts(ui);
	}

	/// The place where the actual image being edited is displayed
	fn image_zone(&mut self, ui: &mut Ui) {
		let scene = egui::Scene::new()
			.sense(egui::Sense::DRAG)
			.drag_pan_buttons(egui::DragPanButtons::MIDDLE)
			.zoom_range(0.0..=f32::INFINITY);

		let mut inner_rect = Rect::NAN;
		let response = scene.show(ui, &mut self.scene_rect, |ui| {
			self.layers.draw_layers(ui);

			if let Some(rect) = &self.selection {
				let painter = ui.painter();
				if self.tool == Tool::Line && self.interacting {
					painter.extend(bresenham::line(rect.a, rect.b).iter().map(|coord| {
						eframe::epaint::Shape::Rect(eframe::epaint::RectShape::filled(
							PixRect {a: *coord, b: *coord}.into(),
							0.0,
							self.color
						))
					}));
				} else {
					painter.rect_filled(rect.into(), 0, Color32::from_rgba_unmultiplied(0, 0, 255, 64));
				}
			}

			inner_rect = ui.min_rect();

			if self.show_grid {
				let rect = ui.response().rect;
				let ratio = ui.content_rect().width() / rect.width();

				if ratio > MIN_GRID_SIZE {
					if let Some(img) = self.layers.get_active() {
						// Get visible bounds of the image
						let min_x = rect.min.x.max(0.0).floor() as u32;
						let min_y = rect.min.y.max(0.0).floor() as u32;
						let max_x = rect.max.x.min(img.image.size.max.x).ceil() as u32;
						let max_y = rect.max.y.min(img.image.size.max.y).ceil() as u32;

						let painter = ui.painter();
						let stroke = egui::Stroke::new(1.0 / ratio, Color32::GRAY);
						let x_range = eframe::emath::Rangef::new(min_x as f32, max_x as f32);
						let y_range = eframe::emath::Rangef::new(min_y as f32, max_y as f32);

						for x in min_x..max_x {
							painter.vline(x as f32, y_range, stroke);
						}

						for y in min_y..max_y {
							painter.hline(x_range, y as f32, stroke);
						}
					}
				}
			}
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
	fn shortcuts(&mut self, ui: &Ui) {
		use egui::Key;

		// Check for modifier shortcuts (new, open, save, save as, deselect, copy, paste)
		let ctx = ui.ctx();
		let modifiers = ctx.input(|i| i.modifiers);

		// Get the system shortcut ones (cut, copy, paste)
		ctx.input(|i| {
			for evt in &i.events {
				use egui::Event::*;
				// All three of these need an image to be present
				if let Some(layer) = self.layers.get_active_mut() {
					let img = &mut layer.image;
					match evt {
						Copy => {
							if let Some(select) = self.selection {
								self.clipboard = Some(img.copy(select));
							}
						}, Cut => {
							if let Some(select) = self.selection {
								self.clipboard = Some(img.cut(select));
							}
						}, Paste(_) => {
							if self.clipboard.is_some() {
								self.tool = Tool::Paste;
							}
						},
						_ => {}
					}
				}
			}
		});

		if modifiers.matches_logically(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT) {
			if pressed(ctx, Key::S) {
				self.save_as();
			}
		} else if modifiers.matches_logically(egui::Modifiers::COMMAND) {
			if pressed(ctx, Key::N) { // New
				self.creating_img = Some(Default::default());
			} else if pressed(ctx, Key::O) { // Open
				self.open(ctx);
			} else if pressed(ctx, Key::D) { // Deselect
				self.selection = None;
			}

			// Ones that need an image to be present
			if let Some(layer) = self.layers.get_active_mut() {
				let img = &mut layer.image;

				if pressed(ctx, Key::S) { // Save
					self.save_img();
				} else if pressed(ctx, Key::C) { // Copy
					if let Some(select) = self.selection {
						self.clipboard = Some(img.copy(select));
					}
				} else if pressed(ctx, Key::X) { // Cut
					if let Some(select) = self.selection {
						self.clipboard = Some(img.cut(select));
					}
				} else if pressed(ctx, Key::V) { // Paste
					if self.clipboard.is_some() {
						self.tool = Tool::Paste;
					}
				} else if pressed(ctx, Key::Z) { // Undo
					img.undo();
				} else if pressed(ctx, Key::Y) { // Redo
					img.redo();
				} else if pressed(ctx, Key::Delete) || pressed(ctx, Key::Backspace) {
					if let Some(select) = self.selection {
						self.clipboard = Some(img.cut(select));
					}
				}
			}
		} else if modifiers.is_none() {
			set_if_key(ctx, Key::P, &mut self.tool, Tool::Pencil);
			set_if_key(ctx, Key::S, &mut self.tool, Tool::Select);
			set_if_key(ctx, Key::K, &mut self.tool, Tool::Eyedropper);
		}
	}

	fn tool_process(&mut self, response: egui::Response) -> Option<()> {
		self.cursor_pos = None; // Reset it so that if the cursor is outside of the image, it stays None
		let img = &mut self.layers.get_active_mut()?.image;
		let pos = response.hover_pos()?;

		if response.drag_stopped() {
			self.last_coord = None;
			self.interacting = false;

			let col = if response.drag_stopped_by(PRIMARY_CLICK) { self.color } else { self.secondary };
			if self.tool == Tool::Rect {
				if let Some(rect) = self.selection.take() {
					img.paste(rect.min(), &ColorImage::filled(rect.size(), col));
				}
			} else if self.tool == Tool::Line {
				if let Some(rect) = self.selection.take() {
					for point in bresenham::line(rect.a, rect.b) {
						img.edit(col, point);
					}
					img.save_state();
				}
			}

			if self.save_after_release {
				img.save_state();
			}
		}

		if pos.x < 0.0 || pos.y < 0.0 { return None; }
		if pos.x >= img.size.max.x || pos.y >= img.size.max.y { return None; }

		// Cast it to PixelCoord
		let coords = [pos.x as usize, pos.y as usize];
		self.cursor_pos = Some(coords);

		let idx = coord_to_idx(coords, &img.data);
		let primary_down = response.dragged_by(PRIMARY_CLICK);
		let secondary_down = response.dragged_by(SECONDARY_CLICK);
		if primary_down || secondary_down {
			match self.tool {
				Tool::Pencil => {
					self.draw(coords, primary_down);
				},
				// These are grouped since they are all have the exact same dragging behaviour
				Tool::Select | Tool::Rect | Tool::Line => {
					if self.interacting && let Some(rect) = &mut self.selection {
						rect.b = coords;
					} else {
						self.selection = Some(PixRect{
							a: coords,
							b: coords
						});
					}
				},
				Tool::Eyedropper => {
					if primary_down {
						self.color = img.data.pixels[idx];
					} else if secondary_down {
						self.secondary = img.data.pixels[idx];
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

	fn draw(&mut self, coords: PixCoord, primary: bool) {
		if self.last_coord.is_none_or(|last| coords != last) {
			if let Some(layer) = &mut self.layers.get_active_mut() {
				let img = &mut layer.image;
				let color = if primary { self.color } else { self.secondary };

				if let Some(last) = self.last_coord {
					for point in bresenham::line(last, coords) {
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

	fn open(&mut self, ctx: &egui::Context) {
		if let Some(path) = rfd::FileDialog::new().pick_file() {
			if let Ok(image_data) = load_image_from_path(&path) {
				self.assign_img(ctx, image_data, &path);
			}
		}
	}

	// Marked mut because it might call save_as which sets the stored path
	fn save_img(&mut self) {
		if !self.layers.is_empty() {
			if let Some(path) = &self.path {
				self.layers.save(path);
			} else {
				self.save_as();
			}
		}
	}

	// Marked mut because it sets the stored path
	fn save_as(&mut self) {
		let exts: Vec<&str> = image::ImageFormat::all().flat_map(|fmt| fmt.extensions_str()).map(Deref::deref).collect();
		// TODO: Make this start where the current path is if there is one
		let dialog = rfd::FileDialog::new().set_file_name("image.png").add_filter("Images", &exts);
		if let Some(path) = dialog.save_file() {
			self.layers.save(&path);

			self.path = Some(path);
		}
	}

	fn assign_img(&mut self, ctx: &egui::Context, data: ColorImage, path: &Path) {
		// This feels like a memory leak tbh
		self.layers = LayerManager::default();

		self.layers.add_layer(TextureImage::new(data, ctx));

		// force a zoom reset
		self.scene_rect = Rect::NAN;

		self.path = Some(path.to_path_buf());
	}
}