use geng::Camera2d;

use super::*;

use renderer::*;

type Coord = i32;
type Time = i32;
type Score = u32;
type Position = Vec2<Coord>;

pub const PLAYER_COLOR: Color<f32> = Color::BLUE;
const INTERPOLATION_SPEED: f32 = 10.0;

// Things in world coordinates
const TILE_SIZE: Vec2<f32> = vec2(1.0, 1.0);
const UNIT_RADIUS: f32 = 0.25;
const GRID_WIDTH: f32 = 0.05;
const GRID_COLOR: Color<f32> = Color::GRAY;
const DAMAGE_WIDTH: f32 = 0.025;
const DAMAGE_COLOR: Color<f32> = Color::RED;
const DAMAGE_EXTRA_SPACE: f32 = 0.25;

// Things in screen coordinates
const ATTACKS_OFFSET: f32 = 25.0;
const ATTACKS_WIDTH: f32 = 300.0;
const ATTACKS_BORDER_WIDTH: f32 = 5.0;
const ATTACKS_BORDER_COLOR: Color<f32> = Color::GRAY;
const ULTIMATE_HEIGHT: f32 = 300.0;

#[derive(Debug, Clone)]
struct Player {
    pub color: Color<f32>,
    pub position: Position,
    pub render_pos: Vec2<f32>,
}

#[derive(Debug, Clone)]
struct Enemy {
    pub typ: EnemyType,
    pub color: Color<f32>,
    pub position: Position,
    pub render_pos: Vec2<f32>,
    pub movement: MovementType,
    pub is_dead: bool,
}

type Id = usize;

#[derive(Debug, Clone)]
enum Caster {
    Player,
    Enemy { id: Id },
}

#[derive(Debug, Clone)]
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

pub fn grid_cell_aabb(cell_pos: Position, tile_size: Vec2<f32>) -> AABB<f32> {
    AABB::point(cell_pos.map(|x| x as f32) * tile_size).extend_symmetric(tile_size / 2.0)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum EnemyType {
    Attacker,
    Frog,
    King,
}

#[derive(Debug, Clone)]
struct SpawnPrefab {
    movement: MovementType,
    min_score: Score,
    next_spawn: Time,
    color: Color<f32>,
    cooldowns: HashMap<usize, f32>,
    large_multiplier: f32,
    killed_siblings: usize,
}

impl SpawnPrefab {
    pub fn refresh_cooldown(&mut self, siblings: usize) {
        let killed_multiplier = 1.0 - self.killed_siblings as f32 * 0.05;
        let cooldown = self
            .cooldowns
            .get(&siblings)
            .copied()
            .unwrap_or(self.large_multiplier)
            * killed_multiplier;
        self.next_spawn = cooldown.ceil() as _;
    }
}

pub struct Action {
    cooldown: Time,
    next: Time,
}

impl Action {
    pub fn new(cooldown: Time) -> Self {
        Self {
            cooldown,
            next: cooldown,
        }
    }

    pub fn set_on_cooldown(&mut self) {
        self.next = self.cooldown;
    }

    pub fn is_ready(&self) -> bool {
        self.next <= 0
    }

    pub fn update(&mut self, delta_time: Time) -> bool {
        self.next -= delta_time;
        self.is_ready()
    }
}

pub struct Attack {
    action: Action,
    pattern: Vec<Position>,
}

impl Attack {
    pub fn new(cooldown: Time, pattern: impl IntoIterator<Item = Position>) -> Self {
        Self {
            action: Action::new(cooldown),
            pattern: pattern.into_iter().collect(),
        }
    }

    pub fn rotate_left(&mut self) {
        for pos in &mut self.pattern {
            *pos = vec2(-pos.y, pos.x);
        }
    }

    pub fn attack_positions(&self, caster_pos: Position) -> impl Iterator<Item = Position> + '_ {
        self.pattern.iter().map(move |delta| caster_pos + *delta)
    }
}

pub struct Teleport {
    action: Action,
    radius: Coord,
}

impl Teleport {
    pub fn new(cooldown: Time, radius: Coord) -> Self {
        Self {
            action: Action::new(cooldown),
            radius,
        }
    }

    pub fn boundary(&self) -> AABB<Coord> {
        AABB::ZERO.extend_uniform(self.radius)
    }

    pub fn deltas(&self) -> impl Iterator<Item = Position> + '_ {
        (-self.radius..=self.radius)
            .flat_map(|x| (-self.radius..=self.radius).map(move |y| vec2(x, y)))
            .filter(|pos| {
                *pos != Vec2::ZERO && pos.x.abs() <= self.radius && pos.y.abs() <= self.radius
            })
    }
}

pub struct GameState {
    geng: Geng,
    assets: Rc<Assets>,
    camera: Camera2d,
    arena_bounds: AABB<Coord>,
    highscore: AutoSave<Score>,
    score: Score,
    player_attacks: Vec<Attack>,
    player_ultimate: Teleport,
    using_ultimate: Option<Position>,
    player: Player,
    enemies: Vec<Enemy>,
    damages: Vec<Position>,
    spawn_prefabs: HashMap<EnemyType, SpawnPrefab>,
}

