use super::*;

impl GameState {
    pub fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        let framebuffer_size = framebuffer.size().map(|x| x as f32);

        // Game camera
        let mut renderer = Renderer::new(&self.geng, &self.assets, &self.camera, framebuffer);

        // Grid
        renderer.draw_grid(
            self.arena_bounds,
            TILE_SIZE,
            -TILE_SIZE / 2.0,
            GRID_WIDTH,
            GRID_COLOR,
        );

        // Wrap indicator
        if self.player.position.x == self.arena_bounds.x_min
            || self.player.position.x == self.arena_bounds.x_max
        {
            let left_pos = vec2(self.arena_bounds.x_min, self.player.position.y).map(|x| x as f32);
            let right_pos =
                vec2(self.arena_bounds.x_max + 1, self.player.position.y).map(|x| x as f32);
            for pos in [left_pos, right_pos]
                .into_iter()
                .map(|x| x - TILE_SIZE / 2.0)
            {
                renderer.draw_aabb(
                    AABB::point(pos)
                        .extend_symmetric(vec2(GRID_WIDTH / 2.0, 0.0))
                        .extend_up(TILE_SIZE.y),
                    WRAP_COLOR,
                );
            }
        }
        if self.player.position.y == self.arena_bounds.y_min
            || self.player.position.y == self.arena_bounds.y_max
        {
            let bottom_pos =
                vec2(self.player.position.x, self.arena_bounds.y_max + 1).map(|x| x as f32);
            let top_pos = vec2(self.player.position.x, self.arena_bounds.y_min).map(|x| x as f32);
            for pos in [bottom_pos, top_pos]
                .into_iter()
                .map(|x| x - TILE_SIZE / 2.0)
            {
                renderer.draw_aabb(
                    AABB::point(pos)
                        .extend_symmetric(vec2(0.0, GRID_WIDTH / 2.0))
                        .extend_right(TILE_SIZE.y),
                    WRAP_COLOR,
                );
            }
        }

        // Spawns
        for (spawn_pos, _) in &self.spawns {
            let aabb = logic::grid_cell_aabb(*spawn_pos, TILE_SIZE);
            let aabb = AABB::point(aabb.center()).extend_symmetric(WARNING_SIZE / 2.0);
            renderer.draw_texture(&self.assets.exclamation, aabb);
        }

        // Enemies
        for enemy in &self.enemies {
            renderer.draw_circle(
                enemy.interpolation.current() * TILE_SIZE,
                UNIT_RADIUS,
                enemy.color,
            );
        }

        // Ultimate
        if let Some(origin) = self.using_ultimate {
            for pos in self
                .player_ultimate
                .deltas()
                .map(|pos| logic::wrap_pos(pos + origin, self.arena_bounds).0)
                .map(|pos| logic::grid_cell_aabb(pos, TILE_SIZE).center())
            {
                renderer.draw_circle(pos, 0.1, Color::MAGENTA);
            }
        }

        // Player
        let mut color = self.player.color;
        color.a = if self.using_ultimate.is_some() {
            PLAYER_ULTIMATE_ALPHA
        } else {
            1.0
        };
        renderer.draw_circle(
            self.player.interpolation.current() * TILE_SIZE,
            UNIT_RADIUS,
            color,
        );

        // Damage
        for &pos in &self.damages {
            let aabb = logic::grid_cell_aabb(pos, TILE_SIZE).extend_uniform(-DAMAGE_EXTRA_SPACE);
            renderer.draw_cross(aabb, DAMAGE_WIDTH, DAMAGE_COLOR);
        }

