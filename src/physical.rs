use std::fmt::Display;

use crate::{
    motor::STEP_DIVISION,
    position::{PositionMM, PositionStep, PositionStepFloat},
};

pub struct Physical {
    motor_pos: [PositionMM; 2],
    x_limits: [f64; 2],
    y_limits: [f64; 2],
    y_offset: f64,
    steps_per_mm: f64,
    mm_per_step: f64,
    max_velocity: f32,
}

impl Display for Physical {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "motor_pos: [{}, {}], steps_per_mm: {}, mm_per_step: {}, max_velocity: {}",
            self.motor_pos[0],
            self.motor_pos[1],
            self.steps_per_mm,
            self.mm_per_step,
            self.max_velocity
        )
    }
}

impl Physical {
    pub fn new() -> Physical {
        let mm = PositionMM::new;
        let motor_pos = [mm([0.0, 368.8]), mm([297.0, 368.8])];
        let spool_radius: f64 = 5.75;
        let gear_ratio: f64 = (59.0_f64 / 17.0_f64).powi(2);
        let motor_steps_per_revolution = 100 * STEP_DIVISION;
        // left, right
        let spool_circumfrence = spool_radius * 2.0 * std::f64::consts::PI;
        // steps_per_mm is aprox 33.2
        let steps_per_mm = motor_steps_per_revolution as f64 * gear_ratio / spool_circumfrence;
        let max_rpm = 100.0_f32;
        // max_revs_per_second is about 1.7
        let max_revs_per_second = max_rpm / 60.0;
        // max_steps_per_second is about 170
        let max_steps_per_second = max_revs_per_second * motor_steps_per_revolution as f32;
        // max velocity is about 5 mm/s
        let max_velocity = max_steps_per_second / steps_per_mm as f32;
        let x_limits = [45.0, 260.0];
        let y_limits = [50.0, 328.0];
        let y_offset = -15.0;
        Physical {
            motor_pos,
            steps_per_mm,
            mm_per_step: 1.0 / steps_per_mm,
            max_velocity,
            x_limits,
            y_limits,
            y_offset,
        }
    }
    pub fn in_bounds(&self, position:&PositionMM) -> bool {
        let y_limits = self.y_limits.map(|x| x + self.y_offset);
        position.in_imits(&self.x_limits, &y_limits)
    }
    pub fn mm_to_step(&self, dist: &f64) -> f64 {
        dist * self.steps_per_mm
    }
    pub fn get_motor_dist_float(&self, mm: &PositionMM) -> PositionStepFloat {
        let mut rr = self.motor_pos.iter().map(|mp| {
            let r = mp.dist(mm);
            self.mm_to_step(&r)
        });
        let rr = [rr.next().unwrap(), rr.next().unwrap()];
        PositionStepFloat::new(rr)
    }
    pub fn get_motor_dist(&self, mm: &PositionMM) -> PositionStep {
        PositionStep::from_position_step_float(&self.get_motor_dist_float(mm))
    }
    pub fn get_motor_position(&self, index: usize) -> &PositionMM {
        &self.motor_pos[index]
    }
    pub fn get_max_velocity(&self) -> f32 {
        self.max_velocity
    }
    pub fn step_to_mm(&self, step: &usize) -> f64 {
        *step as f64 * self.mm_per_step
    }
}
