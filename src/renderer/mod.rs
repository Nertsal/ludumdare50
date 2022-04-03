use geng::Draw2d;

use crate::model::{Attack, Teleport};

use super::*;

pub struct Renderer<'a, 'f, C: geng::AbstractCamera2d> {
    geng: &'a Geng,
    assets: &'a Rc<Assets>,
    camera: &'a C,
    framebuffer: &'a mut ugli::Framebuffer<'f>,
}

impl<'a, 'f, C: geng::AbstractCamera2d> Renderer<'a, 'f, C> {
    pub fn new(
        geng: &'a Geng,
        assets: &'a Rc<Assets>,
        camera: &'a C,
        framebuffer: &'a mut ugli::Framebuffer<'f>,
    ) -> Self {
        Self {
            geng,
            assets,
            camera,
            framebuffer,
        }
    }

    pub fn draw_circle(&mut self, center: Vec2<f32>, radius: f32, color: Color<f32>) {
        draw_2d::Ellipse::circle(center, radius, color).draw_2d(
            self.geng,
            self.framebuffer,
            self.camera,
        );
    }

    pub fn draw_circle_with_cut(
        &mut self,
        center: Vec2<f32>,
        inner_radius: f32,
        radius: f32,
        color: Color<f32>,
    ) {
        draw_2d::Ellipse::circle_with_cut(center, inner_radius, radius, color).draw_2d(
            self.geng,
            self.framebuffer,
            self.camera,
        );
    }

    pub fn draw_aabb(&mut self, aabb: AABB<f32>, width: f32, color: Color<f32>) {
        let corners = aabb.corners();
        draw_2d::Chain::new(
            Chain::new(
                corners
                    .into_iter()
                    .chain(std::iter::once(corners[0]))
                    .collect(),
            ),
            width,
            color,
            0,
        )
        .draw_2d(self.geng, self.framebuffer, self.camera);
    }

    pub fn draw_grid(
        &mut self,
        grid_size: AABB<i32>,
        tile_size: Vec2<f32>,
        offset: Vec2<f32>,
        width: f32,
        color: Color<f32>,
    ) {
        let min = grid_size.bottom_left().map(|x| x as f32) * tile_size;
        let max = grid_size.top_right().map(|x| (x + 1) as f32) * tile_size;
        for [start, end] in (grid_size.x_min..=grid_size.x_max + 1)
            .map(|x| x as f32 * tile_size.x)
            .map(|x| [vec2(x, min.y), vec2(x, max.y)])
            .chain(
                (grid_size.y_min..=grid_size.y_max + 1)
                    .map(|y| y as f32 * tile_size.y)
                    .map(|y| [vec2(min.x, y), vec2(max.x, y)]),
            )
            .map(|points| points.map(|pos| pos + offset))
        {
            draw_2d::Segment::new(Segment::new(start, end), width, color).draw_2d(
                self.geng,
                self.framebuffer,
                self.camera,
            );
        }
    }

    pub fn draw_attacks(
        &mut self,
        actions: &[Attack],
        action_limit: usize,
        bounds: AABB<f32>,
        border_width: f32,
        border_color: Color<f32>,
    ) {
        if action_limit == 0 {
            return;
        }

        let top_right = bounds.top_right();
        let bottom_left = bounds.bottom_left();
        let single_height = bounds.height() / action_limit as f32;
        let single_aabb =
            AABB::from_corners(vec2(bottom_left.x, top_right.y - single_height), top_right);

        self.draw_grid(
            AABB::from_corners(vec2(0, 0), vec2(0, action_limit as i32 - 1)),
            single_aabb.size(),
            bottom_left,
            border_width,
            border_color,
        );

        for (index, attack) in actions.iter().enumerate().take(action_limit) {
            let aabb = single_aabb.translate(vec2(0.0, -single_height * index as f32));
            let aabb = aabb.extend_uniform(-aabb.width() * 0.1);
            self.draw_attack(attack, aabb);
        }
    }

