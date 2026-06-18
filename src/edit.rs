use eframe::egui;
use egui::{Color32, ColorImage};

pub type PixelCoord = [usize; 2];

fn pixel_from_coord(coord: PixelCoord, img: &ColorImage) -> Option<Color32> {
	img.pixels.get(coord[0] + coord[1] * img.width()).cloned()
}

#[derive(Default, Debug, Clone, Copy)]
pub struct PixelEdit {
	pub oldcol: Color32,
	pub coord: PixelCoord,
}

impl PixelEdit {
	pub fn new(data: &ColorImage, coord: PixelCoord) -> Option<Self> {
		Some(PixelEdit {
			oldcol: pixel_from_coord(coord, data)?,
			coord
		})
	}
}

#[derive(Default, Debug, Clone)]
pub struct BlockEdit {
	pub old: ColorImage,
	pub coord: PixelCoord,
}

impl BlockEdit {
	pub fn new(target: &ColorImage, data: &ColorImage, coord: PixelCoord) -> Self {
		BlockEdit {
			old: target.region_by_pixels(coord, data.size),
			coord
		}
	}
}

pub enum Edit {
	Pixels(Vec<PixelEdit>),
	Block(BlockEdit),
}