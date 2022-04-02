use std::collections::VecDeque;

use geng::Camera2d;

use super::*;

use renderer::*;

type Coord = i32;
type Position = Vec2<Coord>;

const TILE_SIZE: Vec2<f32> = vec2(1.0, 1.0);
const UNIT_RADIUS: f32 = 0.25;
const GRID_WIDTH: f32 = 0.05;
const GRID_COLOR: Color<f32> = Color::GRAY;
const ACTIONS_OFFSET: f32 = 25.0;
const ACTIONS_WIDTH: f32 = 100.0;

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
    SingleDouble { is_next_single: bool },
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
            Self::SingleDouble { is_next_single } => {
                let delta = if *is_next_single {
                    Self::Direct.move_towards(target)
                } else {
                    Self::Direct.move_towards(target) * 2
                };
                *is_next_single = !*is_next_single;
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

pub enum Action {
    AttackDirect,
}

#[derive(Default)]
pub struct ActionQueue {
    actions: VecDeque<Option<Action>>,
}

impl ActionQueue {
    pub fn iter(&self) -> impl Iterator<Item = &Option<Action>> {
        self.actions.iter()
    }

    pub fn pop(&mut self) -> Option<Action> {
        self.actions.pop_front().flatten()
    }

    pub fn enqueue(&mut self, action: Action, time: usize) {
        if self.actions.len() <= time {
            for _ in 0..time - self.actions.len() {
                self.actions.push_back(None);
            }
            self.actions.push_back(Some(action));
            return;
        }
        self.actions[time] = Some(action);
    }
}

pub struct GameState {
    geng: Geng,
    assets: Rc<Assets>,
    camera: Camera2d,
    arena_bounds: AABB<Coord>,
    player_actions: ActionQueue,
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
                fov: 15.0,
            },
            player_actions: ActionQueue::default(),
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
                    movement: MovementType::SingleDouble {
                        is_next_single: true,
                    },
                },
            ],
        }
    }

    pub fn tick(&mut self, player_move: Position) {
        // Move player
        self.player.position = clamp_pos(self.player.position + player_move, self.arena_bounds);

        // Move enemies
        for enemy in &mut self.enemies {
            let delta = self.player.position - enemy.position;
            enemy.position = clamp_pos(
                enemy.position + enemy.movement.move_towards(delta),
                self.arena_bounds,
            );
        }

        // Player actions
        for action in self.player_actions.pop() {
            match action {
                Action::AttackDirect => {
                    warn!("TODO: AttackDirect");
                }
            }
        }

        // Gen next action
        if global_rng().gen_bool(0.1) {
            self.player_actions.enqueue(Action::AttackDirect, 0);
        }
    }
}

impl geng::State for GameState {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        let framebuffer_size = framebuffer.size().map(|x| x as f32);
        ugli::clear(framebuffer, Some(Color::BLACK), None);
        let mut renderer = Renderer::new(&self.geng, &self.assets, &self.camera, framebuffer);

        // Enemies
        for enemy in &self.enemies {
            renderer.draw_circle(
                enemy.position.map(|x| x as f32) * TILE_SIZE,
                UNIT_RADIUS,
                enemy.color,
            );
        }

        // Player
        renderer.draw_circle(
            self.player.position.map(|x| x as f32) * TILE_SIZE,
            UNIT_RADIUS,
            self.player.color,
        );

        // Grid
        renderer.draw_grid(self.arena_bounds, TILE_SIZE, GRID_WIDTH, GRID_COLOR);

        let mut renderer = Renderer::new(
            &self.geng,
            &self.assets,
            &geng::PixelPerfectCamera,
            framebuffer,
        );

        // Actions
        renderer.draw_actions(
            &self.player_actions,
            5,
            AABB::from_corners(
                vec2(
                    framebuffer_size.x - ACTIONS_WIDTH - ACTIONS_OFFSET,
                    ACTIONS_OFFSET,
                ),
                framebuffer_size.map(|x| x - ACTIONS_OFFSET),
            ),
        );
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