impl GameState {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            arena_bounds: AABB::from_corners(vec2(-4, -4), vec2(5, 5)),
            highscore: AutoSave::load(static_path().join("highscore.json").to_str().unwrap()),
            score: 0,
            using_ultimate: None,
            camera: Camera2d {
                center: Vec2::ZERO,
                rotation: 0.0,
                fov: 15.0,
            },
            player: Player {
                color: PLAYER_COLOR,
                position: vec2(0, 0),
                render_pos: vec2(0.0, 0.0),
            },
            enemies: vec![],
            damages: vec![],
            player_attacks: vec![
                Attack::new(2, [vec2(1, 0)]),
                // Attack::new(2, [vec2(1, 0), vec2(2, 1)]),
                // Attack::new(2, [vec2(1, 0), vec2(2, 0), vec2(1, 1)]),
                // Attack::new(2, [vec2(1, 0), vec2(2, 0), vec2(3, 0), vec2(3, 1)]),
            ],
            player_ultimate: Teleport::new(5, 2),
            spawn_prefabs: [
                (
                    EnemyType::Attacker,
                    SpawnPrefab {
                        movement: MovementType::Direct,
                        min_score: 0,
                        next_spawn: 1,
                        color: Color::RED,
                        cooldowns: [(0, 2.0), (1, 4.0), (2, 6.0), (3, 7.0)]
                            .into_iter()
                            .collect(),
                        large_multiplier: 8.0,
                        killed_siblings: 0,
                    },
                ),
                (
                    EnemyType::Frog,
                    SpawnPrefab {
                        movement: MovementType::SingleDouble {
                            is_next_single: true,
                        },
                        min_score: 10,
                        next_spawn: 1,
                        color: Color::GREEN,
                        cooldowns: [(0, 6.0), (1, 12.0), (2, 12.0), (3, 18.0)]
                            .into_iter()
                            .collect(),
                        large_multiplier: 20.0,
                        killed_siblings: 0,
                    },
                ),
                (
                    EnemyType::King,
                    SpawnPrefab {
                        movement: MovementType::Neighbour,
                        min_score: 60,
                        next_spawn: 1,
                        color: Color::MAGENTA,
                        cooldowns: [(0, 6.0), (1, 10.0), (2, 15.0), (3, 15.0)]
                            .into_iter()
                            .collect(),
                        large_multiplier: 18.0,
                        killed_siblings: 0,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        }
    }

    pub fn tick(&mut self, player_move: Position) {
        // Move player

        self.player.position = clamp_pos(self.player.position + player_move, self.arena_bounds);

        if let Some(origin) = self.using_ultimate {
            self.player.position = clamp_pos(
                self.player.position,
                self.player_ultimate.boundary().translate(origin),
            );
            return;
        }

        self.damages = vec![];

        // self.player_collide();

        // Move enemies
        for enemy in &mut self.enemies {
            let delta = self.player.position - enemy.position;
            enemy.position = clamp_pos(
                enemy.position + enemy.movement.move_towards(delta),
                self.arena_bounds,
            );
        }

        // self.player_collide();

        // Player actions
        let mut attack_positions = Vec::new();
        for attack in &mut self.player_attacks {
            if attack.action.update(1) {
                attack.action.set_on_cooldown();
                attack_positions.extend(attack.attack_positions(self.player.position));
            }
        }
        self.player_ultimate.action.update(1);
        self.attack_positions(Caster::Player, &attack_positions);

        // Count siblings
        let mut siblings = HashMap::new();
        for (enemy_type, _) in &self.spawn_prefabs {
            siblings.insert(enemy_type.clone(), 0);
        }
        for enemy in &self.enemies {
            *siblings.get_mut(&enemy.typ).unwrap() += 1;
        }

        // Spawn new enemies
        for (enemy_type, prefab) in self
            .spawn_prefabs
            .iter_mut()
            .filter(|(_, prefab)| self.score >= prefab.min_score)
        {
            prefab.next_spawn -= 1;
            if prefab.next_spawn <= 0 {
                let sibs = siblings.get_mut(enemy_type).unwrap();
                *sibs += 1;
                prefab.refresh_cooldown(*sibs);
                let spawn_points = self.arena_bounds.corners();
                let &spawn_point = spawn_points
                    .choose(&mut global_rng())
                    .expect("Failed to find a spawn point");
                let enemy = Enemy {
                    typ: enemy_type.clone(),
                    color: prefab.color,
                    position: spawn_point,
                    render_pos: spawn_point.map(|x| x as f32),
                    movement: prefab.movement.clone(),
                    is_dead: false,
                };
                self.enemies.push(enemy);
            }
        }
    }

    fn get_in_point(&self, position: Position) -> Option<Caster> {
        let mut units = std::iter::once((Caster::Player, self.player.position)).chain(
            self.enemies
                .iter()
                .enumerate()
                .map(|(id, enemy)| (Caster::Enemy { id }, enemy.position)),
        );
        units
            .find(|(_, unit_pos)| *unit_pos == position)
            .map(|(caster, _)| caster)
    }

    fn attack_positions(&mut self, caster: Caster, positions: &[Position]) {
        self.damages.extend(positions);
        match caster {
            Caster::Player => {
                for enemy in &mut self.enemies {
                    if positions.contains(&enemy.position) {
                        enemy.is_dead = true;
                    }
                }
                self.enemies.retain(|enemy| {
                    if enemy.is_dead {
                        self.score += 1;
                        *self.highscore = (*self.highscore).max(self.score);
                        self.spawn_prefabs
                            .get_mut(&enemy.typ)
                            .unwrap()
                            .killed_siblings += 1;
                    }
                    !enemy.is_dead
                });
            }
            Caster::Enemy { id } => todo!(),
        }
    }

    fn use_ultimate(&mut self) {
        if self.using_ultimate.is_some() {
            self.using_ultimate = None;
        } else if self.player_ultimate.action.is_ready() {
            self.using_ultimate = Some(self.player.position);
            self.player_ultimate.action.set_on_cooldown();
        }
    }
}

impl geng::State for GameState {
    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;