    pub fn draw_attack(&mut self, attack: &Attack, aabb: AABB<f32>) {
        let boundary = AABB::points_bounding_box(
            attack
                .attack_positions(Vec2::ZERO)
                .chain(std::iter::once(Vec2::ZERO)),
        );
        let (scale, offset) = scale_align_aabb(boundary.map(|x| x as f32), aabb);
        let aabb = aabb.translate(offset);

        let tile_size = vec2(scale, scale);
        self.draw_grid(
            boundary,
            tile_size,
            aabb.center() - tile_size / 2.0,
            2.5,
            Color::GRAY,
        );

        self.draw_circle_with_cut(
            aabb.center(),
            scale / 2.0 * 0.7,
            scale / 2.0 * 0.8,
            model::PLAYER_COLOR,
        );
        for pos in attack.attack_positions(Vec2::ZERO) {
            let aabb = model::grid_cell_aabb(pos, tile_size)
                .translate(aabb.center())
                .extend_uniform(-scale * 0.2);
            self.draw_cross(aabb, scale * 0.1, Color::RED)
        }
    }

    pub fn draw_ultimate(
        &mut self,
        ultimate: &Teleport,
        aabb: AABB<f32>,
        border_width: f32,
        border_color: Color<f32>,
        font_size: f32,
    ) {
        let aabb = aabb.extend_up(-font_size);
        self.draw_text(
            "ULTIMATE",
            vec2(aabb.center().x, aabb.top_left().y),
            vec2(0.5, 1.0),
            font_size,
            Color::MAGENTA,
        );
        let aabb = aabb.extend_up(-2.5 * font_size);

        let boundary = AABB::ZERO.extend_uniform(ultimate.radius);
        let (scale, offset) = scale_align_aabb(boundary.map(|x| x as f32), aabb);
        let aabb = aabb.translate(offset);

        let tile_size = vec2(scale, scale);
        self.draw_grid(
            boundary,
            tile_size,
            aabb.center() - tile_size / 2.0,
            border_width,
            border_color,
        );

        self.draw_circle_with_cut(
            aabb.center(),
            scale / 2.0 * 0.7,
            scale / 2.0 * 0.8,
            model::PLAYER_COLOR,
        );
        for pos in (boundary.x_min..=boundary.x_max)
            .flat_map(|x| (boundary.y_min..=boundary.y_max).map(move |y| vec2(x, y)))
            .filter(|pos| {
                *pos != Vec2::ZERO
                    && pos.x.abs() <= ultimate.radius
                    && pos.y.abs() <= ultimate.radius
            })
            .map(|pos| model::grid_cell_aabb(pos, tile_size).center())
        {
            self.draw_circle(pos + aabb.center(), scale * 0.1, Color::MAGENTA);
        }
    }

    pub fn draw_cross(&mut self, aabb: AABB<f32>, width: f32, color: Color<f32>) {
        draw_2d::Segment::new(
            Segment::new(aabb.bottom_left(), aabb.top_right()),
            width,
            color,
        )
        .draw_2d(self.geng, self.framebuffer, self.camera);
        draw_2d::Segment::new(
            Segment::new(aabb.top_left(), aabb.bottom_right()),
            width,
            color,
        )
        .draw_2d(self.geng, self.framebuffer, self.camera);
    }

    pub fn draw_text(
        &mut self,
        text: &str,
        pos: Vec2<f32>,
        alignment: Vec2<f32>,
        font_size: f32,
        color: Color<f32>,
    ) {
        draw_2d::Text::unit(self.geng.default_font().clone(), text, color)
            .scale_uniform(font_size)
            .align_bounding_box(alignment)
            .translate(pos)
            .draw_2d(self.geng, self.framebuffer, self.camera);
    }
}

fn scale_align_aabb(aabb: AABB<f32>, target: AABB<f32>) -> (f32, Vec2<f32>) {
    let scale = target.size() / aabb.size().map(|x| x as f32 + 1.0);
    let scale = scale.x.min(scale.y);
    let offset = -aabb.center() * scale;
    (scale, offset)
}
