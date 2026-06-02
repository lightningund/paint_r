use std::path::PathBuf;
use eframe::egui;
use image::ImageReader;

static IMG_EXTS: [&str; 5] = ["jpg", "jpeg", "png", "bmp", "qoi"];
static ANIM_EXTS: [&str; 4] = ["webp", "gif", "avif", "apng"];
static VIDEO_EXTS: [&str; 4] = ["mov", "mp4", "avi", "webm"];

fn is_ext(path_name: &PathBuf, exts: &[&str]) -> bool {
	if let Some(ext_os) = path_name.extension() && let Some(ext) = ext_os.to_str() {
		exts.contains(&ext)
	} else {
		false
	}
}

fn is_img(path_name: &PathBuf) -> bool { is_ext(path_name, &IMG_EXTS) }
fn is_anim(path_name: &PathBuf) -> bool { is_ext(path_name, &ANIM_EXTS) }
fn is_video(path_name: &PathBuf) -> bool { is_ext(path_name, &VIDEO_EXTS) }

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
fn load_image_from_path(path: &PathBuf) -> Result<egui::ColorImage, image::ImageError> {
	let image = ImageReader::open(path)?.decode()?;
	let size = [image.width() as _, image.height() as _];
	let image_buffer = image.to_rgba8();
	let pixels = image_buffer.as_flat_samples();
	Ok(egui::ColorImage::from_rgba_unmultiplied(
		size,
		pixels.as_slice(),
	))
}

#[derive(Default)]
struct MyApp {
	picked_path: Option<PathBuf>,
	img: Option<egui::ColorImage>,
}

// ui.heading = <h1>
// ui.label = <p>

impl eframe::App for MyApp {
	// This is called every time the screen updates
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show_inside(ui, |ui| {
			// Header
			ui.heading("My egui Application");

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

			if let Some(img) = self.img {
				ui.image(img);
			}
		});
	}
}

fn preview_files_being_dropped(ctx: &egui::Context) {
	use egui::{Align2, Color32, Id, LayerId, Order, TextStyle};
	use std::fmt::Write as _;

	if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
		// Create a list of all the files being dropped
		let text = ctx.input(|i| {
			let mut text = "Dropping files:\n".to_owned();
			for file in &i.raw.hovered_files {
				if let Some(path) = &file.path {
					write!(text, "\n{}", path.display()).ok();
				} else {
					text += "\n???";
				}
			}
			text
		});

		// Create a "painter" to draw the darkened screen and text
		let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

		// Full screen
		let content_rect = ctx.content_rect();
		// Darken the window
		painter.rect_filled(content_rect, 0.0, Color32::from_black_alpha(192));
		// Draw the text
		painter.text(
			content_rect.center(),
			Align2::CENTER_CENTER,
			text,
			TextStyle::Heading.resolve(&ctx.global_style()),
			Color32::WHITE,
		);
	}
}
