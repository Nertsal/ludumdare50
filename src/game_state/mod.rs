use geng::{Camera2d, Draw2d};

use super::*;

type Position = Vec2<i32>;
const TILE_SIZE: Vec2<f32> = vec2(1.0, 1.0);
const PLAYER_RADIUS: f32 = 0.25;
const PLAYER_COLOR: Color<f32> = Color::BLUE;

struct Player {
    pub position: Position,
}

pub struct GameState {
    geng: Geng,
    assets: Rc<Assets>,
    camera: Camera2d,
    player: Player,
}

impl GameState {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            camera: Camera2d {
                center: Vec2::ZERO,
                rotation: 0.0,
                fov: 10.0,
            },
            player: Player {
                position: vec2(0, 0),
            },
        }
    }
}

impl geng::State for GameState {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Color::BLACK), None);

        draw_2d::Ellipse::circle(
            self.player.position.map(|x| x as f32) * TILE_SIZE,
            PLAYER_RADIUS,
            PLAYER_COLOR,
        )
        .draw_2d(&self.geng, framebuffer, &self.camera);
    }

    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::KeyDown { key } => match key {
                geng::Key::Left => {
                    self.player.position.x -= 1;
                }
                geng::Key::Right => {
                    self.player.position.x += 1;
                }
                geng::Key::Down => {
                    self.player.position.y -= 1;
                }
                geng::Key::Up => {
                    self.player.position.y += 1;
                }
                _ => {}
            },
            _ => {}
        }
    }
}
