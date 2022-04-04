use super::*;
use model::*;

mod interpolation;

pub use interpolation::*;

impl GameState {
    pub fn tick(&mut self, player_move: Position) {
        if self.player.is_dead {
            return;
        }

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
            self.play_sound(self.assets.blip.play());
            return;
        }

        // Move player
        self.play_sound(self.assets.movement.play());
        self.freeze_move_timer = false;
        let old_pos = self.player.position;
        let pos = self.player.position + player_move;
        let (mut pos, jump) = wrap_pos(pos, self.arena_bounds);
        if let Some(origin) = self.using_ultimate {
            pos = clamp_wrapped_pos(
                pos,
                self.player_ultimate.boundary().translate(origin),
                self.arena_bounds,
            );
        }

        self.player.position = pos;
        if jump {
            let jump_dir = pos - old_pos;
            let jump_dir =
                vec2(jump_dir.x.signum(), jump_dir.y.signum()).map(|x| x as f32) * TILE_SIZE / 2.0;
            self.player
                .interpolation
                .queue(old_pos.map(|x| x as f32) - jump_dir);
            self.player
                .interpolation
                .queue_jump(pos.map(|x| x as f32) + jump_dir);
        }
        self.player.interpolation.queue(pos.map(|x| x as f32));

        if self.using_ultimate.is_some() {
            return;
        }

        self.move_time_left = self.move_time_limit;
        self.damages = vec![];

        // self.player_collide();

        // Move enemies
        for enemy in &mut self.enemies {
            let delta = self.player.position - enemy.position;
            enemy.position = clamp_pos(
                enemy.position + enemy.movement.move_towards(delta),
                self.arena_bounds,
            );
            enemy.interpolation.queue(enemy.position.map(|x| x as f32));
        }

        self.player_collide();

        // Player actions
        let mut attack_positions = Vec::new();
        let mut attacked = false;
        for attack in &mut self.player_attacks {
            if attack.action.update(1) {
                attacked = true;
                attack.action.set_on_cooldown();
                attack_positions.extend(attack.attack_positions(self.player.position));
            }
        }
        self.player_ultimate.action.update(1);
        self.attack_positions(Caster::Player, &attack_positions);
        if attacked {
            self.play_sound(self.assets.hit.play());
        }

        // Count siblings
        let mut siblings = HashMap::new();
        for (enemy_type, _) in &self.spawn_prefabs {
            siblings.insert(enemy_type.clone(), 0);
        }
        for enemy in &self.enemies {
            *siblings.get_mut(&enemy.typ).unwrap() += 1;
        }

        // Spawn new enemies
        self.enemies
            .extend(self.spawns.drain(..).map(|(_, enemy)| enemy));
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
                    interpolation: Interpolation::new(spawn_point.map(|x| x as f32)),
                    movement: prefab.movement.clone(),
                    is_dead: false,
                };
                self.spawns.push((spawn_point, enemy));
            }
        }
    }

    fn player_collide(&mut self) {
        if self
            .enemies
            .iter()
            .any(|enemy| enemy.position == self.player.position)
        {
            self.kill_player();
        }
    }

    pub fn play_sound(&mut self, mut sound: geng::SoundEffect) {
        sound.set_volume(self.volume);
    }

    pub fn kill_player(&mut self) {
        self.player.is_dead = true;
        self.play_sound(self.assets.death.play());
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

    pub fn use_ultimate(&mut self) {
        if self.player.is_dead {
            return;
        }

        if self.using_ultimate.is_some() {
            self.move_time_left = self.move_time_limit;
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
                .iter_mut()
                .filter_map(|(&typ, upgrade)| match upgrade {
                    Upgrade::Global { info } => {
                        let meet_requirement = typ
                            .requirement(info.current)
                            .check(self.score, attack_slots(*self.highscore));
                        if meet_requirement && info.current < info.max {
                            match typ {
                                UpgradeType::NewAttack => {
                                    let attack_index =
                                        (0..self.potential_attacks.len()).choose(&mut global_rng());
                                    attack_index.map(|i| (typ, Some(i)))
                                }
                                _ => Some((typ, None)),
                            }
                        } else {
                            None
                        }
                    }
                    Upgrade::Attack { info } => {
                        while info.len() < self.player_attacks.len() {
                            info.push(UpgradeInfo::new(info[0].max));
                        }

                        let attack_index = info
                            .iter()
                            .enumerate()
                            .filter(|(i, info)| {
                                info.current < info.max
                                    && match typ {
                                        UpgradeType::ReduceAttackCooldown => {
                                            self.player_attacks[*i].action.cooldown > 2
                                        }
                                        _ => true,
                                    }
                            })
                            .map(|(i, _)| i)
                            .choose(&mut global_rng());
                        attack_index.map(|i| ((typ, Some(i))))
                    }
                });
            let options = options.choose_multiple(&mut global_rng(), 3);
            self.upgrade_menu = Some(UpgradeMenu {
                lvl_ups_left: lvl_ups,
                options,
                choice: 0,
            });

            self.play_sound(self.assets.upgrade.play());
        }
    }

    pub fn select_upgrade(&mut self) {
        if let Some(mut menu) = self.upgrade_menu.take() {
            if let Some((upgrade_type, attack_index)) = menu.options.get(menu.choice) {
                if let Some(upgrade) = self.upgrades.get_mut(upgrade_type) {
                    match upgrade_type {
                        UpgradeType::NewAttack => {
                            let attack = self.potential_attacks.remove(attack_index.unwrap());
                            self.player_attacks.push(attack);
                        }
                        UpgradeType::IncUltRadius => {
                            self.player_ultimate.radius += 1;
                        }
                        UpgradeType::ReduceUltCooldown => {
                            self.player_ultimate.action.cooldown -= 1;
                        }
                        UpgradeType::IncDeathTimer => {
                            self.move_time_limit += 2.0;
                        }
                        UpgradeType::ReduceAttackCooldown => {
                            self.player_attacks
                                .get_mut(attack_index.unwrap())
                                .unwrap()
                                .action
                                .cooldown_multiplier *= 0.8;
                        }
                        UpgradeType::UpgradeAttack => {
                            self.player_attacks
                                .get_mut(attack_index.unwrap())
                                .unwrap()
                                .upgrade();
                        }
                    }

                    match upgrade {
                        Upgrade::Global { info, .. } => {
                            info.current += 1;
                        }
                        Upgrade::Attack { info } => {
                            info.get_mut(attack_index.unwrap()).unwrap().current += 1;
                        }
                    }

                    menu.lvl_ups_left -= 1;
                    self.upgrade(menu.lvl_ups_left);
                    self.play_sound(self.assets.select.play());
                }
            }
        }
    }
}

