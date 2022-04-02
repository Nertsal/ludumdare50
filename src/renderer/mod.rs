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
}
