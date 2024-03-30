use std::time::Instant;

use crate::{controller::MoveStatus, physical::Physical, position::{PositionStep, PositionUM}};


#[derive(Default)]
pub struct SCurve {
    start: PositionUM,
    end: PositionUM,
}

impl SCurve {
    pub fn new(start: PositionUM, end: PositionUM, now: Instant, physical: &Physical) -> Self{
        todo!();
        SCurve{start, end}
    }
    /// Return if we are in the process of moving or not
    pub fn get_move_status(&self, now:&Instant) -> MoveStatus {
        todo!();
    }
    /// Return the desired step of the motors
    pub fn get_desired(&self, now:&Instant, physical: &Physical) -> PositionStep {
        todo!();
    }
}