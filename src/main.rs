use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

// The simple-signal crate is used to handle incoming signals.
use simple_signal::{self, Signal};

use rppal::gpio::{Gpio, OutputPin};

// Gpio uses BCM pin numbering. BCM GPIO 23 is tied to physical pin 16.
const RIGHT_PINS: [u8; 4] = [4, 22, 17, 27];
const LEFT_PINS: [u8; 4] = [12, 21, 16, 20];

struct Motor {
    pins: [OutputPin; 4],
    current: usize,
    position: i32,
    side: Side,
}

enum Side {
    Left,
    Right,
}
impl Motor {
    fn new(side: Side) -> Motor {
        // init output pins
        let pin_nums = match side {
            Side::Left => LEFT_PINS,
            Side::Right => RIGHT_PINS,
        };
        let gpio = Gpio::new().unwrap();
        let make_output = |n| gpio.get(n).unwrap().into_output();
        let mut pins = [
            make_output(pin_nums[0]),
            make_output(pin_nums[1]),
            make_output(pin_nums[2]),
            make_output(pin_nums[3]),
        ];
        // set initial pin value
        let current = 0;
        pins[current].set_high();
        pins.iter_mut().skip(1).for_each(|pin| pin.set_low());
        Motor {
            pins,
            current,
            position: 0,
            side,
        }
    }
    fn step_down(&mut self) {
        match self.side {
            Side::Left => self.step_clock_wise(),
            Side::Right => self.step_counter_clock_wise(),
        };
        self.position -= 1;
    }
    fn step_up(&mut self) {
        match self.side {
            Side::Right => self.step_clock_wise(),
            Side::Left => self.step_counter_clock_wise(),
        };
        self.position += 1;
    }
    fn update_pins(&mut self) {
        // current in [00,01,02,03,04,05,06,07,08,09,10,11,12,13,14,15]
        // pin 0      [
        // pin 1      [0,1,1,1,0,0,0,0]
        // pin 2      [0,0,0,0,0,0,0,0]
        // pin 3      [0,0,0,0,0,0,0,0]
        // c/4        [ 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3]
        // c%2        [0,1,0,1,0,1,0,1]
        // (c+1)%4/2  [0,0,0,0,0,0,0,0]
        let step_division = 2;
        let main_pin = self.current / step_division;
        let secondary_pin = (self.current + 1) % 4 / 2;
        match self.current {
            0 => {
                self.pins[0].set_high();
                self.pins[1].set_low();
                self.pins[2].set_low();
                self.pins[3].set_low();
            }
            1 => {
                self.pins[0].set_high();
                self.pins[1].set_high();
                self.pins[2].set_low();
                self.pins[3].set_low();
            }
            2 => {
                self.pins[0].set_low();
                self.pins[1].set_high();
                self.pins[2].set_low();
                self.pins[3].set_low();
            }
            3 => {
                self.pins[0].set_low();
                self.pins[1].set_high();
                self.pins[2].set_high();
                self.pins[3].set_low();
            }
            4 => {
                self.pins[0].set_low();
                self.pins[1].set_low();
                self.pins[2].set_high();
                self.pins[3].set_low();
            }
            5 => {
                self.pins[0].set_low();
                self.pins[1].set_low();
                self.pins[2].set_high();
                self.pins[3].set_high();
            }
            6 => {
                self.pins[0].set_low();
                self.pins[1].set_low();
                self.pins[2].set_low();
                self.pins[3].set_high();
            }
            7 => {
                self.pins[0].set_high();
                self.pins[1].set_low();
                self.pins[2].set_low();
                self.pins[3].set_high();
            }
            _ => panic!(),
        }
    }
    fn step_clock_wise(&mut self) {
        self.current += 1;
        self.current %= self.pins.len() * 2;
        self.update_pins();
    }
    fn step_counter_clock_wise(&mut self) {
        self.current = {
            if self.current == 0 {
                (self.pins.len() * 2) - 1
            } else {
                self.current - 1
            }
        };
        self.update_pins();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("Eveline start");
    // Retrieve the GPIO pins and configure them as outputs.
    let mut right_motor = Motor::new(Side::Right);
    let mut left_motor = Motor::new(Side::Left);

    let delay = {
        let rpm = 10;
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
    let mut last_time = Instant::now();
    let mut target_position = 200;
    while running.load(Ordering::SeqCst) {
        if last_time.elapsed().as_millis() >= delay {
            last_time = Instant::now();
            if left_motor.position == target_position {
                target_position += 200;
                target_position %= 400;
                println!("new target {}", target_position);
            }
            if left_motor.position > target_position {
                left_motor.step_down();
            } else {
                left_motor.step_up();
            }
            if right_motor.position > target_position {
                right_motor.step_down();
            } else {
                right_motor.step_up();
            }
        }
    }

    println!("Eveline done");

    Ok(())

    // When the pin variable goes out of scope, the GPIO pin mode is automatically reset
    // to its original value, provided reset_on_drop is set to true (default).
}
