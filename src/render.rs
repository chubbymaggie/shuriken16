extern crate sdl2;
extern crate byteorder;

use std::time;
use std::rc::Rc;
use self::byteorder::{ByteOrder, LittleEndian};
use game::GameState;
use map::{MapLayer, BlendMode};
use tile::{TileSet, PaletteWithOffset};
use ui::{TextLayerRenderer, TextLayer, TextLayerContents, UILayer};
use sprite::SpriteAnimation;

#[derive(Debug)]
pub enum ResolutionTargetMode {
	FixedVerticalResolution,
	PixelPerfect
}

#[derive(Debug)]
pub struct ResolutionTarget {
	mode: ResolutionTargetMode,
	min_height: usize,
	max_height: usize,
	min_aspect_ratio: f32,
	max_aspect_ratio: f32
}

#[derive(Debug)]
pub struct RenderSize {
	pub width: usize,
	pub height: usize
}

pub struct FrameRateTextRenderer {
	start_time: time::Instant,
	last_elapsed_secs: u64,
	last_frames: usize,
	frame_rate: usize
}

impl ResolutionTarget {
	pub fn fixed_vertical_resolution(height: usize) -> ResolutionTarget {
		ResolutionTarget {
			mode: ResolutionTargetMode::FixedVerticalResolution,
			min_height: height,
			max_height: height,
			min_aspect_ratio: 4.0/3.0,
			max_aspect_ratio: 18.0/9.0
		}
	}

	pub fn fixed_vertical_resolution_with_aspect_ratio(height: usize, min_aspect_ratio: f32, max_aspect_ratio: f32) -> ResolutionTarget {
		ResolutionTarget {
			mode: ResolutionTargetMode::FixedVerticalResolution,
			min_height: height,
			max_height: height,
			min_aspect_ratio,
			max_aspect_ratio
		}
	}

	pub fn pixel_perfect(min_height: usize, max_height: usize) -> ResolutionTarget {
		ResolutionTarget {
			mode: ResolutionTargetMode::PixelPerfect,
			min_height,
			max_height,
			min_aspect_ratio: 4.0/3.0,
			max_aspect_ratio: 18.0/9.0
		}
	}

	pub fn pixel_perfect_with_aspect_ratio(min_height: usize, max_height: usize,
		min_aspect_ratio: f32, max_aspect_ratio: f32) -> ResolutionTarget {
		ResolutionTarget {
			mode: ResolutionTargetMode::PixelPerfect,
			min_height,
			max_height,
			min_aspect_ratio,
			max_aspect_ratio
		}
	}

	fn compute_render_size_for_height(&self, window_width: usize, window_height: usize,
		target_height: usize) -> (RenderSize, RenderSize) {
		let mut render_height = target_height;
		let mut render_width = (render_height * window_width) / window_height;
		let mut dest_width = window_width;
		let mut dest_height = window_height;
		let aspect = (window_width as f32) / (window_height as f32);
		if aspect < self.min_aspect_ratio {
			render_height = (render_width as f32 / self.min_aspect_ratio) as usize;
			dest_height = (dest_width as f32 / self.min_aspect_ratio) as usize;
		} else if aspect > self.max_aspect_ratio {
			render_width = (render_height as f32 * self.max_aspect_ratio) as usize;
			dest_width = (dest_height as f32 * self.max_aspect_ratio) as usize;
		}
		let render_size = RenderSize { width: render_width, height: render_height };
		let dest_size = RenderSize { width: dest_width, height: dest_height };
		(render_size, dest_size)
	}

	pub fn compute_render_sizes(&self, window_width: usize, window_height: usize) -> (RenderSize, RenderSize) {
		match self.mode {
			ResolutionTargetMode::FixedVerticalResolution =>
				self.compute_render_size_for_height(window_width, window_height, self.max_height),
			ResolutionTargetMode::PixelPerfect => {
				let mut target_height = self.min_height;
				for scale in 1.. {
					let pre_scaled_height = window_height / scale;
					if pre_scaled_height < self.min_height {
						break;
					}
					if pre_scaled_height <= self.max_height {
						target_height = pre_scaled_height;
						break;
					}
				}
				self.compute_render_size_for_height(window_width, window_height, target_height)
			}
		}
	}
}

impl FrameRateTextRenderer {
	pub fn new() -> FrameRateTextRenderer {
		FrameRateTextRenderer {
			start_time: time::Instant::now(),
			last_elapsed_secs: 0,
			last_frames: 0,
			frame_rate: 0
		}
	}

