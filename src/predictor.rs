use std::time::{Duration, Instant};

use crate::{
    motor::StepInstruction,
    position::{Position, PositionStepFloat},
};

pub enum Prediction {
    Wait(Duration),
    MoveMotors([StepInstruction; 2]),
}

pub struct Predictor {
    last_instant: Instant,
    last_remainder: [f64; 2],
}

impl Default for Predictor {
    fn default() -> Self {
        Predictor {
            last_instant: Instant::now(),
            last_remainder: [f64::default(); 2],
        }
    }
}

impl Predictor {
    pub fn new() -> Self {
        Predictor::default()
    }
    pub fn predict(
        &mut self,
        current_position: &Position,
        desired: &PositionStepFloat,
    ) -> Prediction {
        let mut remainders = desired
            .iter()
            .zip(current_position.iter_step())
            .map(|(&desired, &current)| desired - current as f64);
        let remainders = [remainders.next().unwrap(), remainders.next().unwrap()];
        // info!("r: {} {}",remainders[0], remainders[1]);
        let mut move_now = false;
        let instructions = remainders.map(|r| {
            if r > 1.0 {
                move_now = true;
                StepInstruction::StepUp
            } else if r < -1.0 {
                move_now = true;
                StepInstruction::StepDown
            } else {
                StepInstruction::Hold
            }
        });
        // let mut wait_time = if !move_now {
        //     // r0 = m * t0 + b
        //     // sub 0 for t0
        //     // r0 = b
        //     let remainder_diff = remainders
        //         .iter()
        //         .zip(self.last_remainder.iter())
        //         .map(|(r, lr)| r - lr);
        //     let time_diff = self.last_instant.elapsed().as_secs_f64();
        //     let m = remainder_diff.map(|rd| rd / time_diff);
        //     // r = m dt + r0
        //     // dt = (sign(m) - r0)/m
        //     let mut pred_time = m.zip(self.last_remainder.iter()).map(|(m, r0)| {
        //         let sign_m = if m > 0.0 { 1.0 } else { -1.0 };
        //         (sign_m - r0) / m
        //     });
        //     let pred_time = [pred_time.next().unwrap(), pred_time.next().unwrap()];
        //     pred_time[0].min(pred_time[1])
        // } else {
        //     0.0
        // };
        // info!("wait_time: {}", wait_time);
        // wait_time = wait_time.min(0.01);
        let wait_time = 0.0;
        self.last_instant = Instant::now();
        self.last_remainder = remainders;
        if move_now {
            return Prediction::MoveMotors(instructions);
        }
        Prediction::Wait(Duration::from_secs_f64(wait_time))
    }
}
