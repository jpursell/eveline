use std::error::Error;
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

// The simple-signal crate is used to handle incoming signals.
use simple_signal::{self, Signal};

use rppal::gpio::{Gpio, OutputPin};

// Gpio uses BCM pin numbering. BCM GPIO 23 is tied to physical pin 16.
const RIGHT_PINS: [u8; 4] = [4, 22, 17, 27];
const LEFT_PINS: [u8; 4] = [12, 21, 16, 20];
const PWM_FREQ: f64 = 100.0;
const STEP_DIVISION: usize = 4;

struct Motor {
    pins: [OutputPin; 4],
    current: usize,
    position: i32,
    side: Side,
    current_pwm: usize,
    current_on: usize,
}

enum Side {
    Left,
    Right,
}
impl Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Left => write!(f, "L"),
            Side::Right => write!(f, "R"),
        }
    }
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
        pins.iter_mut().for_each(|pin| pin.set_low());
        let current = 0;
        let current_on = 0;
        pins[current_on].set_high();
        let current_pwm = 1;
        pins[current_pwm].set_pwm_frequency(PWM_FREQ, 0.0).unwrap();
        Motor {
            pins,
            current,
            position: 0,
            side,
            current_pwm,
            current_on,
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
        // pin 0      [ 1, 1, 1,.7, 0, 0, 0, 0, 0, 0, 0, 0, 0,.7, 1, 1]
        // pin 1      [ 0,.7, 1, 1, 1, 1, 1,.7, 0, 0, 0, 0, 0, 0, 0, 0]
        // pin 2      [ 0, 0, 0, 0, 0,.7, 1, 1, 1, 1, 1,.7, 0, 0, 0, 0]
        // pin 3      [ 0, 0, 0, 0, 0, 0, 0, 0, 0,.7, 1, 1, 1, 1, 1,.7]
        // c/4        [ 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3]
        // (c+4)%16/4 [ 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 0, 0, 0, 0]
        // c%4        [ 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3]
        let main_pin = self.current / STEP_DIVISION;
        let secondary_pin = (main_pin + 1) % self.pins.len();
        let phase = self.current % STEP_DIVISION;
        println!(
            "{} current {} main {} sec {} phase {}",
            self.side, self.current, main_pin, secondary_pin, phase
        );
        let (on_pin, pwm_pin, duty_cycle) = if phase < STEP_DIVISION / 2 {
            let on_pin = main_pin;
            let pwm_pin = secondary_pin;
            let duty_cycle = (phase as f64 / STEP_DIVISION as f64 * std::f64::consts::PI).sin();
            (on_pin, pwm_pin, duty_cycle)
        } else {
            let on_pin = secondary_pin;
            let pwm_pin = main_pin;
            let duty_cycle = ((STEP_DIVISION - phase) as f64 / STEP_DIVISION as f64
                * std::f64::consts::PI)
                .sin();
            (on_pin, pwm_pin, duty_cycle)
        };
        println!(
            "{} on_pin {} pwm_pin {} duty {}",
            self.side, on_pin, pwm_pin, duty_cycle
        );
        if self.current_pwm != pwm_pin {
            println!("{} clear pwm on {}", self.side, self.current_pwm);
            self.pins[self.current_pwm].clear_pwm().unwrap();
            self.pins[self.current_pwm].set_low();
            self.current_pwm = pwm_pin;
        }
        if self.current_on != on_pin {
            println!("{} turn off {}", self.side, self.current_on);
            self.pins[on_pin].set_high();
            self.pins[self.current_on].set_low();
            self.current_on = on_pin;
        }
        self.pins[pwm_pin]
            .set_pwm_frequency(PWM_FREQ, duty_cycle)
            .unwrap();
    }
    fn step_clock_wise(&mut self) {
        self.current += 1;
        self.current %= self.pins.len() * STEP_DIVISION;
        self.update_pins();
    }
    fn step_counter_clock_wise(&mut self) {
        self.current = {
            if self.current == 0 {
                (self.pins.len() * STEP_DIVISION) - 1
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
        let rpm = 0.1;
        let steps = 100.0 * STEP_DIVISION as f64;
        let steps_per_minute = steps * rpm;
        let seconds_per_minute = 60.0;
        seconds_per_minute / steps_per_minute
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
    let mut target_position = 100 * STEP_DIVISION as i32;
    while running.load(Ordering::SeqCst) {
        if last_time.elapsed().as_secs_f64() as f64 >= delay {
            last_time = Instant::now();
            if left_motor.position == target_position {
                target_position += 100 * STEP_DIVISION as i32;
                target_position %= 200 * STEP_DIVISION as i32;
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
