use eframe::egui;
use egui::{ColorImage, Rect, Pos2};

pub type PixCoord = [usize; 2];

pub fn size_to_rect(size: PixCoord) -> Rect {
	Rect::from_two_pos(Pos2::ZERO, Pos2::new(size[0] as f32, size[1] as f32))
}

pub fn rect_to_size(rect: Rect) -> PixCoord {
	[rect.width() as usize, rect.height() as usize]
}

pub fn coord_to_idx(coord: PixCoord, img: &ColorImage) -> usize {
	coord[0] + coord[1] * img.width()
}

pub fn coord_min(a: PixCoord, b: PixCoord) -> PixCoord {
	[a[0].min(b[0]), a[1].min(b[1])]
}

pub fn coord_max(a: PixCoord, b: PixCoord) -> PixCoord {
	[a[0].max(b[0]), a[1].max(b[1])]
}

pub fn coord_add(a: PixCoord, b: PixCoord) -> PixCoord {
	[a[0] + b[0], a[1] + b[1]]
}

pub fn coord_sub(a: PixCoord, b: PixCoord) -> PixCoord {
	[a[0] - b[0], a[1] - b[1]]
}

/// Converts to f32 first
fn u_map(val: usize, a_min: usize, a_max: usize, b_min: usize, b_max: usize) -> usize {
	let fval = val as f32;
	let fa_min = a_min as f32;
	let fa_max = a_max as f32;
	let fb_min = b_min as f32;
	let fb_max = b_max as f32;
	(((fval - fa_min) / (fa_max - fa_min)) * (fb_max - fb_min) + fb_min) as usize
}

pub fn coord_map(t: PixCoord, a_min: PixCoord, a_max: PixCoord, b_min: PixCoord, b_max: PixCoord) -> PixCoord {
	[
		u_map(t[0], a_min[0], a_max[0], b_min[0], b_max[0]),
		u_map(t[1], a_min[1], a_max[1], b_min[1], b_max[1]),
	]
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixRect {
	pub a: PixCoord,
	pub b: PixCoord,
}

impl From<PixRect> for Rect {
	fn from(value: PixRect) -> Self {
		let min = value.min();
		let max = value.outer();
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
	pub fn min(&self) -> PixCoord {
		coord_min(self.a, self.b)
	}

	pub fn max(&self) -> PixCoord {
		coord_max(self.a, self.b)
	}

	/// Max +1, good for boundaries that are exclusive
	pub fn outer(&self) -> PixCoord {
		coord_add(self.max(), [1, 1])
	}

	pub fn size(&self) -> PixCoord {
		let mi = self.min();
		let ma = self.max();
		coord_sub(ma, mi)
	}

	/// Like `outer()` but for size
	pub fn outer_size(&self) -> PixCoord {
		let mi = self.min();
		let ma = self.outer();
		coord_sub(ma, mi)
	}

	/// Returns the rect that includes the given coordinate
	pub fn include(&self, coord: PixCoord) -> Self {
		Self {
			a: coord_min(self.min(), coord),
			b: coord_max(self.max(), coord)
		}
	}

	/// Caps the size
	pub fn limit(&self, max_size: PixCoord) -> Self {
		Self {
			a: coord_min(self.min(), max_size),
			b: coord_min(self.max(), max_size)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_u_map() {
		assert_eq!(u_map(  0, 0, 100, 0, 1000),    0);
		assert_eq!(u_map( 20, 0, 100, 0, 1000),  200);
		assert_eq!(u_map( 50, 0, 100, 0, 1000),  500);
		assert_eq!(u_map( 80, 0, 100, 0, 1000),  800);
		assert_eq!(u_map(100, 0, 100, 0, 1000), 1000);
		assert_eq!(u_map(   0, 0, 1000, 0, 100),   0);
		assert_eq!(u_map( 200, 0, 1000, 0, 100),  20);
		assert_eq!(u_map( 500, 0, 1000, 0, 100),  50);
		assert_eq!(u_map( 800, 0, 1000, 0, 100),  80);
		assert_eq!(u_map(1000, 0, 1000, 0, 100), 100);
	}
}