pub fn clamp_pos(pos: Position, aabb: AABB<Coord>) -> Position {
    vec2(
        pos.x.clamp(aabb.x_min, aabb.x_max),
        pos.y.clamp(aabb.y_min, aabb.y_max),
    )
}

pub fn wrap_pos(pos: Position, bounds: AABB<Coord>) -> (Position, bool) {
    let (x, jump_x) = wrap_coord(pos.x, vec2(bounds.x_min, bounds.x_max));
    let (y, jump_y) = wrap_coord(pos.y, vec2(bounds.y_min, bounds.y_max));
    (vec2(x, y), jump_x || jump_y)
}

pub fn wrap_coord(mut pos: Coord, bounds: Vec2<Coord>) -> (Coord, bool) {
    let width = bounds.y - bounds.x + 1;
    let mut jump = false;
    while pos < bounds.x {
        pos += width;
        jump = true;
    }
    while pos > bounds.y {
        pos -= width;
        jump = true;
    }
    (pos, jump)
}

pub fn grid_cell_aabb(cell_pos: Position, tile_size: Vec2<f32>) -> AABB<f32> {
    AABB::point(cell_pos.map(|x| x as f32) * tile_size).extend_symmetric(tile_size / 2.0)
}

pub fn clamp_wrapped_pos(pos: Position, aabb: AABB<Coord>, bounds: AABB<Coord>) -> Position {
    let x = clamp_wrapped_coord(
        pos.x,
        vec2(aabb.x_min, aabb.x_max),
        vec2(bounds.x_min, bounds.x_max),
    );
    let y = clamp_wrapped_coord(
        pos.y,
        vec2(aabb.y_min, aabb.y_max),
        vec2(bounds.y_min, bounds.y_max),
    );
    vec2(x, y)
}

fn clamp_wrapped_coord(pos: Coord, wrapping: Vec2<Coord>, bounds: Vec2<Coord>) -> Coord {
    let (pos, _) = wrap_coord(pos, bounds);
    let (min, wrap_min) = wrap_coord(wrapping.x, bounds);
    let (max, wrap_max) = wrap_coord(wrapping.y, bounds);
    if wrap_min ^ wrap_max {
        // Different sides
        let d_min = min - pos;
        let d_max = pos - max;
        if d_min > 0 && d_max > 0 {
            if d_min > d_max {
                max
            } else {
                min
            }
        } else {
            pos
        }
    } else {
        // Same side
        pos.clamp(min, max)
    }
}

pub fn attack_slots(score: Score) -> usize {
    SLOTS_REQUIREMENTS
        .iter()
        .enumerate()
        .find(|(_, &min)| min > score)
        .map(|(i, _)| i)
        .unwrap_or(SLOTS_REQUIREMENTS.len())
}
