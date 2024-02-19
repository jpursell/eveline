use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// The simple-signal crate is used to handle incoming signals.
use simple_signal::{self, Signal};

use rppal::gpio::Gpio;

// Gpio uses BCM pin numbering. BCM GPIO 23 is tied to physical pin 16.
const RIGHT_PINS: [u8; 4] = [2, 17, 3, 4];
const LEFT_PINS: [u8; 4] = [12, 21, 16, 20];

fn main() -> Result<(), Box<dyn Error>> {
    // Retrieve the GPIO pins and configure them as outputs.
    let mut right_pins = RIGHT_PINS
        .iter()
        .map(|&x| Gpio::new().unwrap().get(x).unwrap().into_output())
        .collect::<Vec<_>>();
    let mut left_pins = LEFT_PINS
        .iter()
        .map(|&x| Gpio::new().unwrap().get(x).unwrap().into_output())
        .collect::<Vec<_>>();
    right_pins.iter_mut().skip(1).for_each(|x| {
        x.set_low();
    });
    right_pins[0].set_high();
    left_pins.iter_mut().skip(1).for_each(|x| {
        x.set_low();
    });
    left_pins[0].set_high();

    let delay = {
        let rpm = 100;
        let steps = 100;
        let steps_per_minute = steps * rpm;
        let millis_per_minute = 60 * 1000;
        millis_per_minute / steps_per_minute
    };

    let running = Arc::new(AtomicBool::new(true));

    // When a SIGINT (Ctrl-C) or SIGTERM signal is caught, atomically set running to false.
    simple_signal::set_handler(&[Signal::Int, Signal::Term], {
        let running = running.clone();
        move |_| {
            running.store(false, Ordering::SeqCst);
        }
    });

    // Blink the LED until running is set to false.
    let mut current = 0;
    while running.load(Ordering::SeqCst) {
        let next = (current + 1) % 4;
        right_pins[next].set_high();
        left_pins[next].set_high();
        right_pins[current].set_low();
        left_pins[current].set_low();
        current = next;
        thread::sleep(Duration::from_millis(delay));
    }

    Ok(())

    // When the pin variable goes out of scope, the GPIO pin mode is automatically reset
    // to its original value, provided reset_on_drop is set to true (default).
}
