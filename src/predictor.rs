use std::time::{Duration, Instant};

use crate::{motor::StepInstruction, position::{PositionStep, PositionStepFloat}};

pub enum Prediction {
    WaitNanos(Duration),
    MoveMotors([StepInstruction;2]),
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
    pub fn predict(&self, current_position: &PositionStep, desired: &PositionStepFloat) -> Prediction {
        let mut remainders = desired
            .iter()
            .zip(current_position.iter())
            .map(|(&desired, &current)| desired - current as f64);
        let remainders = [remainders.next().unwrap(), remainders.next().unwrap()];
        let mut move_now = false;
        let instructions = remainders.map(|r|{
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
        todo!("Finish this. We need to make sure we update last_instant, last_remainder even if we are moving now. Put in code for sleep prediction.")
        // let mut max_remainder: f64 = 0.0;
        // for (num, &remainder) in remainder.iter().enumerate() {
        //     if remainder > 1.0 {
        //         instructions[num] = StepInstruction::StepUp;
        //         move_now = true;
        //     } else if remainder < -1.0 {
        //         instructions[num] = StepInstruction::StepDown;
        //         move_now = true;
        //     } else {
        //         max_remainder = max_remainder.max(remainder.abs());
        //     }
        // }
        if move_now {
            return Prediction::MoveMotors(instructions);
        }

        let since = self.last.elapsed().as_secs_f64();
        let advancement = max_remainder
        Prediction::WaitNanos(Duration::from_secs_f64(0.0))
    }
}
