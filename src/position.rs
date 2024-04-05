use std::{fmt::Display, ops::Index};

use crate::{motor::StepInstruction, physical::Physical};

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct PositionUM {
    xy: [usize; 2],
}

impl Display for PositionUM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "xy: [{}, {}]", self.xy[0], self.xy[1])
    }
}

impl PositionUM {
    pub fn new(xy: [usize; 2]) -> Self {
        PositionUM { xy }
    }
    pub fn from_mm(xy: [f32; 2]) -> Self {
        Self::new(xy.map(|p| (p * 1000.0).round() as usize))
    }
    /// Distance in mm
    pub fn dist(&self, um: &PositionUM) -> f64 {
        let r2 = ((self[0] - um[0]).pow(2) + (self[1] - um[1]).pow(2)) as f64;
        r2.sqrt() / 1000.0
    }
    pub fn get_direction(&self, um: &PositionUM) -> [f64; 2] {
        let mut xy = self
            .xy
            .iter()
            .zip(um.xy.iter())
            .map(|(xy0, xy1)| (xy1 - xy0) as f64 / 1000.0);
        [xy.next().unwrap(), xy.next().unwrap()]
    }
    pub fn iter(&self) -> impl Iterator<Item = &usize> {
        self.xy.iter()
    }
}

impl Index<usize> for PositionUM {
    type Output = usize;
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
            StepInstruction::StepUp => {self.rr[index] += 1;}
            StepInstruction::StepDown => {self.rr[index] -= 1;}
            StepInstruction::Hold => {},
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
}

#[derive(Default, Clone, Copy)]
pub struct Position {
    um: PositionUM,
    step: PositionStep,
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "um:[{}], step:[{}]", self.um, self.step)
    }
}

impl Position {
    pub fn new(um: PositionUM, step: PositionStep) -> Self {
        Position { um, step }
    }
    // pub fn from_mm(xy: [f32; 2], physical: &Physical) -> Self {
    //     let um = PositionUM::from_mm(xy);
    //     Self::from_um(um, physical)
    // }
    pub fn from_step(step: PositionStep, physical: &Physical) -> Self {
        let r_m0 = (step[0] as f64) / 1000.0;
        let r_m1 = (step[1] as f64) / 1000.0;
        // solved for let motor_pos = [mm([0.0, 368.8]), mm([297.0, 368.8])];
        let x = 0.00168350168350168 * r_m0.powi(2) - 0.00168350168350168*r_m1.powi(2) + 148.5;
        let x_m0 = physical.get_motor_position(0)[0] as f64 / 1000.0;
        let y_m0 = physical.get_motor_position(0)[1] as f64 / 1000.0;
        let y = (r_m0.powi(2) - (x - x_m0).powi(2)).sqrt() + y_m0;
        let um = PositionUM::from_mm([x as f32, y as f32]);
        Position::new(um, step)
    }
    pub fn from_um(um: PositionUM, physical: &Physical) -> Self {
        let rr = physical.get_motor_dist(&um);
        Position::new(um, rr)
    }
    pub fn get_step(&self) -> &PositionStep {
        &self.step
    }
    /// Distance in mm
    // pub fn dist(&self, um: &PositionUM) -> f64 {
    //     self.um.dist(um)
    // }
    pub fn iter_step(&self) -> impl Iterator<Item = &usize> {
        self.step.iter()
    }
}

impl PartialEq<PositionUM> for Position {
    fn eq(&self, other: &PositionUM) -> bool {
        self.um == *other
    }
}
impl PartialEq<Position> for PositionUM {
    fn eq(&self, other: &Position) -> bool {
        *other == *self
    }
}

impl From<Position> for PositionUM {
    fn from(value: Position) -> Self {
        value.um
    }
}

impl From<Position> for PositionStep {
    fn from(value: Position) -> Self {
        value.step
    }
}