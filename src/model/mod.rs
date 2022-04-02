use geng::Camera2d;

use super::*;

use renderer::*;

type Coord = i32;
type Position = Vec2<Coord>;
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

impl MovementType {
    pub fn move_towards(&mut self, target: Position) -> Position {
        match self {
            Self::Direct => {
                if target.x.abs() >= target.y.abs() {
                    vec2(target.x.signum(), 0)
                } else {
                    vec2(0, target.y.signum())
                }
            }
            Self::Neighbour => vec2(target.x.signum(), target.y.signum()),
            Self::SingleDouble { isNextSingle } => {
                let delta = if *isNextSingle {
                    Self::Direct.move_towards(target)
                } else {
                    Self::Direct.move_towards(target) * 2
                };
                *isNextSingle = !*isNextSingle;
                delta
            }
        }
    }
}

fn clamp_pos(pos: Position, aabb: AABB<Coord>) -> Position {
    vec2(
        pos.x.clamp(aabb.x_min, aabb.x_max),
        pos.y.clamp(aabb.y_min, aabb.y_max),
    )
}

pub struct GameState {
    geng: Geng,
    assets: Rc<Assets>,
    camera: Camera2d,
    arena_bounds: AABB<Coord>,
    player: Player,
    enemies: Vec<Enemy>,
}

impl GameState {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            arena_bounds: AABB::from_corners(vec2(-4, -4), vec2(5, 5)),
            camera: Camera2d {
                center: Vec2::ZERO,
                rotation: 0.0,
                fov: 10.0,
            },
            player: Player {
                color: Color::BLUE,
                position: vec2(0, 0),
            },
            enemies: vec![
                Enemy {
                    color: Color::RED,
                    position: vec2(5, 5),
                    movement: MovementType::Direct,
                },
                Enemy {
                    color: Color::GREEN,
                    position: vec2(-4, -4),
                    movement: MovementType::Neighbour,
                },
                Enemy {
                    color: Color::MAGENTA,
                    position: vec2(-4, 5),
                    movement: MovementType::SingleDouble { isNextSingle: true },
                },
            ],
        }
    }

    pub fn tick(&mut self, player_move: Position) {
        self.player.position = clamp_pos(self.player.position + player_move, self.arena_bounds);

        for enemy in &mut self.enemies {
            let delta = self.player.position - enemy.position;
            enemy.position = clamp_pos(
                enemy.position + enemy.movement.move_towards(delta),
                self.arena_bounds,
            );
        }
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
                geng::Key::Space => {
                    self.tick(vec2(0, 0));
                }
                _ => {}
            },
            _ => {}
        }
    }
}
