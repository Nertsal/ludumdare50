use super::*;

impl GameState {
    pub fn reset(&mut self) {
        let state = Self::new(&self.geng, &self.assets);
        *self = state;
    }

    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
            arena_bounds: AABB::from_corners(vec2(-4, -4), vec2(5, 5)),
            highscore: AutoSave::load(static_path().join("highscore.json").to_str().unwrap()),
            score: 0,
            move_time_limit: 6.0,
            move_time_left: 6.0,
            experience: Experience::new(),
            using_ultimate: None,
            upgrade_menu: None,
            freeze_move_timer: true,
            fade: Fade {
                min: 0.0,
                max: 1.0,
                current: 1.0,
                speed: -1.0 / FADE_TIME,
            },
            camera: Camera2d {
                center: Vec2::ZERO,
                rotation: 0.0,
                fov: 15.0,
            },
            player: Player {
                color: PLAYER_COLOR,
                position: Vec2::ZERO,
                interpolation: Interpolation::new(Vec2::ZERO),
                is_dead: false,
            },
            enemies: vec![],
            damages: vec![],
            player_attacks: initial_attacks().collect(),
            potential_attacks: potential_attacks().collect(),
            player_ultimate: Teleport::new(5, 1),
            upgrades: [
                (
                    UpgradeType::ReduceAttackCooldown,
                    Upgrade::Attack {
                        info: vec![UpgradeInfo::new(3)],
                    },
                ),
                (
                    UpgradeType::UpgradeAttack,
                    Upgrade::Attack {
                        info: vec![UpgradeInfo::new(2)],
                    },
                ),
                (
                    UpgradeType::NewAttack,
                    Upgrade::Global {
                        info: UpgradeInfo::new(3),
                        requirement: 0,
                    },
                ),
                (
                    UpgradeType::IncUltRadius,
                    Upgrade::Global {
                        info: UpgradeInfo::new(2),
                        requirement: 30,
                    },
                ),
                (
                    UpgradeType::ReduceUltCooldown,
                    Upgrade::Global {
                        info: UpgradeInfo::new(2),
                        requirement: 100,
                    },
                ),
                (
                    UpgradeType::IncDeathTimer,
                    Upgrade::Global {
                        info: UpgradeInfo::new(2),
                        requirement: 0,
                    },
                ),
            ]
            .into_iter()
            .collect(),
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
}

fn rotate_randomly(attacks: impl IntoIterator<Item = Attack>) -> impl Iterator<Item = Attack> {
    attacks.into_iter().map(|mut attack| {
        let rotation = global_rng().gen_range(0..=3);
        for _ in 0..rotation {
            attack.rotate_left()
        }
        attack
    })
}

fn initial_attacks() -> impl Iterator<Item = Attack> {
    rotate_randomly([Attack::new(
        2,
        [vec2(1, 0)],
        Some(Attack::new(
            2,
            [vec2(1, 0), vec2(2, 0)],
            Some(Attack::new(2, [vec2(1, 0), vec2(2, 0), vec2(3, 0)], None)),
        )),
    )])
}

fn potential_attacks() -> impl Iterator<Item = Attack> {
    rotate_randomly([
        Attack::new(
            2,
            [vec2(1, 0), vec2(2, 1)],
            Some(Attack::new(
                2,
                [vec2(1, 0), vec2(2, 1), vec2(2, -1)],
                Some(Attack::new(
                    2,
                    [vec2(1, 0), vec2(2, 1), vec2(2, -1), vec2(2, 0)],
                    None,
                )),
            )),
        ),
        Attack::new(
            2,
            [vec2(1, 0), vec2(2, 0), vec2(1, 1)],
            Some(Attack::new(
                2,
                [vec2(1, 0), vec2(2, 0), vec2(1, 1), vec2(1, -1)],
                Some(Attack::new(
                    2,
                    [vec2(1, 0), vec2(2, 0), vec2(1, 1), vec2(1, -1), vec2(3, 1)],
                    None,
                )),
            )),
        ),
        Attack::new(
            2,
            [vec2(1, 0), vec2(2, 0), vec2(3, 0), vec2(3, 1)],
            Some(Attack::new(
                2,
                [vec2(1, 0), vec2(2, 0), vec2(3, 0), vec2(3, 1), vec2(3, -1)],
                Some(Attack::new(
                    2,
                    [
                        vec2(1, 0),
                        vec2(2, 0),
                        vec2(3, 0),
                        vec2(3, 1),
                        vec2(3, -1),
                        vec2(4, 1),
                        vec2(4, -1),
                    ],
                    None,
                )),
            )),
        ),
        Attack::new(
            2,
            [vec2(1, 0), vec2(2, 1), vec2(2, 0), vec2(2, -1)],
            Some(Attack::new(
                2,
                [
                    vec2(1, 0),
                    vec2(2, 1),
                    vec2(2, 0),
                    vec2(2, -1),
                    vec2(3, 1),
                    vec2(3, -1),
                ],
                Some(Attack::new(
                    2,
                    [
                        vec2(1, 0),
                        vec2(2, 1),
                        vec2(2, 0),
                        vec2(2, -1),
                        vec2(3, 1),
                        vec2(3, -1),
                        vec2(4, 0),
                    ],
                    None,
                )),
            )),
        ),
        Attack::new(
            2,
            [vec2(1, 1), vec2(1, -1), vec2(2, 0), vec2(3, 0)],
            Some(Attack::new(
                2,
                [
                    vec2(1, 1),
                    vec2(1, -1),
                    vec2(2, 0),
                    vec2(3, 0),
                    vec2(4, 1),
                    vec2(4, -1),
                ],
                Some(Attack::new(
                    2,
                    [
                        vec2(1, 1),
                        vec2(1, -1),
                        vec2(2, 0),
                        vec2(3, 0),
                        vec2(4, 1),
                        vec2(4, -1),
                        vec2(4, 0),
                        vec2(5, 0),
                    ],
                    None,
                )),
            )),
        ),
    ])
}
