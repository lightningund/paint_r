use std::path::PathBuf;
use eframe::egui;
use egui::{Ui, TextureHandle, ColorImage, Rect, Widget as _};
use image::ImageReader;

fn main() -> eframe::Result {
	println!("Hello, world!");
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
fn load_image_from_path(path: &PathBuf) -> Result<ColorImage, image::ImageError> {
	let image = ImageReader::open(path)?.decode()?;
	let size = [image.width() as _, image.height() as _];
	let image_buffer = image.to_rgba8();
	let pixels = image_buffer.as_flat_samples();
	Ok(ColorImage::from_rgba_unmultiplied(
		size,
		pixels.as_slice(),
	))
}

// There's gotta be a better way to do this lmao
fn size_to_rect(size: [usize; 2]) -> Rect {
	Rect::with_max_x(Rect::with_max_y(Rect::ZERO, size[1] as f32), size[0] as f32)
}

struct Image {
	path: PathBuf,
	handle: TextureHandle,
	size: Rect,
}

struct MyApp {
	// In 0-1 NDC
	// TODO: Make this screen coords, but relative to the drawing frame
	lines: Vec<Vec<egui::Pos2>>,
	stroke: egui::Stroke,
	scene_rect: Rect,
	img: Option<Image>,
}

impl Default for MyApp {
	fn default() -> Self {
		Self {
			lines: Default::default(),
			stroke: Default::default(),
			scene_rect: Rect::ZERO,
			img: Default::default(),
		}
	}
}

impl eframe::App for MyApp {
	// This is called every time the screen updates
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show_inside(ui, |ui| {
			// Create a button, and check if it was clicked. Then, with short circuiting,
			// if it was clicked we create a file dialog and assign it to path based on what the user clicks
			// This also won't execute inside the block if the user cancels and there is no path
			if ui.button("Open").clicked() && let Some(path) = rfd::FileDialog::new().pick_file() {
				if let Ok(image_data) = load_image_from_path(&path) {
					// If we have an image already, just update it
					if let Some(img) = &mut self.img {
						img.path = path;
						img.size = size_to_rect(image_data.size);
						img.handle.set(image_data, Default::default());
					} else {
						self.img = Some(Image{
							path,
							size: size_to_rect(image_data.size),
							handle: ui.ctx().load_texture("screenshot_demo", image_data, egui::TextureOptions{
								magnification: egui::TextureFilter::Nearest,
								minification: egui::TextureFilter::Linear,
								..Default::default()
							}),
						});
					}

					// force a zoom reset
					self.scene_rect = Rect::NAN;
				}
			}

			if let Some(img) = &self.img {
				// Add a label and the path itself on the same line
				ui.horizontal(|ui| {
					ui.label("Picked File:");
					ui.monospace(img.path.display().to_string());
				});
			}

			self.scene_surface(ui);

			// Stroke/Brush settings
			// self.brush_settings(ui);
			// Drawing canvas
			// egui::Frame::canvas(ui.style()).show(ui, |ui| {
			// 	// If we have an image, show it
			// 	if let Some(texture) = &self.tex {
			// 		egui::Image::new(texture).paint_at(ui, ui.available_rect_before_wrap());
			// 	}

			// 	self.drawing_surface(ui);
			// });
		});
	}
}

impl MyApp {
	fn scene_surface(&mut self, ui: &mut Ui) {
		ui.label(format!("Scene rect: {:#?}", self.scene_rect));
		ui.separator();

		let scene = egui::Scene::new()
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

		if let Some(pos) = response.hover_pos() {
			ui.put(size_to_rect([350, 50]), egui::Label::new(format!("Pointer Pos: {}, {}", pos.x as i32, pos.y as i32)));
		}
	}

	fn brush_settings(&mut self, ui: &mut Ui) -> egui::Response {
		ui.horizontal(|ui| {
			ui.label("Stroke:");
			ui.add(&mut self.stroke);
			ui.separator();
			if ui.button("Clear Painting").clicked() {
				self.lines.clear();
			}
		})
		.response
	}

	fn drawing_surface(&mut self, ui: &mut Ui) -> egui::Response {
		let (mut response, painter) = ui.allocate_painter(ui.available_size_before_wrap(), egui::Sense::drag());

		let to_screen = egui::emath::RectTransform::from_to(
			Rect::from_min_size(egui::Pos2::ZERO, response.rect.square_proportions()),
			response.rect,
		);

		let from_screen = to_screen.inverse();

		if self.lines.is_empty() {
			self.lines.push(vec![]);
		}

		let current_line = self.lines.last_mut().unwrap();

		// If we are clicking
		if let Some(pointer_pos) = response.interact_pointer_pos() {
			let canvas_pos = from_screen * pointer_pos;
			// If the current position is different from the last position
			if current_line.last() != Some(&canvas_pos) {
				current_line.push(canvas_pos);
				response.mark_changed();
			}
		// If we aren't clicking and the current line isn't empty, then start a new empty line
		} else if !current_line.is_empty() {
			self.lines.push(vec![]);
			// I feel like this is not needed since creating a new empty line doesn't actually change anything
			response.mark_changed();
		}

		let shapes = self.lines.iter()
			// Filter out ones that aren't actually lines yet
			.filter(|line| line.len() >= 2)
			// Turn it into a polyline shape
			.map(|line| {
				// Map from NDC to screen
				let points: Vec<egui::Pos2> = line.iter().map(|p| to_screen * *p).collect();
				// Turn it into a polyline shape
				egui::Shape::line(points, self.stroke)
			});

		painter.extend(shapes);

		response
	}
}
