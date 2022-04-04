use geng::prelude::*;

mod logic;
mod model;
mod renderer;

#[derive(geng::Assets)]
pub struct Assets {
    pub lock: ugli::Texture,
    pub exclamation: ugli::Texture,
    pub hit: geng::Sound,
    pub death: geng::Sound,
    pub movement: geng::Sound,
    pub blip: geng::Sound,
    pub select: geng::Sound,
    pub upgrade: geng::Sound,
    // pub music: geng::Sound,
}

fn main() {
    logger::init().unwrap();
    geng::setup_panic_handler();

    let geng = Geng::new("Delay the inevitable");
    let assets = <Assets as geng::LoadAsset>::load(&geng, &static_path());

    geng::run(
        &geng,
        geng::LoadingScreen::new(&geng, geng::EmptyLoadingScreen, assets, {
            let geng = geng.clone();
            move |assets| {
                let mut assets = assets.unwrap();
                assets.lock.set_filter(ugli::Filter::Nearest);
                assets.exclamation.set_filter(ugli::Filter::Nearest);
                // assets.music.looped = true;
                model::GameState::new(&geng, &Rc::new(assets))
            }
        }),
    );
}
