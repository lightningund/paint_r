// Works going down-right and when dx > dy
fn oct(dx: i32, dy: i32) -> Vec<[i32; 2]> {
	// From Wikipedia
	let mut points: Vec<[i32; 2]> = vec![];

	let mut d = 2*dy - dx;
	let mut y = 0;

	for x in 0..=dx {
		points.push([x, y]);
		if d > 0 {
			y += 1;
			d -= 2 * dx;
		}

		d += 2 * dy;
	}

	points
}

// Works going down-right
fn quad(dx: i32, dy: i32) -> Vec<[i32; 2]> {
	if dx < dy {
		let points = oct(dy, dx);
		return points.iter().map(|point| [point[1], point[0]]).collect();
	} else {
		let points = oct(dx, dy);
		return points;
	}
}

// Works in all directions, but only starting at the origin
pub fn full(dx: i32, dy: i32) -> Vec<[i32; 2]> {
	let flipped_x = dx < 0;
	let flipped_y = dy < 0;
	quad(dx.abs(), dy.abs()).iter().map(|point| [
		if flipped_x { -point[0] } else { point[0] },
		if flipped_y { -point[1] } else { point[1] }
	]).collect()
}

pub fn line(start: [usize; 2], end: [usize; 2]) -> Vec<[usize; 2]> {
	let x0 = start[0] as i32;
	let y0 = start[1] as i32;
	let x1 = end[0] as i32;
	let y1 = end[1] as i32;

	let dx = x1 - x0;
	let dy = y1 - y0;

	full(dx, dy).iter()
		.skip(1) // skip the first one since it's the one from last frame
		.map(|point| [(point[0] + x0) as usize, (point[1] + y0) as usize])
		.collect()
}