	pub fn new_ui_layer(font_tile_set: Rc<TileSet>, font_base: u8) -> Box<UILayer> {
		let mut layer = TextLayer::new(font_tile_set, font_base);
		layer.renderer = Some(Box::new(FrameRateTextRenderer::new()));
		Box::new(layer)
	}
}

impl TextLayerRenderer for FrameRateTextRenderer {
	fn update(&mut self, layer: &mut TextLayerContents, game_state: &GameState) {
		let elapsed_time = self.start_time.elapsed();
		let elapsed_secs = elapsed_time.as_secs();
		if elapsed_secs != self.last_elapsed_secs {
			self.last_elapsed_secs = elapsed_secs;
			self.frame_rate = game_state.frame - self.last_frames;
			self.last_frames = game_state.frame;
		}

		layer.clear();
		let text = format!("{} FPS", self.frame_rate);
		let x = (layer.width() as i32 - text.len() as i32) - 1;
		layer.write(x, 1, &text);
	}
}

fn normal_blend(pixel: &mut u16, color: u16) {
	*pixel = color;
}

fn add_blend(pixel: &mut u16, color: u16) {
	let existing_color = *pixel;
	let existing_r = (existing_color >> 10) & 0x1f;
	let existing_g = (existing_color >> 5) & 0x1f;
	let existing_b = existing_color & 0x1f;

	let add_r = (color >> 10) & 0x1f;
	let add_g = (color >> 5) & 0x1f;
	let add_b = color & 0x1f;

	let mut blended_r = existing_r + add_r;
	let mut blended_g = existing_g + add_g;
	let mut blended_b = existing_b + add_b;

	if blended_r > 0x1f {
		blended_r = 0x1f;
	}
	if blended_g > 0x1f {
		blended_g = 0x1f;
	}
	if blended_b > 0x1f {
		blended_b = 0x1f;
	}

	*pixel = (blended_r << 10) | (blended_g << 5) | blended_b;
}

fn subtract_blend(pixel: &mut u16, color: u16) {
	let existing_color = *pixel;
	let existing_r = (existing_color >> 10) & 0x1f;
	let existing_g = (existing_color >> 5) & 0x1f;
	let existing_b = existing_color & 0x1f;

	let sub_r = (color >> 10) & 0x1f;
	let sub_g = (color >> 5) & 0x1f;
	let sub_b = color & 0x1f;

	let blended_r;
	let blended_g;
	let blended_b;
	if sub_r >= existing_r {
		blended_r = 0;
	} else {
		blended_r = existing_r - sub_r;
	}
	if sub_g >= existing_g {
		blended_g = 0;
	} else {
		blended_g = existing_g - sub_g;
	}
	if sub_b >= existing_b {
		blended_b = 0;
	} else {
		blended_b = existing_b - sub_b;
	}

	*pixel = (blended_r << 10) | (blended_g << 5) | blended_b;
}

fn multiply_blend(pixel: &mut u16, color: u16) {
	let existing_color = *pixel;
	let existing_r = (existing_color >> 10) & 0x1f;
	let existing_g = (existing_color >> 5) & 0x1f;
	let existing_b = existing_color & 0x1f;

	let add_r = (color >> 10) & 0x1f;
	let add_g = (color >> 5) & 0x1f;
	let add_b = color & 0x1f;

	let mut blended_r = (existing_r * add_r) / 16;
	let mut blended_g = (existing_g * add_g) / 16;
	let mut blended_b = (existing_b * add_b) / 16;

	if blended_r > 0x1f {
		blended_r = 0x1f;
	}
	if blended_g > 0x1f {
		blended_g = 0x1f;
	}
	if blended_b > 0x1f {
		blended_b = 0x1f;
	}

	*pixel = (blended_r << 10) | (blended_g << 5) | blended_b;
}

fn alpha_blend(pixel: &mut u16, color: u16, alpha: u8, blend: &Fn(&mut u16, u16)) {
	let mut mixed_color = color;
	blend(&mut mixed_color, color);

	let mixed_r = (mixed_color >> 10) & 0x1f;
	let mixed_g = (mixed_color >> 5) & 0x1f;
	let mixed_b = mixed_color & 0x1f;

	let existing_color = *pixel;
	let existing_r = (existing_color >> 10) & 0x1f;
	let existing_g = (existing_color >> 5) & 0x1f;
	let existing_b = existing_color & 0x1f;

	let blended_r = ((mixed_r * (16 - alpha as u16)) + (existing_r * alpha as u16)) / 16;
	let blended_g = ((mixed_g * (16 - alpha as u16)) + (existing_g * alpha as u16)) / 16;
	let blended_b = ((mixed_b * (16 - alpha as u16)) + (existing_b * alpha as u16)) / 16;

	*pixel = (blended_r << 10) | (blended_g << 5) | blended_b;
}

