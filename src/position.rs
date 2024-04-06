use std::{fmt::Display, ops::Index};

use crate::{motor::StepInstruction, physical::Physical};

#[derive(Default, Copy, Clone)]
pub struct PositionMM {
    xy: [f64; 2],
}

impl PositionMM {
    pub fn new(xy: [f64; 2]) -> Self {
        PositionMM { xy }
    }
    pub fn iter(&self) -> impl Iterator<Item = &f64> {
        self.xy.iter()
    }
    /// Distance in mm
    pub fn dist(&self, mm: &PositionMM) -> f64 {
        ((self[0] - mm[0]).powi(2) + (self[1] - mm[1]).powi(2)).sqrt()
    }
    pub fn get_direction(&self, mm: &PositionMM) -> [f64; 2] {
        let dist = self.dist(mm);
        let mut xy = self
            .iter()
            .zip(mm.iter())
            .map(|(xy0, xy1)| (xy1 - xy0) / dist);
        [xy.next().unwrap(), xy.next().unwrap()]
    }
}

impl Display for PositionMM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "xy:[{}, {}]", self.xy[0], self.xy[1])
    }
}

impl Index<usize> for PositionMM {
    type Output = f64;
    fn index(&self, index: usize) -> &Self::Output {
        &self.xy[index]
    }
}

#[derive(Default, Clone, Copy)]
pub struct PositionStep {
    rr: [usize; 2],
}

impl Display for PositionStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rr:[{}, {}]", self.rr[0], self.rr[1])
    }
}

impl PositionStep {
    pub fn new(rr: [usize; 2]) -> Self {
        PositionStep { rr }
    }
    pub fn iter(&self) -> impl Iterator<Item = &usize> {
        self.rr.iter()
    }
    pub fn step(&mut self, index: usize, instruction: &StepInstruction) {
        match instruction {
            StepInstruction::StepUp => {
                self.rr[index] += 1;
            }
            StepInstruction::StepDown => {
                self.rr[index] -= 1;
            }
            StepInstruction::Hold => {}
        }
    }
}

impl Index<usize> for PositionStep {
    type Output = usize;
    fn index(&self, index: usize) -> &Self::Output {
        &self.rr[index]
    }
}

pub struct PositionStepFloat {
    rr: [f64; 2],
}

impl PositionStepFloat {
    pub fn new(rr: [f64; 2]) -> Self {
        PositionStepFloat { rr }
    }
    pub fn iter(&self) -> impl Iterator<Item = &f64> {
        self.rr.iter()
    }
    fn from_position_step(step: &PositionStep, physical: &Physical) -> Self {
        let rr = step.rr.map(|r| physical.step_to_mm(&r));
        Self::new(rr)
    }
}

impl Index<usize> for PositionStepFloat {
    type Output = f64;
    fn index(&self, index: usize) -> &Self::Output {
        &self.rr[index]
    }
}

#[derive(Default, Clone, Copy)]
pub struct Position {
    mm: PositionMM,
    step: PositionStep,
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mm:[{}], step:[{}]", self.mm, self.step)
    }
}

impl Position {
    pub fn new(mm: PositionMM, step: PositionStep) -> Self {
        Position { mm, step }
    }
    pub fn from_step(step: PositionStep, physical: &Physical) -> Self {
        let stepf: PositionStepFloat = PositionStepFloat::from_position_step(&step, physical);
        let r_m0 = stepf[0];
        let r_m1 = stepf[1];
        // solved for let motor_pos = [mm([0.0, 368.8]), mm([297.0, 368.8])];
        let x = 0.00168350168350168 * r_m0.powi(2) - 0.00168350168350168 * r_m1.powi(2) + 148.5;
        let pos_m0: &PositionMM = physical.get_motor_position(0);
        let x_m0: f64 = pos_m0[0];
        let y_m0: f64 = pos_m0[1];
        let y = (r_m0.powi(2) - (x - x_m0).powi(2)).sqrt() + y_m0;
        let mm = PositionMM::new([x, y]);
        Position::new(mm, step)
    }
    pub fn from_mm(mm: PositionMM, physical: &Physical) -> Self {
        let rr = physical.get_motor_dist(&mm);
        Position::new(mm, rr)
    }
    pub fn get_step(&self) -> &PositionStep {
        &self.step
    }
    pub fn iter_step(&self) -> impl Iterator<Item = &usize> {
        self.step.iter()
    }
}

impl PartialEq<PositionMM> for Position {
    fn eq(&self, other: &PositionMM) -> bool {
        self.mm[0] == other[0] && self.mm[1] == other[1]
    }
}
impl PartialEq<Position> for PositionMM {
    fn eq(&self, other: &Position) -> bool {
        other.mm[0] == self[0] && other.mm[1] == self[1]
    }
}

impl From<Position> for PositionMM {
    fn from(value: Position) -> Self {
        value.mm
    }
}

impl From<Position> for PositionStep {
    fn from(value: Position) -> Self {
        value.step
    }
}
