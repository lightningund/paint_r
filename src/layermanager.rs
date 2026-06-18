use std::path::Path;
use eframe::egui;
use egui::{Ui, ColorImage, Color32};

use crate::textureimage::*;

#[derive(Debug, Default, Clone, Copy)]
pub struct DimensionError {
	target_size: PixelCoord,
	actual_size: PixelCoord,
}

impl std::fmt::Display for DimensionError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "Sizing error: Expected size: {:?}, Actual size: {:?}", self.target_size, self.actual_size)
	}
}

impl std::error::Error for DimensionError {}

#[derive(Default, Clone, Eq, PartialEq, Debug)]
pub struct LayerSettings {
	pub opacity: u8,
	pub name: String,
}

pub struct Layer {
	pub image: TextureImage,
	pub enabled: bool,
	pub settings: LayerSettings,
}

#[derive(Default)]
pub struct LayerManager {
	size: PixelCoord,
	layers: Vec<Layer>,
	curr_layer: usize,
	backup_settings: Option<LayerSettings>,
	configuring_layer: Option<usize>, // If we are currently editing the settings of a layer in the popout, the idx is stored here
}

impl LayerManager {
	pub fn is_empty(&self) -> bool {
		self.layers.is_empty()
	}

	pub fn get_active(&self) -> Option<&Layer> {
		self.layers.get(self.curr_layer)
	}

	pub fn get_active_mut(&mut self) -> Option<&mut Layer> {
		self.layers.get_mut(self.curr_layer)
	}

	pub fn add_layer(&mut self, img: TextureImage) -> Result<&Layer, DimensionError> {
		if self.is_empty() {
			// If this is the first image, use its size to determine our own
			self.size = img.data.size;
		}

		if self.size != img.data.size {
			return Err(DimensionError{ target_size: self.size, actual_size: img.data.size });
		}

		self.layers.push(Layer{
			image: img,
			enabled: true,
			settings: LayerSettings {
				opacity: 255,
				name: format!("Layer {}", self.layers.len())
			}
		});
		Ok(self.layers.last().unwrap())
	}

	/// Does nothing if empty
	pub fn add_empty_layer(&mut self, ctx: &egui::Context) {
		if self.is_empty() { return; }

		self.add_layer(
			TextureImage::new(ColorImage::filled(self.size, Color32::TRANSPARENT), ctx)
		).expect("Layer size somehow incorrect?");
	}

	fn draw_layer_settings(&mut self, ui: &mut Ui) {
		if let Some(idx) = self.configuring_layer {
			let cfg = &mut self.layers[idx].settings;
			let mut open = true;
			let window = egui::Window::new("Layer Settings")
				.order(egui::Order::Foreground)
				.collapsible(false)
				.default_pos(ui.clip_rect().center())
				.open(&mut open);

			// TODO: add settings for blending mode, delete
			window.show(ui.ctx(), |ui| {
				ui.horizontal(|ui| {
					let name_label = ui.label("Name:");
					ui.text_edit_singleline(&mut cfg.name).labelled_by(name_label.id);
				});
				ui.horizontal(|ui| {
					let opacity_label = ui.label("Opacity:");
					ui.add(egui::Slider::new(&mut cfg.opacity, 0..=255)).labelled_by(opacity_label.id);
				});

				ui.horizontal(|ui| {
					ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
						if ui.button("Cancel").clicked() {
							self.configuring_layer = None;
							if let Some(backup) = &self.backup_settings {
								*cfg = backup.clone();
							}
						}
						if ui.button("Save").clicked() {
							self.configuring_layer = None;
						}
					});
				});
			});

			if !open {
				self.configuring_layer = None;
				if let Some(backup) = &self.backup_settings {
					*cfg = backup.clone();
				}
			}
		}
	}

	pub fn draw_panel(&mut self, ui: &mut Ui) {
		egui::Panel::right(ui.next_auto_id()).show_inside(ui, |ui| {
			ui.heading("Layers");
			if ui.button("New Layer").clicked() {
				self.add_empty_layer(ui.ctx());
			}

			for (idx, layer) in self.layers.iter_mut().enumerate().rev() {
				ui.horizontal(|ui| {
					ui.checkbox(&mut layer.enabled, "");

					ui.selectable_value(&mut self.curr_layer, idx, format!("{}", layer.settings.name)).context_menu(|ui| {
						if ui.button("Settings").clicked() {
							self.configuring_layer = Some(idx);
							self.backup_settings = Some(layer.settings.clone());
						}
					});
				});
			}
		});

		self.draw_layer_settings(ui);
	}

	pub fn draw_layers(&self, ui: &mut Ui) {
		let mut img_pos = ui.cursor();
		for layer in &self.layers {
			if !layer.enabled { continue; }

			img_pos.max = layer.image.size.max;
			ui.put(img_pos, egui::Image::new(&layer.image.handle).tint(Color32::from_white_alpha(layer.settings.opacity)));
		}
	}

	pub fn save(&self, path: &Path) -> Result<(), image::ImageError> {
		let buf = image::ImageBuffer::<image::Rgba<u8>, _>::from_fn(
			self.size[0] as u32,
			self.size[1] as u32,
			|x, y| -> image::Rgba<u8> {
				let mut pixel = Color32::TRANSPARENT;
				let idx = (x as usize) + (y as usize) * self.size[0];
				for l in &self.layers {
					let mut l_pixel = l.image.data.pixels[idx];
					l_pixel = l_pixel.gamma_multiply_u8(l.settings.opacity);
					pixel = pixel.blend(l_pixel);
				}
				image::Rgba(pixel.to_array())
			}
		);

		buf.save(path)
	}
}