fn render_tile_4bit(render_buf: &mut [u16], tile_data: &[u8], left: usize, width: usize, palette: &Option<PaletteWithOffset>,
	blend: &Fn(&mut u16, u16)) {
	let palette_entries = match palette {
		Some(pal_with_offset) => &pal_with_offset.palette.entries[pal_with_offset.offset..],
		None => return
	};
	for i in 0..width {
		let x = left + i;
		let color_index = (tile_data[x / 2] >> (4 * (x & 1))) & 0xf;
		if color_index != 0 {
			let color = palette_entries[color_index as usize];
			blend(&mut render_buf[i], color);
		}
	}
}

fn render_tile_8bit(render_buf: &mut [u16], tile_data: &[u8], left: usize, width: usize, palette: &Option<PaletteWithOffset>,
	blend: &Fn(&mut u16, u16)) {
	let palette_entries = match palette {
		Some(pal_with_offset) => &pal_with_offset.palette.entries[pal_with_offset.offset..],
		None => return
	};
	for i in 0..width {
		let x = left + i;
		let color_index = tile_data[x];
		if color_index != 0 {
			let color = palette_entries[color_index as usize];
			blend(&mut render_buf[i], color);
		}
	}
}

fn render_tile_16bit(render_buf: &mut [u16], tile_data: &[u8], left: usize, width: usize, _palette: &Option<PaletteWithOffset>,
	blend: &Fn(&mut u16, u16)) {
	for i in 0..width {
		let x = left + i;
		let color = LittleEndian::read_u16(&tile_data[x * 2 .. (x + 1) * 2]);
		if (color & 0x8000) == 0 {
			blend(&mut render_buf[i], color);
		}
	}
}

fn render_layer_with_blending(render_size: &RenderSize, render_buf: &mut Vec<Vec<u16>>,
	game: &GameState, layer: &MapLayer, scroll_x: isize, scroll_y: isize,
	tile_renderer: &Fn(&mut [u16], &[u8], usize, usize, &Option<PaletteWithOffset>, &Fn(&mut u16, u16)),
	blend: &Fn(&mut u16, u16)) {
	// Compute scrolling for this layer
	let parallax_x = layer.parallax_x as isize;
	let parallax_y = layer.parallax_y as isize;
	let auto_scroll_x = layer.auto_scroll_x as isize;
	let auto_scroll_y = layer.auto_scroll_y as isize;
	let frame = game.frame as isize;
	let bias_x = 0x40000000 - (0x40000000 % (layer.tile_width * layer.width)) as isize;
	let bias_y = 0x40000000 - (0x40000000 % (layer.tile_height * layer.height)) as isize;
	let scroll_x = (((scroll_x * parallax_x + auto_scroll_x * frame) / 0x100) + bias_x) as usize;
	let scroll_y = (((scroll_y * parallax_y + auto_scroll_y * frame) / 0x100) + bias_y) as usize;

	// Compute bounds of rendering
	let left_tile = scroll_x / layer.tile_width;
	let left_pixel = scroll_x % layer.tile_width;
	let right_tile = (scroll_x + render_size.width - 1) / layer.tile_width;
	let right_pixel = (scroll_x + render_size.width - 1) % layer.tile_width;
	let top_tile = scroll_y / layer.tile_height;
	let top_pixel = scroll_y % layer.tile_height;
	let bottom_tile = (scroll_y + render_size.height - 1) / layer.tile_height;
	let bottom_pixel = (scroll_y + render_size.height - 1) % layer.tile_height;

	// Compute tile data layout
	let tile_pitch = ((layer.tile_width * layer.tile_depth) + 7) / 8;

	// Render tiles
	let mut target_y = 0;
	for tile_y in top_tile ..= bottom_tile {
		// Grab slice of map layer for this row of tiles
		let map_row_index = tile_y % layer.height;
		let map_row = &layer.tiles[map_row_index * layer.width .. (map_row_index + 1) * layer.width];

		// Compute rendering extents for current row of tiles
		let cur_top_pixel;
		let cur_bottom_pixel;
		if tile_y == top_tile {
			cur_top_pixel = top_pixel;
		} else {
			cur_top_pixel = 0;
		}
		if tile_y == bottom_tile {
			cur_bottom_pixel = bottom_pixel;
		} else {
			cur_bottom_pixel = layer.tile_height - 1;
		}

		let mut target_x = 0;
		for tile_x in left_tile ..= right_tile {
			// Look up tile in map layer
			let tile = &map_row[tile_x % layer.width];
			if let Some(tile_ref) = tile {
				// Grab tile data for the current animation frame, and get palette for tile
				let tile_data = tile_ref.tile_set.data_for_time(tile_ref.tile_index, game.frame);
				let palette = &tile_ref.tile_set.tiles[tile_ref.tile_index].palette;

				// Compute rendering extents for current tile
				let cur_left_pixel;
				let cur_right_pixel;
				if tile_x == left_tile {
					cur_left_pixel = left_pixel;
				} else {
					cur_left_pixel = 0;
				}
				if tile_x == right_tile {
					cur_right_pixel = right_pixel;
				} else {
					cur_right_pixel = layer.tile_width - 1;
				}

				let tile_render_width = (cur_right_pixel - cur_left_pixel) + 1;

				// Render tile
				for pixel_y in cur_top_pixel ..= cur_bottom_pixel {
					let tile_data_row = &tile_data[pixel_y * tile_pitch .. (pixel_y + 1) * tile_pitch];
					let render_buf_row = &mut render_buf[target_y + (pixel_y - cur_top_pixel)];
					let render_buf_tile = &mut render_buf_row[target_x .. target_x + tile_render_width];
					tile_renderer(render_buf_tile, tile_data_row, cur_left_pixel, tile_render_width, palette, blend);
				}
			}

			// Update render target x coordinate
			if tile_x == left_tile {
				target_x += layer.tile_width - left_pixel;
			} else {
				target_x += layer.tile_width;
			}
		}

		// Update render target y coordinate
		if tile_y == top_tile {
			target_y += layer.tile_height - top_pixel;
		} else {
			target_y += layer.tile_height;
		}
	}
}

