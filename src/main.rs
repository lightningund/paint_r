use eframe::egui;

fn main() -> eframe::Result {
	println!("Hello, world!");
	let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 500.0]),
		..Default::default()
	};
	eframe::run_native(
		"My egui App",
		options,
		Box::new(|_| {
			Ok(Box::<MyApp>::default())
		}),
	)
}

struct MyApp {
	name: String,
	age: u32,
}

impl Default for MyApp {
	fn default() -> Self {
		Self {
			name: "Arthur".to_owned(),
			age: 42,
		}
	}
}

impl eframe::App for MyApp {
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show_inside(ui, |ui| {
			ui.heading("My egui Application");
			ui.horizontal(|ui| {
				let name_label = ui.label("Your name: ");
				ui.text_edit_singleline(&mut self.name)
					.labelled_by(name_label.id);
			});
			ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
			if ui.button("Increment").clicked() {
				self.age += 1;
			}
			ui.label(format!("Hello '{}', age {}", self.name, self.age));

			// ui.image(egui::include_image!(
			// 	"../../../crates/egui/assets/ferris.png"
			// ));
		});
	}
}
