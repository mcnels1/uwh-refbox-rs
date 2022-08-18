use std::{str::FromStr, sync::mpsc::channel};
//use uwh_common::game_snapshot::GamePeriod;
use network::State;
use std::net::IpAddr;

use macroquad::prelude::*;
use uwh_common::game_snapshot::GamePeriod;
mod load_images;
mod network;
mod pages;
mod pages_alpha;
mod pages_color;

const APP_CONFIG_NAME: &str = "uwh-overlay-drawing";

fn window_conf() -> Conf {
    Conf {
        window_title: "UWH Overlay".to_owned(),
        window_width: 1920,
        window_height: 1080,
        window_resizable: false,
        ..Default::default()
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    refbox_ip: IpAddr,
    refbox_port: u64,
    uwhscores_url: String,
}

impl AppConfig {
    /// Get the app config's refbox ip.
    pub fn refbox_ip(&self) -> IpAddr {
        self.refbox_ip
    }

    /// Get the app config's refbox port.
    pub fn refbox_port(&self) -> u64 {
        self.refbox_port
    }

    /// Get a reference to the app config's uwhscores url.
    pub fn uwhscores_url(&self) -> &str {
        self.uwhscores_url.as_ref()
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            refbox_ip: IpAddr::from_str("127.0.0.1").unwrap(),
            refbox_port: 8000,
            uwhscores_url: String::from("uwhscores.com"),
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let (tx, rx) = channel::<State>();

    let config: AppConfig = match confy::load(APP_CONFIG_NAME, None) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read config file, overwriting with default. Error: {e}");
            let config = AppConfig::default();
            confy::store(APP_CONFIG_NAME, None, &config).unwrap();
            config
        }
    };

    std::thread::spawn(|| {
        network::networking_thread(tx, config)
            .expect("Networking error. Does the supplied URL exist and is it live?")
    });

    let args: Vec<String> = std::env::args().collect();
    assert!(
        args.len() == 2,
        "Got {} args instead of one. Pass one argument, --color or --alpha to get the color or alpha feed respectively",
        args.len() - 1
    );

    let (textures, is_alpha_mode) = if args[1] == *"--color" {
        (load_images::Textures::init_color(), false)
    } else if args[1] == *"--alpha" {
        (load_images::Textures::init_alpha(), true)
    } else {
        panic!("Expected --color or --alpha arg!")
    };

    let mut game_state: Option<State> = None;

    // Should the goal graphic be displayed?
    // let mut show_goal = false;
    //keeps track of old whit and black scores in order to detect a change and show the goal graphic
    // let (mut b_score, mut w_score) = (0, 0);
    let mut renderer = pages::PageRenderer {
        animation_counter: 0f32,
        textures,
        is_alpha_mode,
    };

    loop {
        clear_background(BLACK);
        if let Ok(state) = rx.try_recv() {
            game_state = Some(state);
        }
        // if show_goal {
        //     if !is_alpha_mode {
        //         pages_color::show_goal_graphic(
        //             &textures,
        //             &mut secondary_animation_counter,
        //             &mut show_goal,
        //         );
        //     } else {
        //         pages_alpha::show_goal_graphic(
        //             &textures,
        //             &mut secondary_animation_counter,
        //             &mut show_goal,
        //         );
        //     }
        // }

        if let Some(state) = &game_state {
            // if state.snapshot.b_score != b_score || state.snapshot.w_score != w_score {
            //     w_score = state.snapshot.w_score;
            //     b_score = state.snapshot.b_score;
            //     show_goal = true;
            // }
            match state.snapshot.current_period {
                GamePeriod::BetweenGames => match state.snapshot.secs_in_period {
                    151..=u16::MAX => {
                        // If an old game just finished, display its scores for a minute
                        if state.snapshot.is_old_game && state.snapshot.secs_in_period > 2800 {
                            renderer.final_scores(state);
                        } else {
                            renderer.next_game(state);
                        }
                    }
                    30..=150 => {
                        renderer.roster(state);
                    }
                    _ => {
                        renderer.pre_game_display(state);
                    }
                },
                GamePeriod::FirstHalf | GamePeriod::SecondHalf | GamePeriod::HalfTime => {
                    renderer.in_game_display(state);
                }
                GamePeriod::OvertimeFirstHalf
                | GamePeriod::OvertimeHalfTime
                | GamePeriod::OvertimeSecondHalf
                | GamePeriod::PreOvertime => {
                    renderer.overtime_display();
                }
                _ => {}
            }
        }
        next_frame().await;
    }
}
