use geng::prelude::*;

pub struct State {
    geng: Geng,
}

impl State {
    pub fn new(geng: &Geng) -> Self {
        Self { geng: geng.clone() }
    }
}

impl geng::State for State {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Color::BLACK), None);
    }
}

fn main() {
    logger::init().unwrap();
    geng::setup_panic_handler();

    let geng = Geng::new("Delay the inevitable");
    let state = State::new(&geng);

    geng::run(&geng, state);
}