fn render_layer_with_renderer(render_size: &RenderSize, render_buf: &mut Vec<Vec<u16>>,
	game: &GameState, scroll_x: isize, scroll_y: isize, layer: &MapLayer,
	tile_renderer: &Fn(&mut [u16], &[u8], usize, usize, &Option<PaletteWithOffset>, &Fn(&mut u16, u16))) {
	match layer.alpha {
		0 => {
			match layer.blend_mode {
				BlendMode::Normal =>
					render_layer_with_blending(render_size, render_buf, game, layer,
						scroll_x, scroll_y, tile_renderer, &normal_blend),
				BlendMode::Add =>
					render_layer_with_blending(render_size, render_buf, game, layer,
						scroll_x, scroll_y, tile_renderer, &add_blend),
				BlendMode::Subtract =>
					render_layer_with_blending(render_size, render_buf, game, layer,
						scroll_x, scroll_y, tile_renderer, &subtract_blend),
				BlendMode::Multiply =>
					render_layer_with_blending(render_size, render_buf, game, layer,
						scroll_x, scroll_y, tile_renderer, &multiply_blend)
			};
		},
		alpha => {
			match layer.blend_mode {
				BlendMode::Normal =>
					render_layer_with_blending(render_size, render_buf, game, layer, scroll_x, scroll_y,
						tile_renderer, &|pixel, color| alpha_blend(pixel, color, alpha, &normal_blend)),
				BlendMode::Add =>
					render_layer_with_blending(render_size, render_buf, game, layer, scroll_x, scroll_y,
						tile_renderer, &|pixel, color| alpha_blend(pixel, color, alpha, &add_blend)),
				BlendMode::Subtract =>
					render_layer_with_blending(render_size, render_buf, game, layer, scroll_x, scroll_y,
						tile_renderer, &|pixel, color| alpha_blend(pixel, color, alpha, &subtract_blend)),
				BlendMode::Multiply =>
					render_layer_with_blending(render_size, render_buf, game, layer, scroll_x, scroll_y,
						tile_renderer, &|pixel, color| alpha_blend(pixel, color, alpha, &multiply_blend)),
			};
		}
	};
}

