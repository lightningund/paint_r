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

pub struct Layer {
	pub image: TextureImage,
	pub enabled: bool,
	pub name: String,
}

#[derive(Default)]
pub struct LayerManager {
	size: PixelCoord,
	layers: Vec<Layer>,
	curr_layer: usize,
	renaming_layer: Option<usize>,
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

		self.layers.push(Layer{ image: img, enabled: true, name: format!("Layer {}", self.layers.len()) });
		Ok(self.layers.last().unwrap())
	}

	/// Does nothing if empty
	pub fn add_empty_layer(&mut self, ctx: &egui::Context) {
		if self.is_empty() { return; }

		self.add_layer(
			TextureImage::new(ColorImage::filled(self.size, Color32::TRANSPARENT), ctx)
		).expect("Layer size somehow incorrect?");
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

					if Some(idx) == self.renaming_layer {
						if ui.text_edit_singleline(&mut layer.name).lost_focus() {
							self.renaming_layer = None;
						}
					} else {
						ui.selectable_value(&mut self.curr_layer, idx, format!("{}", layer.name)).context_menu(|ui| {
							// TODO: add settings for blending mode, opacity, delete
							if ui.button("Rename").clicked() {
								self.renaming_layer = Some(idx);
							}
						});
					}
				});
			}
		});
	}

	pub fn draw_layers(&self, ui: &mut Ui) {
		let mut img_pos = ui.cursor();
		for layer in &self.layers {
			if !layer.enabled { continue; }

			img_pos.max = layer.image.size.max;
			ui.put(img_pos, egui::Image::new(&layer.image.handle));
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
					pixel = pixel.blend(l.image.data.pixels[idx]);
				}
				image::Rgba(pixel.to_array())
			}
		);

		buf.save(path)
	}
}