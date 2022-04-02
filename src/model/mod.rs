use geng::Camera2d;

use super::*;

use renderer::*;

type Position = Vec2<i32>;
const TILE_SIZE: Vec2<f32> = vec2(1.0, 1.0);
const UNIT_RADIUS: f32 = 0.25;

struct Player {
    pub color: Color<f32>,
    pub position: Position,
}

struct Enemy {
    pub color: Color<f32>,
    pub position: Position,
    pub movement: MovementType,
}

enum MovementType {
    Direct,
    Neighbour,
    SingleDouble { isNextSingle: bool },
}

pub struct GameState {
    geng: Geng,
    assets: Rc<Assets>,
    camera: Camera2d,
    player: Player,
    enemies: Vec<Enemy>,
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
                color: Color::BLUE,
                position: vec2(0, 0),
            },
            enemies: vec![],
        }
    }

    pub fn tick(&mut self, player_move: Position) {
        self.player.position += player_move;
    }
}

impl geng::State for GameState {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Color::BLACK), None);

        let mut renderer = Renderer::new(&self.geng, &self.camera, framebuffer);

        renderer.draw_circle(
            self.player.position.map(|x| x as f32) * TILE_SIZE,
            UNIT_RADIUS,
            self.player.color,
        );

        for enemy in &self.enemies {
            renderer.draw_circle(
                enemy.position.map(|x| x as f32) * TILE_SIZE,
                UNIT_RADIUS,
                enemy.color,
            );
        }
    }

    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::KeyDown { key } => match key {
                geng::Key::Left => {
                    self.tick(vec2(-1, 0));
                }
                geng::Key::Right => {
                    self.tick(vec2(1, 0));
                }
                geng::Key::Down => {
                    self.tick(vec2(0, -1));
                }
                geng::Key::Up => {
                    self.tick(vec2(0, 1));
                }
                _ => {}
            },
            _ => {}
        }
    }
}