        // UI camera
        let framebuffer_size = vec2(
            self.ui_camera.fov / framebuffer_size.y * framebuffer_size.x,
            self.ui_camera.fov,
        );
        self.ui_camera.center = framebuffer_size / 2.0;
        let mut renderer = Renderer::new(&self.geng, &self.assets, &self.ui_camera, framebuffer);

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
            logic::attack_slots(*self.highscore),
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
            &format!("Points: {}", self.score),
            vec2(10.0, framebuffer_size.y - 10.0),
            vec2(0.0, 1.0),
            20.0,
            Color::GRAY,
        );
        renderer.draw_text(
            &format!("High Score: {}", *self.highscore),
            vec2(10.0, framebuffer_size.y - 100.0),
            vec2(0.0, 1.0),
            15.0,
            Color::GRAY,
        );

        // Move time
        renderer.draw_text(
            &format!("Time left: {:.1}", self.move_time_left),
            vec2(framebuffer_size.x / 2.0, framebuffer_size.y - 10.0),
            vec2(0.5, 1.0),
            20.0,
            Color::GRAY,
        );
        let time_aabb = AABB::point(vec2(framebuffer_size.x / 2.0, framebuffer_size.y - 100.0))
            .extend_symmetric(TIME_BAR_SIZE / 2.0);
        renderer.draw_aabb(time_aabb, TIME_BAR_BACKGROUND_COLOR);
        let time_ratio = self.move_time_left / self.move_time_limit;
        let time_bar = time_aabb.extend_symmetric(vec2(0.0, -TIME_BAR_INNER_SPACE));
        let time_bar = time_bar.extend_right((time_ratio - 1.0) * time_bar.width());
        renderer.draw_aabb(time_bar, TIME_BAR_COLOR);
        renderer.draw_aabb_frame(time_aabb, TIME_BORDER_WIDTH, TIME_BORDER_COLOR);

        // Experience
        let exp_aabb = AABB::point(vec2(EXPERIENCE_BAR_SIZE.x * 2.0, framebuffer_size.y / 2.0))
            .extend_symmetric(EXPERIENCE_BAR_SIZE / 2.0);
        renderer.draw_aabb(exp_aabb, EXPERIENCE_BAR_BACKGROUND_COLOR);
        let exp_ratio = self.experience.exp as f32 / self.experience.exp_to_next_lvl as f32;
        let exp_bar = exp_aabb.extend_symmetric(vec2(-EXPERIENCE_BAR_INNER_SPACE, 0.0));
        let exp_bar = exp_bar.extend_up((exp_ratio - 1.0) * exp_bar.height());
        renderer.draw_aabb(exp_bar, EXPERIENCE_BAR_COLOR);
        renderer.draw_aabb_frame(exp_aabb, EXPERIENCE_BORDER_WIDTH, EXPERIENCE_BORDER_COLOR);
        let level_aabb = AABB::point(vec2(exp_aabb.center().x, exp_aabb.y_min))
            .extend_uniform(EXPERIENCE_BAR_SIZE.x);
        renderer.draw_level(self.experience.level, level_aabb);
        let level_aabb = level_aabb.translate(vec2(0.0, exp_aabb.height()));
        renderer.draw_level(self.experience.level + 1, level_aabb);

        // Upgrade menu
        if let Some(upgrade_menu) = &self.upgrade_menu {
            let upgrades_width = (UPGRADE_SIZE.x + UPGRADE_EXTRA_SPACE)
                * upgrade_menu.options.len() as f32
                - UPGRADE_EXTRA_SPACE;
            renderer.draw_aabb(
                AABB::point(framebuffer_size / 2.0).extend_symmetric(
                    vec2(
                        upgrades_width + UPGRADE_EXTRA_SPACE,
                        UPGRADE_SIZE.y + UPGRADE_EXTRA_SPACE,
                    ) / 2.0,
                ),
                UPGRADE_BACKGROUND_COLOR,
            );
            let upgrade_aabb = AABB::ZERO.extend_symmetric(UPGRADE_SIZE / 2.0);
            let left_pos =
                framebuffer_size / 2.0 - vec2((upgrades_width - UPGRADE_SIZE.x) / 2.0, 0.0);
            for (i, (upgrade, attack_index)) in upgrade_menu.options.iter().enumerate() {
                let attack_index = attack_index.map(|i| i + 1);
                let aabb = upgrade_aabb.translate(
                    left_pos + i as f32 * vec2(UPGRADE_SIZE.x + UPGRADE_EXTRA_SPACE, 0.0),
                );
                renderer.draw_aabb_frame(aabb, UPGRADE_FRAME_WIDTH, UPGRADE_FRAME_COLOR);
                if i == upgrade_menu.choice {
                    renderer.draw_aabb(
                        aabb.extend_uniform(-UPGRADE_FRAME_WIDTH / 2.0),
                        UPGRADE_SELECTED_COLOR,
                    );
                }
                let aabb = aabb.extend_uniform(-UPGRADE_SIZE.x * 0.1);
                let text = match upgrade {
                    UpgradeType::NewAttack => {
                        let text_aabb = aabb.extend_down(-aabb.height() / 2.0);
                        renderer.draw_text_fit(
                            "New",
                            text_aabb.extend_down(-text_aabb.height() / 2.0),
                            UPGRADE_TEXT_COLOR,
                        );
                        let new_attack = &self.potential_attacks[attack_index.unwrap()];
                        renderer.draw_text_fit(
                            &format!("Cooldown {}", new_attack.action.cooldown),
                            text_aabb.extend_up(-text_aabb.height() / 2.0),
                            UPGRADE_TEXT_COLOR,
                        );
                        renderer.draw_attack(new_attack, aabb.extend_up(-aabb.height() / 2.0));
                        format!("")
                    }
                    UpgradeType::IncUltRadius => format!("+1 TP Radius"),
                    UpgradeType::ReduceUltCooldown => format!("-1 TP Cooldown"),
                    UpgradeType::IncDeathTimer => format!("+2 sec Life"),
                    UpgradeType::ReduceAttackCooldown => {
                        format!("-20% Cooldown {}", attack_index.unwrap())
                    }
                    UpgradeType::UpgradeAttack => format!("Upgrade {}", attack_index.unwrap()),
                };
                renderer.draw_text_fit(&text, aabb, UPGRADE_TEXT_COLOR);
            }
        }

        // Fade
        let mut color = FADE_COLOR;
        color.a = self.fade.current;
        renderer.draw_aabb(AABB::ZERO.extend_positive(framebuffer_size), color);
    }
}
