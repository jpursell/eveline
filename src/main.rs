mod controller;
mod motor;
mod physical;
mod position;
mod predictor;
mod scurve;

use crate::controller::Controller;
use simple_signal::{self, Signal};
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Eveline start");
    let mut controller = Controller::new();

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
    println!("Eveline done");
    Ok(())
}
