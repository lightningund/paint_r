use std::path::PathBuf;
use eframe::egui;
use egui::{emath, Rect, Image, Frame, Pos2, Sense, Stroke, Ui, Widget as _, UserData, ViewportCommand, TextureHandle, ColorImage};
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

#[derive(Default)]
struct MyApp {
	// In 0-1 NDC
	// TODO: Make this screen coords, but relative to the drawing frame
	lines: Vec<Vec<Pos2>>,
	stroke: Stroke,
	tex: Option<TextureHandle>,
	picked_path: Option<PathBuf>,
	img: Option<ColorImage>,
}

impl eframe::App for MyApp {
	// This is called every time the screen updates
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show_inside(ui, |ui| {
			self.ui_control(ui);

			// Create a button, and check if it was clicked. Then, with short circuiting,
			// if it was clicked we create a file dialog and assign it to path based on what the user clicks
			// This also won't execute inside the block if the user cancels and there is no path
			if ui.button("Open").clicked() && let Some(path) = rfd::FileDialog::new().pick_file() {
				if let Ok(img) = load_image_from_path(&path) {
					self.img = Some(img);
				}

				self.picked_path = Some(path);
			}

			// If self.picked_path is assigned
			if let Some(picked_path) = &self.picked_path {
				// Add a label and the path itself on the same line
				ui.horizontal(|ui| {
					ui.label("Picked File:");
					ui.monospace(picked_path.display().to_string());
				});
			}

			// If we took a screenshot this frame, save it into the app data
			if let Some(image) = self.img.take() {
				// If we have a texture made already, just update it
				if let Some(tex) = &mut self.tex {
					tex.set(image, Default::default());
				} else {
					self.tex = Some(ui.ctx().load_texture("screenshot_demo", image, Default::default()));
				}
			}

			// Drawing canvas
			Frame::canvas(ui.style()).show(ui, |ui| {
				// If we have an image, show it
				if let Some(texture) = &self.tex {
					Image::new(texture).paint_at(ui, ui.available_rect_before_wrap());
					// Image::new(texture).shrink_to_fit().ui(ui);
				}

				self.ui_content(ui);
			});
		});
	}
}

// Originally from https://github.com/emilk/egui/tree/main/crates/egui_demo_lib/src/demo/painting.rs
impl MyApp {
	fn ui_control(&mut self, ui: &mut egui::Ui) -> egui::Response {
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

	fn ui_content(&mut self, ui: &mut Ui) -> egui::Response {
		let (mut response, painter) = ui.allocate_painter(ui.available_size_before_wrap(), Sense::drag());

		let to_screen = emath::RectTransform::from_to(
			Rect::from_min_size(Pos2::ZERO, response.rect.square_proportions()),
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
				let points: Vec<Pos2> = line.iter().map(|p| to_screen * *p).collect();
				// Turn it into a polyline shape
				egui::Shape::line(points, self.stroke)
			});

		painter.extend(shapes);

		response
	}
}
