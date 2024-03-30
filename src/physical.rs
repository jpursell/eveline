use crate::{motor::STEP_DIVISION, position::{PositionStep, PositionUM}};


pub struct Physical {
    motor_pos: [PositionUM; 2],
    steps_per_mm: f64,
    max_velocity: f32,
    max_acceleration: f32,
    max_jerk: f32,
}

impl Physical {
    pub fn new() -> Physical {
        let mm = PositionUM::from_mm;
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
        Physical {
            motor_pos,
            steps_per_mm,
            max_velocity,
            max_acceleration: 1.0,
            max_jerk: 1.0,
        }
    }
    pub fn get_motor_dist(&self, um: &PositionUM) -> PositionStep {
        let mut rr = self.motor_pos.iter()
            .map(|mp| {
                let r = mp.dist(um);
                let step = r * self.steps_per_mm;
                step.round() as usize
            }
        );
        let rr = [rr.next().unwrap(), rr.next().unwrap()];
        PositionStep::new(rr)
    }
}