use geng::{Camera2d, Draw2d};

use super::*;

pub struct Renderer<'a, 'f> {
    geng: &'a Geng,
    camera: &'a Camera2d,
    framebuffer: &'a mut ugli::Framebuffer<'f>,
}

impl<'a, 'f> Renderer<'a, 'f> {
    pub fn new(
        geng: &'a Geng,
        camera: &'a Camera2d,
        framebuffer: &'a mut ugli::Framebuffer<'f>,
    ) -> Self {
        Self {
            geng,
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

    pub fn draw_grid(
        &mut self,
        grid_size: AABB<i32>,
        tile_size: Vec2<f32>,
        width: f32,
        color: Color<f32>,
    ) {
        let min = grid_size.bottom_left().map(|x| x as f32) * tile_size;
        let max = grid_size.top_right().map(|x| (x + 1) as f32) * tile_size;
        for [start, end] in (grid_size.x_min..=grid_size.x_max + 1)
            .map(|x| x as f32 * tile_size.x)
            .map(|x| [vec2(x, min.y), vec2(x, max.y)])
            .chain(
                (grid_size.x_min..=grid_size.x_max + 1)
                    .map(|y| y as f32 * tile_size.y)
                    .map(|y| [vec2(min.x, y), vec2(max.x, y)]),
            )
            .map(|points| points.map(|pos| pos - tile_size / 2.0))
        {
            draw_2d::Segment::new(Segment::new(start, end), width, color).draw_2d(
                self.geng,
                self.framebuffer,
                self.camera,
            );
        }
    }
}