        // Interpolate player and enemies
        self.player.render_pos += (self.player.position.map(|x| x as f32) - self.player.render_pos)
            .clamp_len(..=INTERPOLATION_SPEED * delta_time);
        for enemy in &mut self.enemies {
            enemy.render_pos += (enemy.position.map(|x| x as f32) - enemy.render_pos)
                .clamp_len(..=INTERPOLATION_SPEED * delta_time);
        }
    }

    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        let framebuffer_size = framebuffer.size().map(|x| x as f32);
        ugli::clear(framebuffer, Some(Color::BLACK), None);
        let mut renderer = Renderer::new(&self.geng, &self.assets, &self.camera, framebuffer);

        // Enemies
        for enemy in &self.enemies {
            renderer.draw_circle(enemy.render_pos * TILE_SIZE, UNIT_RADIUS, enemy.color);
        }

        // Player
        renderer.draw_circle(
            self.player.render_pos * TILE_SIZE,
            UNIT_RADIUS,
            self.player.color,
        );

        // Damage
        for &pos in &self.damages {
            let aabb = grid_cell_aabb(pos, TILE_SIZE).extend_uniform(-DAMAGE_EXTRA_SPACE);
            renderer.draw_cross(aabb, DAMAGE_WIDTH, DAMAGE_COLOR);
        }

        // Grid
        renderer.draw_grid(
            self.arena_bounds,
            TILE_SIZE,
            -TILE_SIZE / 2.0,
            GRID_WIDTH,
            GRID_COLOR,
        );

        // Ultimate
        if let Some(origin) = self.using_ultimate {
            for pos in self
                .player_ultimate
                .deltas()
                .map(|pos| pos + origin)
                .map(|pos| model::grid_cell_aabb(pos, TILE_SIZE).center())
            {
                renderer.draw_circle(pos, 0.1, Color::MAGENTA);
            }
        }

        let mut renderer = Renderer::new(
            &self.geng,
            &self.assets,
            &geng::PixelPerfectCamera,
            framebuffer,
        );

        // Attacks
        let attacks_aabb = AABB::from_corners(
            vec2(
                framebuffer_size.x - ATTACKS_WIDTH - ATTACKS_OFFSET,
                ATTACKS_OFFSET + ULTIMATE_HEIGHT,
            ),
            framebuffer_size.map(|x| x - ATTACKS_OFFSET),
        );
        renderer.draw_attacks(
            &self.player_attacks,
            4,
            attacks_aabb,
            ATTACKS_BORDER_WIDTH,
            ATTACKS_BORDER_COLOR,
        );

        // Ultimate
        let ultimate_aabb = AABB::from_corners(
            attacks_aabb.bottom_left(),
            vec2(framebuffer_size.x - ATTACKS_OFFSET, ATTACKS_OFFSET),
        );
        renderer.draw_ultimate(
            &self.player_ultimate,
            ultimate_aabb,
            ATTACKS_BORDER_WIDTH,
            ATTACKS_BORDER_COLOR,
            10.0,
        );

        // Score text
        renderer.draw_text(
            &format!("Score: {}", self.score),
            vec2(10.0, framebuffer_size.y - 10.0),
            vec2(0.0, 1.0),
            20.0,
            Color::GRAY,
        );
        renderer.draw_text(
            &format!("High Score: {}", *self.highscore),
            vec2(10.0, framebuffer_size.y - 100.0),
            vec2(0.0, 1.0),
            20.0,
            Color::GRAY,
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
                    self.use_ultimate();
                }
                _ => {}
            },
            _ => {}
        }
    }
}
