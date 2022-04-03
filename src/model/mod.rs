mod init;
mod draw;

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
const UPGRADE_SIZE: Vec2<f32> = vec2(100.0, 100.0);
const UPGRADE_EXTRA_SPACE: f32 = 50.0;
const UPGRADE_FRAME_WIDTH: f32 = 1.0;
const UPGRADE_FRAME_COLOR: Color<f32> = Color::GREEN;
const UPGRADE_BACKGROUND_COLOR: Color<f32> = Color {
    r: 0.3,
    g: 0.3,
    b: 0.3,
    a: 0.7,
};
const UPGRADE_TEXT_COLOR: Color<f32> = Color::WHITE;
const UPGRADE_SELECTED_COLOR: Color<f32> = Color {
    r: 0.5,
    g: 0.5,
    b: 0.5,
    a: 0.8,
};

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
    cooldown_multiplier: f32,
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

pub struct Attack {
    action: Action,
    pattern: Vec<Position>,
    upgrade: Option<Box<Attack>>,
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

struct UpgradeInfo {
    current: usize,
    max: usize,
}

impl UpgradeInfo {
    pub fn new(max_upgrades: usize) -> Self {
        Self {
            current: 0,
            max: max_upgrades,
        }
    }
}

enum Upgrade {
    Global {
        info: UpgradeInfo,
        requirement: Score,
    },
    Attack {
        info: Vec<UpgradeInfo>,
    },
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum UpgradeType {
    NewAttack,
    IncUltRadius,
    ReduceUltCooldown,
    IncDeathTimer,
    ReduceAttackCooldown,
    UpgradeAttack,
}

struct UpgradeMenu {
    lvl_ups_left: usize,
    options: Vec<(UpgradeType, Vec<usize>)>,
    choice: usize,
}

struct Experience {
    level: u32,
    exp: Score,
    exp_to_next_lvl: Score,
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

pub struct GameState {
    geng: Geng,
    assets: Rc<Assets>,
    camera: Camera2d,
    arena_bounds: AABB<Coord>,
    highscore: AutoSave<Score>,
    score: Score,
    experience: Experience,
    player_attacks: Vec<Attack>,
    potential_attacks: Vec<Attack>,
    player_ultimate: Teleport,
    using_ultimate: Option<Position>,
    player: Player,
    enemies: Vec<Enemy>,
    damages: Vec<Position>,
    spawn_prefabs: HashMap<EnemyType, SpawnPrefab>,
    upgrades: HashMap<UpgradeType, Upgrade>,
    upgrade_menu: Option<UpgradeMenu>,
}

impl GameState {
    pub fn tick(&mut self, player_move: Position) {
        if let Some(upgrade_menu) = &mut self.upgrade_menu {
            let mut choice = upgrade_menu.choice as isize + player_move.x.signum() as isize;
            let min = 0;
            let max = upgrade_menu.options.len() as isize - 1;
            if choice < min {
                choice = max;
            } else if choice > max {
                choice = min;
            }
            upgrade_menu.choice = choice as usize;
            return;
        }

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
                let mut lvl_ups = 0;
                self.enemies.retain(|enemy| {
                    if enemy.is_dead {
                        self.score += 1;
                        *self.highscore = (*self.highscore).max(self.score);
                        lvl_ups += self.experience.add_exp(1);
                        self.spawn_prefabs
                            .get_mut(&enemy.typ)
                            .unwrap()
                            .killed_siblings += 1;
                    }
                    !enemy.is_dead
                });
                self.upgrade(lvl_ups);
            }
            Caster::Enemy { id } => todo!(),
        }
    }

    fn use_ultimate(&mut self) {
        if self.using_ultimate.is_some() {
            self.using_ultimate = None;
        } else if self.upgrade_menu.is_none() && self.player_ultimate.action.is_ready() {
            self.using_ultimate = Some(self.player.position);
            self.player_ultimate.action.set_on_cooldown();
        }
    }

    fn upgrade(&mut self, lvl_ups: usize) {
        if lvl_ups > 0 {
            let options = self
                .upgrades
                .iter()
                .filter_map(|(&typ, upgrade)| match upgrade {
                    Upgrade::Global { info, requirement } => {
                        if self.score >= *requirement && info.current < info.max {
                            Some((typ, vec![]))
                        } else {
                            None
                        }
                    }
                    Upgrade::Attack { info } => {
                        let options = info
                            .iter()
                            .enumerate()
                            .filter(|(_, info)| info.current < info.max)
                            .map(|(i, _)| i)
                            .collect::<Vec<_>>();
                        if options.is_empty() {
                            None
                        } else {
                            Some((typ, options))
                        }
                    }
                });
            let options = options.choose_multiple(&mut global_rng(), 3);
            self.upgrade_menu = Some(UpgradeMenu {
                lvl_ups_left: lvl_ups,
                options,
                choice: 0,
            });
        }
    }

    fn select_upgrade(&mut self) {
        if let Some(mut menu) = self.upgrade_menu.take() {
            if let Some((upgrade_type, attack_options)) = menu.options.get(menu.choice) {
                let attack_index = attack_options.choose(&mut global_rng());
                if let Some(upgrade) = self.upgrades.get_mut(upgrade_type) {
                    match upgrade_type {
                        UpgradeType::NewAttack => {
                            let attack_index = (0..self.potential_attacks.len())
                                .choose(&mut global_rng())
                                .unwrap();
                            let attack = self.potential_attacks.remove(attack_index);
                            self.player_attacks.push(attack);
                        }
                        UpgradeType::IncUltRadius => {
                            self.player_ultimate.radius += 1;
                        }
                        UpgradeType::ReduceUltCooldown => {
                            self.player_ultimate.action.cooldown -= 1;
                        }
                        UpgradeType::IncDeathTimer => {
                            // self.death_time += 2;
                        }
                        UpgradeType::ReduceAttackCooldown => {
                            self.player_attacks
                                .get_mut(*attack_index.unwrap())
                                .unwrap()
                                .action
                                .cooldown_multiplier *= 0.8;
                        }
                        UpgradeType::UpgradeAttack => {
                            self.player_attacks
                                .get_mut(*attack_index.unwrap())
                                .unwrap()
                                .upgrade();
                        }
                    }

                    match upgrade {
                        Upgrade::Global { info, .. } => {
                            info.current += 1;
                        }
                        Upgrade::Attack { info } => {
                            info.get_mut(*attack_index.unwrap()).unwrap().current += 1;
                        }
                    }

                    menu.lvl_ups_left -= 1;
                    if menu.lvl_ups_left > 0 {
                        self.upgrade_menu = Some(menu);
                    }
                }
            }
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
