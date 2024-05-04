mod controller;
mod draw;
mod gcode;
mod motor;
mod physical;
mod position;
mod predictor;
mod scurve;

use crate::controller::Controller;
use clap::Parser;
use log::info;
use simple_signal::{self, Signal};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    gcode_path: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Eveline start");

    let args = Args::parse();

    let mut controller = Controller::new(args.gcode_path);

    let running = Arc::new(AtomicBool::new(true));

    // When a SIGINT (Ctrl-C) or SIGTERM signal is caught, atomically set running to false.
    simple_signal::set_handler(&[Signal::Int, Signal::Term], {
        let running = running.clone();
        move |_| {
            running.store(false, Ordering::SeqCst);
        }
    });

    // Operate until running is set to false.
    while running.load(Ordering::SeqCst) {
        controller.update();
    }
    info!("Eveline done");
    Ok(())
}
