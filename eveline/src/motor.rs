use std::{fmt::Display, time::Instant};

use rppal::gpio::{Gpio, OutputPin};

// Gpio uses BCM pin numbering. BCM GPIO 23 is tied to physical pin 16.
const RIGHT_PINS: [u8; 4] = [4, 22, 17, 27];
const LEFT_PINS: [u8; 4] = [12, 21, 16, 20];
const PWM_FREQ: f64 = 200.0;
pub const STEP_DIVISION: usize = 1;

#[derive(Clone, Copy)]
pub enum StepInstruction {
    StepLonger,
    StepShorter,
    Hold,
}

pub struct Motor {
    pins: [OutputPin; 4],
    current: usize,
    position: i32,
    side: Side,
    current_pwm: usize,
    current_on: usize,
    min_seconds_per_step: f64,
    time_last_step: Instant,
}

pub enum Side {
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
    pub fn new(side: Side, min_seconds_per_step: f64) -> Motor {
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
        match STEP_DIVISION {
            1 => {
                pins[current_pwm].set_high();
            }
            2 => {
                pins[current_pwm].set_low();
            }
            _ => {
                pins[current_pwm].set_pwm_frequency(PWM_FREQ, 0.0).unwrap();
            }
        }
        Motor {
            pins,
            current,
            position: 0,
            side,
            current_pwm,
            current_on,
            min_seconds_per_step,
            time_last_step: Instant::now(),
        }
    }
    fn step_shorter(&mut self) {
        match self.side {
            Side::Left => self.step_counter_clock_wise(),
            Side::Right => self.step_clock_wise(),
        };
        self.position -= 1;
    }
    fn step_longer(&mut self) {
        match self.side {
            Side::Left => self.step_clock_wise(),
            Side::Right => self.step_counter_clock_wise(),
        };
        self.position += 1;
    }
    pub fn step(&mut self, instruction: &StepInstruction) -> Result<(), ()> {
        match instruction {
            StepInstruction::StepLonger | StepInstruction::StepShorter => {
                if self.time_last_step.elapsed().as_secs_f64() < self.min_seconds_per_step {
                    return Err(());
                }
                self.time_last_step = Instant::now();
            }
            StepInstruction::Hold => return Ok(()),
        }
        match instruction {
            StepInstruction::StepLonger => {
                self.step_longer();
            }
            StepInstruction::StepShorter => {
                self.step_shorter();
            }
            StepInstruction::Hold => {}
        }
        Ok(())
    }
    /// Update pins for whole step mode
    fn update_pins_whole_step(&mut self) {
        let main_pin = self.current;
        let secondary_pin = (main_pin + 1) % self.pins.len();
        if main_pin == self.current_on {
            // nothing happened
            return;
        } else if main_pin == self.current_pwm {
            // current was increased
            self.pins[self.current_on].set_low();
            self.pins[secondary_pin].set_high();
        } else if secondary_pin == self.current_on {
            // current was decreased
            self.pins[self.current_pwm].set_low();
            self.pins[main_pin].set_high();
        } else {
            panic!();
        }
        self.current_on = main_pin;
        self.current_pwm = secondary_pin;
    }
    fn update_pins(&mut self) {
        if STEP_DIVISION == 1 {
            return self.update_pins_whole_step();
        }
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
        let phase = if STEP_DIVISION == 1 {
            0
        } else {
            self.current % STEP_DIVISION
        };
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
        if self.current_pwm != pwm_pin {
            self.pins[self.current_pwm].clear_pwm().unwrap();
            self.pins[self.current_pwm].set_low();
            self.current_pwm = pwm_pin;
        }
        if self.current_on != on_pin {
            self.pins[on_pin].set_high();
            self.pins[self.current_on].set_low();
            self.current_on = on_pin;
        }
        if STEP_DIVISION == 2 {
            if duty_cycle == 1.0 {
                self.pins[pwm_pin].set_high();
            } else if duty_cycle == 0.0 {
                self.pins[pwm_pin].set_low();
            } else {
                panic!()
            }
        } else {
            self.pins[pwm_pin]
                .set_pwm_frequency(PWM_FREQ, duty_cycle)
                .unwrap();
        }
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
