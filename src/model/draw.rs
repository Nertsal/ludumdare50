use super::*;

impl GameState {
    pub fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        let framebuffer_size = framebuffer.size().map(|x| x as f32);
        let mut renderer = Renderer::new(&self.geng, &self.assets, &self.camera, framebuffer);

        // Enemies
        for enemy in &self.enemies {
            renderer.draw_circle(
                enemy.interpolation.current() * TILE_SIZE,
                UNIT_RADIUS,
                enemy.color,
            );
        }

        // Player
        renderer.draw_circle(
            self.player.interpolation.current() * TILE_SIZE,
            UNIT_RADIUS,
            self.player.color,
        );

        // Damage
        for &pos in &self.damages {
            let aabb = logic::grid_cell_aabb(pos, TILE_SIZE).extend_uniform(-DAMAGE_EXTRA_SPACE);
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
                .map(|pos| logic::grid_cell_aabb(pos, TILE_SIZE).center())
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
            for (i, (upgrade, _)) in upgrade_menu.options.iter().enumerate() {
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
                let text = match upgrade {
                    UpgradeType::NewAttack => "NEW",
                    UpgradeType::IncUltRadius => "+1 TP RADIUS",
                    UpgradeType::ReduceUltCooldown => "-1 TP CD",
                    UpgradeType::IncDeathTimer => "+2 SEC LIFE",
                    UpgradeType::ReduceAttackCooldown => "-2 ATTACK CD",
                    UpgradeType::UpgradeAttack => "UPGRADE",
                };
                renderer.draw_text_fit(
                    text,
                    aabb.extend_uniform(-UPGRADE_SIZE.x * 0.1),
                    UPGRADE_TEXT_COLOR,
                );
            }
        }
    }
}