fn render_layer(render_size: &RenderSize, render_buf: &mut Vec<Vec<u16>>, game: &GameState,
	scroll_x: isize, scroll_y: isize, layer: &MapLayer) {
	match layer.tile_depth {
		4 => render_layer_with_renderer(render_size, render_buf, game, scroll_x, scroll_y, &layer, &render_tile_4bit),
		8 => render_layer_with_renderer(render_size, render_buf, game, scroll_x, scroll_y, &layer, &render_tile_8bit),
		16 => render_layer_with_renderer(render_size, render_buf, game, scroll_x, scroll_y, &layer, &render_tile_16bit),
		_ => panic!("Invalid tile bit depth {}", layer.tile_depth)
	};
}

fn render_sprite_with_renderer(render_size: &RenderSize, render_buf: &mut Vec<Vec<u16>>,
	x: isize, y: isize, animation: &SpriteAnimation, frame: usize,
	tile_renderer: &Fn(&mut [u16], &[u8], usize, usize, &Option<PaletteWithOffset>, &Fn(&mut u16, u16))) {
	if (x >= render_size.width as isize) || (y >= render_size.height as isize) ||
		(x <= -(animation.width as isize)) || (y <= -(animation.height as isize)) {
		return;
	}

	let mut x_offset = 0;
	let mut y_offset = 0;
	let x_start;
	let y_start;
	let mut width = animation.width;
	let mut height = animation.height;

	if x < 0 {
		x_offset = (-x) as usize;
		width -= x_offset;
		x_start = 0;
	} else {
		x_start = x as usize;
	}

	if y < 0 {
		y_offset = (-y) as usize;
		height -= y_offset;
		y_start = 0;
	} else {
		y_start = y as usize;
	}

	if (x_start + width) > render_size.width {
		width = render_size.width - x_start;
	}

	if (y_start + height) > render_size.height {
		height = render_size.height - y_start;
	}

	let sprite_data = animation.data_for_time(frame);
	let pitch = ((animation.width * animation.depth) + 7) / 8;

	for pixel_y in 0..height {
		let row_data = &sprite_data[(y_offset + pixel_y) * pitch .. (y_offset + pixel_y + 1) * pitch];
		let render_buf_row = &mut render_buf[y_start + pixel_y];
		let render_buf_tile = &mut render_buf_row[x_start .. x_start + width];
		tile_renderer(render_buf_tile, row_data, x_offset, width, &animation.palette, &normal_blend);
	}
}

fn render_sprite(render_size: &RenderSize, render_buf: &mut Vec<Vec<u16>>, x: isize, y: isize,
	animation: &SpriteAnimation, frame: usize) {
	match animation.depth {
		4 => render_sprite_with_renderer(render_size, render_buf, x, y, animation, frame, &render_tile_4bit),
		8 => render_sprite_with_renderer(render_size, render_buf, x, y, animation, frame, &render_tile_8bit),
		16 => render_sprite_with_renderer(render_size, render_buf, x, y, animation, frame, &render_tile_16bit),
		_ => panic!("Invalid sprite bit depth {}", animation.depth)
	};
}

pub fn render_frame(render_size: &RenderSize, render_buf: &mut Vec<Vec<u16>>, game: &GameState) {
	// Fill initial frame with map's background color
	let background_color = game.map.background_color;
	for y in 0..render_size.height {
		let row = &mut render_buf[y];
		for x in 0..render_size.width {
			row[x] = background_color;
		}
	}

	// Render each map layer
	for layer in &game.map.layers {
		render_layer(render_size, render_buf, game, game.scroll_x, game.scroll_y, &layer);
	}

	for layer in &game.ui_layers {
		layer.borrow_mut().update(&game);

		// Render UI layer centered in render area
		let layer_ref = layer.borrow();
		let map_layer = layer_ref.get_map_layer();
		let layer_width = map_layer.width * map_layer.tile_width;
		let layer_height = map_layer.height * map_layer.tile_height;
		let scroll_x = (render_size.width as isize - layer_width as isize) / 2;
		let scroll_y = (render_size.height as isize - layer_height as isize) / 2;
		render_layer(render_size, render_buf, game, scroll_x, scroll_y, map_layer);
	}

	for actor in &game.actors {
		let actor_ref = actor.borrow();
		let actor_info = actor_ref.actor_info();
		for sprite in &actor_info.sprites {
			render_sprite(render_size, render_buf, actor_info.x + sprite.x_offset - game.scroll_x,
				actor_info.y + sprite.y_offset - game.scroll_y, &sprite.animation, sprite.animation_frame);
		}
	}
}
