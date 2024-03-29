#![allow(non_snake_case)]

use std::fs::OpenOptions;

use std::io::Write;
use tetrio_replay::reconstruct;
use ttrm::event::Event;
use ttrm::GameType;
use viewtris::action::Action;

fn reconstruct_from_events(
    events: &[Event],
    game_type: GameType,
    write_to: &str,
) -> Result<(), Vec<Action>> {
    let action_list = reconstruct(game_type, events).expect("Reconstruction step failed");

    std::fs::create_dir_all("test_out").expect("Could not create the test output directory");
    OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(write_to)
        .and_then(|mut out_file| {
            action_list
                .clone()
                .into_iter()
                .try_for_each(|action| writeln!(out_file, "{action:?}"))
        })
        .map_err(|_| action_list)
}

macro_rules! ttrm_test {
    ($name:ident) => {
        #[test]
        fn $name() {
            let ttr = serde_json::from_slice::<ttrm::Ttrm>(include_bytes!(concat!(
                "../../samples/",
                stringify!($name),
                ".ttrm"
            )))
            .expect("TTRM parsing is not working correctly, check tests in ttrm crate");

            for (i, data) in ttr.data.iter().enumerate() {
                for (j, replay) in data.replays.iter().enumerate() {
                    let write_to = format!(concat!("test_out/", stringify!($name), "_{}_{}.out"), i, j);

                    if let Err(action_list) = reconstruct_from_events(&replay.events, ttr.game_type, &write_to) {
                        println!(concat!(
                            "Test ",
                            stringify!($name),
                            " could not open the output file was writing, output going to stderr instead"
                        ));
                        eprintln!(concat!(stringify!($name), " actions_{}_{}: {:?}"), i, j, action_list);
                    }
                }
            }

        }
    };
}

ttrm_test!(HBSQabUhSS);
