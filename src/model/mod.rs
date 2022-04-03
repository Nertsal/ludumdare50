mod draw;
mod init;

use geng::Camera2d;

use super::*;

use renderer::*;

pub type Coord = i32;
pub type Time = i32;
pub type Score = u32;
pub type Position = Vec2<Coord>;

pub const PLAYER_COLOR: Color<f32> = Color::BLUE;
pub const INTERPOLATION_SPEED: f32 = 10.0;

// Things in world coordinates
pub const TILE_SIZE: Vec2<f32> = vec2(1.0, 1.0);
pub const UNIT_RADIUS: f32 = 0.25;
pub const GRID_WIDTH: f32 = 0.05;
pub const GRID_COLOR: Color<f32> = Color::GRAY;
pub const DAMAGE_WIDTH: f32 = 0.025;
pub const DAMAGE_COLOR: Color<f32> = Color::RED;
pub const DAMAGE_EXTRA_SPACE: f32 = 0.25;
pub const UPGRADE_SIZE: Vec2<f32> = vec2(100.0, 100.0);
pub const UPGRADE_EXTRA_SPACE: f32 = 50.0;
pub const UPGRADE_FRAME_WIDTH: f32 = 1.0;
pub const UPGRADE_FRAME_COLOR: Color<f32> = Color::GREEN;
pub const UPGRADE_BACKGROUND_COLOR: Color<f32> = Color {
    r: 0.3,
    g: 0.3,
    b: 0.3,
    a: 0.7,
};
pub const UPGRADE_TEXT_COLOR: Color<f32> = Color::WHITE;
pub const UPGRADE_SELECTED_COLOR: Color<f32> = Color {
    r: 0.5,
    g: 0.5,
    b: 0.5,
    a: 0.8,
};

// Things in screen coordinates
pub const ATTACKS_OFFSET: f32 = 25.0;
pub const ATTACKS_WIDTH: f32 = 300.0;
pub const ATTACKS_BORDER_WIDTH: f32 = 5.0;
pub const ATTACKS_BORDER_COLOR: Color<f32> = Color::GRAY;
pub const ULTIMATE_HEIGHT: f32 = 300.0;

#[derive(Debug, Clone)]
pub struct Player {
    pub color: Color<f32>,
    pub position: Position,
    pub render_pos: Vec2<f32>,
}

#[derive(Debug, Clone)]
pub struct Enemy {
    pub typ: EnemyType,
    pub color: Color<f32>,
    pub position: Position,
    pub render_pos: Vec2<f32>,
    pub movement: MovementType,
    pub is_dead: bool,
}

type Id = usize;

#[derive(Debug, Clone)]
pub enum Caster {
    Player,
    Enemy { id: Id },
}

#[derive(Debug, Clone)]
pub enum MovementType {
    Direct,
    Neighbour,
    SingleDouble { is_next_single: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EnemyType {
    Attacker,
    Frog,
    King,
}

#[derive(Debug, Clone)]
pub struct SpawnPrefab {
    pub movement: MovementType,
    pub min_score: Score,
    pub next_spawn: Time,
    pub color: Color<f32>,
    pub cooldowns: HashMap<usize, f32>,
    pub large_multiplier: f32,
    pub killed_siblings: usize,
}

pub struct Action {
    pub cooldown: Time,
    pub next: Time,
    pub cooldown_multiplier: f32,
}

pub struct Attack {
    pub action: Action,
    pub pattern: Vec<Position>,
    pub upgrade: Option<Box<Attack>>,
}

pub struct Teleport {
    pub action: Action,
    pub radius: Coord,
}

pub struct UpgradeInfo {
    pub current: usize,
    pub max: usize,
}

pub enum Upgrade {
    Global {
        info: UpgradeInfo,
        requirement: Score,
    },
    Attack {
        info: Vec<UpgradeInfo>,
    },
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum UpgradeType {
    NewAttack,
    IncUltRadius,
    ReduceUltCooldown,
    IncDeathTimer,
    ReduceAttackCooldown,
    UpgradeAttack,
}

pub struct UpgradeMenu {
    pub lvl_ups_left: usize,
    pub options: Vec<(UpgradeType, Vec<usize>)>,
    pub choice: usize,
}

pub struct Experience {
    pub level: u32,
    pub exp: Score,
    pub exp_to_next_lvl: Score,
}

pub struct GameState {
    pub geng: Geng,
    pub assets: Rc<Assets>,
    pub camera: Camera2d,
    pub arena_bounds: AABB<Coord>,
    pub highscore: AutoSave<Score>,
    pub score: Score,
    pub experience: Experience,
    pub player_attacks: Vec<Attack>,
    pub potential_attacks: Vec<Attack>,
    pub player_ultimate: Teleport,
    pub using_ultimate: Option<Position>,
    pub player: Player,
    pub enemies: Vec<Enemy>,
    pub damages: Vec<Position>,
    pub spawn_prefabs: HashMap<EnemyType, SpawnPrefab>,
    pub upgrades: HashMap<UpgradeType, Upgrade>,
    pub upgrade_menu: Option<UpgradeMenu>,
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
        ugli::clear(framebuffer, Some(Color::BLACK), None);
        self.draw(framebuffer);
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
                geng::Key::Enter => {
                    self.select_upgrade();
                }
                _ => {}
            },
            _ => {}
        }
    }
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

impl Action {
    pub fn new(cooldown: Time) -> Self {
        Self {
            cooldown,
            next: cooldown,
            cooldown_multiplier: 1.0,
        }
    }

    pub fn set_on_cooldown(&mut self) {
        self.next = (self.cooldown as f32 * self.cooldown_multiplier).ceil() as Time;
    }

    pub fn is_ready(&self) -> bool {
        self.next <= 0
    }

    pub fn update(&mut self, delta_time: Time) -> bool {
        self.next -= delta_time;
        self.is_ready()
    }
}

impl Attack {
    pub fn new(
        cooldown: Time,
        pattern: impl IntoIterator<Item = Position>,
        upgrade: Option<Attack>,
    ) -> Self {
        Self {
            action: Action::new(cooldown),
            pattern: pattern.into_iter().collect(),
            upgrade: upgrade.map(|attack| Box::new(attack)),
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

    pub fn upgrade(&mut self) {
        if let Some(attack) = self.upgrade.take() {
            *self = *attack;
        }
    }
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

impl UpgradeInfo {
    pub fn new(max_upgrades: usize) -> Self {
        Self {
            current: 0,
            max: max_upgrades,
        }
    }
}

impl Experience {
    pub fn new() -> Self {
        Self {
            level: 0,
            exp: 0,
            exp_to_next_lvl: 5,
        }
    }

    pub fn add_exp(&mut self, exp: Score) -> usize {
        self.exp += exp;
        let mut lvl_ups = 0;
        while self.exp >= self.exp_to_next_lvl {
            self.exp -= self.exp_to_next_lvl;
            lvl_ups += 1;
        }
        lvl_ups
    }
}
