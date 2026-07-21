use eframe::egui;
use egui::{ColorImage, Rect, Pos2};

pub type PixelCoord = [usize; 2];

pub fn size_to_rect(size: PixelCoord) -> Rect {
	Rect::from_two_pos(Pos2::ZERO, Pos2::new(size[0] as f32, size[1] as f32))
}

pub fn coord_to_idx(coord: PixelCoord, img: &ColorImage) -> usize {
	coord[0] + coord[1] * img.width()
}

pub fn coord_min(a: PixelCoord, b: PixelCoord) -> PixelCoord {
	[ a[0].min(b[0]), a[1].min(b[1]) ]
}

pub fn coord_max(a: PixelCoord, b: PixelCoord) -> PixelCoord {
	[ a[0].max(b[0]), a[1].max(b[1]) ]
}

pub fn coord_add(a: PixelCoord, b: PixelCoord) -> PixelCoord {
	[a[0] + b[0], a[1] + b[1]]
}

pub fn coord_sub(a: PixelCoord, b: PixelCoord) -> PixelCoord {
	[a[0] - b[0], a[1] - b[1]]
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixRect {
	pub a: PixelCoord,
	pub b: PixelCoord,
}

impl From<PixRect> for Rect {
	fn from(value: PixRect) -> Self {
		let min = value.min();
		let max = value.max();
		Rect{
			min: egui::Pos2::new(min[0] as f32, min[1] as f32),
			max: egui::Pos2::new(max[0] as f32, max[1] as f32),
		}
	}
}

// TODO: This feels like probably bad practice?
impl From<&PixRect> for Rect {
	fn from(value: &PixRect) -> Self {
		value.clone().into()
	}
}

impl PixRect {
	pub fn min(&self) -> PixelCoord {
		coord_min(self.a, self.b)
	}

	pub fn max(&self) -> PixelCoord {
		coord_max(self.a, self.b)
	}

	pub fn size(&self) -> PixelCoord {
		let mi = self.min();
		let ma = self.max();
		coord_sub(ma, mi)
	}

	/// Returns the rect that includes the given coordinate
	pub fn include(&self, coord: PixelCoord) -> Self {
		Self {
			a: coord_min(coord_min(self.a, self.b), coord),
			b: coord_max(coord_max(self.a, self.b), coord)
		}
	}
}