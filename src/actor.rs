use std::rc::Rc;
use std::cell::{RefCell, Ref, RefMut};
use sprite::{Sprite, SpriteAnimation};
use game::GameState;

pub struct SpriteWithOffset {
	pub sprite: Rc<Sprite>,
	pub animation: Rc<SpriteAnimation>,
	pub animation_frame: usize,
	pub x_offset: isize,
	pub y_offset: isize
}

pub struct BoundingRect {
	pub x: isize,
	pub y: isize,
	pub width: isize,
	pub height: isize
}

pub struct ActorInfo {
	pub x: isize,
	pub y: isize,
	pub subpixel_x: u8,
	pub subpixel_y: u8,
	pub velocity_x: isize,
	pub velocity_y: isize,
	pub collision_bounds: Option<BoundingRect>,
	pub sprites: Vec<SpriteWithOffset>
}

#[derive(Clone)]
pub struct ActorRef {
	actor: Rc<RefCell<Box<Actor>>>
}

pub trait Actor {
	fn actor_info(&self) -> &ActorInfo;
	fn actor_info_mut(&mut self) -> &mut ActorInfo;

	fn update(&mut self, _game_state: &GameState) {}

	fn apply_move(&mut self, game_state: &GameState) {
		let actor_info = self.actor_info_mut();
		let mut full_x = (actor_info.x << 8) + actor_info.subpixel_x as isize;
		let mut full_y = (actor_info.y << 8) + actor_info.subpixel_y as isize;
		full_x += actor_info.velocity_x;
		full_y += actor_info.velocity_y;

		let mut new_x = full_x >> 8;
		let mut new_y = full_y >> 8;

		let collision_x_offset;
		let collision_y_offset;
		let collision_width;
		let collision_height;
		if let Some(collision_bounds) = &actor_info.collision_bounds {
			collision_x_offset = collision_bounds.x;
			collision_y_offset = collision_bounds.y;
			collision_width = collision_bounds.width;
			collision_height = collision_bounds.height;
		} else {
			collision_x_offset = 0;
			collision_y_offset = 0;
			collision_width = 1;
			collision_height = 1;
		}

		let mut bounds = BoundingRect {
			x: actor_info.x + collision_x_offset,
			y: actor_info.y + collision_y_offset,
			width: collision_width,
			height: collision_height
		};

		if let Some(revised_x) = game_state.map.sweep_collision_x(&bounds, new_x + collision_x_offset) {
			new_x = revised_x - collision_x_offset;
			full_x = new_x << 8;
			actor_info.velocity_x = 0;
		}

		bounds.x = new_x + collision_x_offset;

		if let Some(revised_y) = game_state.map.sweep_collision_y(&bounds, new_y + collision_y_offset) {
			new_y = revised_y - collision_y_offset;
			full_y = new_y << 8;
			actor_info.velocity_y = 0;
		}

		actor_info.x = full_x >> 8;
		actor_info.y = full_y >> 8;
		actor_info.subpixel_x = (full_x & 0xff) as u8;
		actor_info.subpixel_y = (full_y & 0xff) as u8;
	}

	fn tick(&mut self, game_state: &GameState) {
		self.update(game_state);
		self.apply_move(game_state);
	}

	fn add_sprite(&mut self, sprite: Rc<Sprite>, x_offset: isize, y_offset: isize) {
		let actor_info = self.actor_info_mut();
		let animation = sprite.get_default_animation();
		actor_info.sprites.push(SpriteWithOffset {
			sprite,
			animation,
			animation_frame: 0,
			x_offset, y_offset
		});
	}

	fn start_animation(&mut self, name: &str) {
		let actor_info = self.actor_info_mut();
		for sprite in &mut actor_info.sprites {
			if let Some(animation) = sprite.sprite.get_animation_by_name(name) {
				if !Rc::ptr_eq(&animation, &sprite.animation) {
					sprite.animation = animation;
					sprite.animation_frame = 0;
				}
			}
		}
	}

	fn set_collision_bounds(&mut self, bounds: BoundingRect) {
		let actor_info = self.actor_info_mut();
		actor_info.collision_bounds = Some(bounds);
	}

	fn on_button_down(&mut self, _name: &str) {}
	fn on_button_up(&mut self, _name: &str) {}
}

impl ActorInfo {
	pub fn new(x: isize, y: isize) -> ActorInfo {
		ActorInfo {
			x, y,
			subpixel_x: 0,
			subpixel_y: 0,
			velocity_x: 0,
			velocity_y: 0,
			collision_bounds: None,
			sprites: Vec::new()
		}
	}
}

impl ActorRef {
	pub fn new(actor: Box<Actor>) -> ActorRef {
		ActorRef {
			actor: Rc::new(RefCell::new(actor))
		}
	}

	pub fn borrow(&self) -> Ref<Box<Actor>> {
		self.actor.borrow()
	}

	pub fn borrow_mut(&self) -> RefMut<Box<Actor>> {
		self.actor.borrow_mut()
	}
}