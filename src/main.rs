use eframe::egui;

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

#[derive(Default)]
struct MyApp {
	// The dropped files are a vector to enable dragging and dropping a bunch of selected files at once
	dropped_files: Vec<egui::DroppedFile>,
	picked_path: Option<String>,
	name: String,
	age: u32,
}

// ui.heading = <h1>
// ui.label = <p>

impl eframe::App for MyApp {
	// This is called every time the screen updates
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show_inside(ui, |ui| {
			// Header
			ui.heading("My egui Application");

			// Label and text input in a line
			ui.horizontal(|ui| {
				let name_label = ui.label("Your name: ");
				ui.text_edit_singleline(&mut self.name)
					.labelled_by(name_label.id);
			});

			// Add a slider from 0 to 120 with a label
			ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));

			// Add a button to also modify age
			if ui.button("Increment").clicked() {
				self.age += 1;
			}

			// Add some text
			ui.label(format!("Hello '{}', age {}", self.name, self.age));

			// Create a button, and check if it was clicked. Then, with short circuiting,
			// if it was clicked we create a file dialog and assign it to path based on what the user clicks
			// This also won't execute inside the block if the user cancels and there is no path
			if ui.button("Open").clicked() && let Some(path) = rfd::FileDialog::new().pick_file() {
				self.picked_path = Some(path.display().to_string());
			}

			// If self.picked_path is assigned
			if let Some(picked_path) = &self.picked_path {
				// Add a label and the path itself on the same line
				ui.horizontal(|ui| {
					ui.label("Picked File:");
					ui.monospace(picked_path);
				});
			}

			// If there are files that have been dropped
			if !self.dropped_files.is_empty() {
				// Create functionally a div
				ui.group(|ui| {
					ui.label("Dropped files:");

					for file in &self.dropped_files {
						// Set the info to the path if it's available, otherwise "???" if the filename is empty, otherwise the filename
						let info =
							if let Some(path) = &file.path {
								path.display().to_string()
							} else {
								if !file.name.is_empty() {
									file.name.clone()
								} else {
									"???".to_owned()
								}
							};

						// If there is a file path
						if let Some(path) = &file.path {
							// Convert it to a string
							let path_str = path.display().to_string();
							// Check if it ends with an image extension
							if path_str.ends_with(".jpeg") || path_str.ends_with(".jpg") || path_str.ends_with(".png") {
								// Load it as an image and add it to the UI
								ui.add(egui::Image::new("file://".to_string() + &path_str)
									.max_height(100.0));
							}
						}

						ui.label(info);
					}
				});
			}
		});

		preview_files_being_dropped(ui.ctx());

		// Collect dropped files:
		ui.input(|i| {
			if !i.raw.dropped_files.is_empty() {
				self.dropped_files.clone_from(&i.raw.dropped_files);
			}
		});
	}
}

fn preview_files_being_dropped(ctx: &egui::Context) {
	use egui::{Align2, Color32, Id, LayerId, Order, TextStyle};
	use std::fmt::Write as _;

	if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
		let text = ctx.input(|i| {
			let mut text = "Dropping files:\n".to_owned();
			for file in &i.raw.hovered_files {
				if let Some(path) = &file.path {
					write!(text, "\n{}", path.display()).ok();
				} else if file.mime.is_empty() {
					text += "\n???";
				} else {
					write!(text, "\n{}", file.mime).ok();
				}
			}
			text
		});

		let painter =
			ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

		let content_rect = ctx.content_rect();
		painter.rect_filled(content_rect, 0.0, Color32::from_black_alpha(192));
		painter.text(
			content_rect.center(),
			Align2::CENTER_CENTER,
			text,
			TextStyle::Heading.resolve(&ctx.global_style()),
			Color32::WHITE,
		);
	}
}
