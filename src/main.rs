use geng::prelude::*;

mod game_state;

#[derive(geng::Assets)]
pub struct Assets {}

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
                let assets = assets.unwrap();
                game_state::GameState::new(&geng, &Rc::new(assets))
            }
        }),
    );
}
