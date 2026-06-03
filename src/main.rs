use std::path::PathBuf;
use eframe::egui;
use egui::{Ui, TextureHandle, ColorImage, Rect, Widget as _};
use image::ImageReader;

static TEX_OPTS: egui::TextureOptions = egui::TextureOptions{
	magnification: egui::TextureFilter::Nearest,
	minification: egui::TextureFilter::Linear,
	mipmap_mode: None,
	wrap_mode: egui::TextureWrapMode::ClampToEdge,
};

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
	size: Rect,
	data: ColorImage,
	handle: TextureHandle,
}

struct MyApp {
	color: [f32; 3], // RGB 0-1
	secondary: [f32; 3],
	scene_rect: Rect,
	img: Option<Image>,
	last_coord: [usize; 2], // The coordinate of the last pixel we modified while dragging
}

impl Default for MyApp {
	fn default() -> Self {
		Self {
    		color: [0.0, 0.0, 0.0],
    		secondary: [0.0, 0.0, 0.0],
			scene_rect: Rect::ZERO,
			img: Default::default(),
    		last_coord: [usize::MAX, usize::MAX],
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
				opening = ui.button("Open").clicked();
				saving = ui.add_enabled(self.img.is_some(), egui::Button::new("Save")).clicked();
				ui.color_edit_button_rgb(&mut self.color);
				ui.label("/");
				ui.color_edit_button_rgb(&mut self.secondary);
			});

			// Create a button, and check if it was clicked. Then, with short circuiting,
			// if it was clicked we create a file dialog and assign it to path based on what the user clicks
			// This also won't execute inside the block if the user cancels and there is no path
			if opening && let Some(path) = rfd::FileDialog::new().pick_file() {
				if let Ok(image_data) = load_image_from_path(&path) {
					// If we have an image already, just update it
					if let Some(img) = &mut self.img {
						img.path = path;
						img.size = size_to_rect(image_data.size);
						img.data = image_data.clone();
						img.handle.set(image_data, TEX_OPTS);
					} else {
						self.img = Some(Image{
							path,
							size: size_to_rect(image_data.size),
							data: image_data.clone(),
							handle: ui.ctx().load_texture("screenshot_demo", image_data, TEX_OPTS),
						});
					}

					// force a zoom reset
					self.scene_rect = Rect::NAN;
				}
			}

			if saving && let Some(img) = &self.img {
				if let Some(path) = rfd::FileDialog::new()
					.set_directory(img.path.parent().map(|p| p.to_path_buf()).unwrap_or(Default::default()))
					.set_file_name(img.path.file_name().and_then(|f| f.to_str()).unwrap_or("image.png"))
					.save_file() {
					// TODO: impl Pixel for Color32 so that I don't need to do the mapping
					let buf_opt = image::ImageBuffer::<image::Rgba<u8>, _>::from_vec(
						img.data.width() as u32,
						img.data.height() as u32,
						img.data.pixels.iter().map(|col| col.to_array()).flatten().collect()
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

impl MyApp {
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

		if let Some(pos) = response.hover_pos() {
			// The position readout works on hover
			let coords = [pos.x as i32, pos.y as i32];
			if coords[0] >= 0 && coords[1] >= 0 {
				ui.put(size_to_rect([350, 50]), egui::Label::new(format!("Pointer Pos: {:?}", coords)));
			}

			// The drawing on drag
			if response.dragged_by(egui::PointerButton::Primary) || response.dragged_by(egui::PointerButton::Secondary) {
				let coords = [pos.x as usize, pos.y as usize];
				if coords != self.last_coord {
					if let Some(img) = &mut self.img && coords[0] < img.data.width() && coords[1] < img.data.height() {
						println!("Clicked: {:?}", coords);
						self.last_coord = coords;
						let primary = response.dragged_by(egui::PointerButton::Primary);
						let color = if primary { self.color } else { self.secondary }.map(|v| (v * 255.0) as u8);
						let idx = coords[0] + coords[1] * img.data.width();
						img.data.pixels[idx] = egui::Color32::from_rgb(color[0], color[1], color[2]);
						img.handle.set_partial(coords, img.data.region_by_pixels(coords, [1, 1]), TEX_OPTS);
					}
				}
			} else {
				self.last_coord = [usize::MAX, usize::MAX];
			}
		}
	}
}
