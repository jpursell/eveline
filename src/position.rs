use std::ops::Index;

use crate::physical::Physical;


#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct PositionUM {
    xy: [usize; 2],
}

impl PositionUM {
    pub fn new(xy: [usize; 2]) -> Self {
        PositionUM { xy }
    }
    pub fn from_mm(xy: [f32; 2]) -> Self {
        Self::new(xy.map(|p| (p * 1000.0).round() as usize))
    }
    pub fn dist(&self, um: &PositionUM) -> f64 {
        let r2 = ((self[0] - um[0]).pow(2) + (self[1] - um[1]).pow(2)) as f64;
        r2.sqrt()
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

impl PositionStep {
    pub fn new(rr: [usize; 2]) -> Self {
        PositionStep {rr}
    }
}

impl Index<usize> for PositionStep {
    type Output = usize;
    fn index(&self, index: usize) -> &Self::Output {
        &self.rr[index]
    }
}

#[derive(Default, Clone, Copy)]
pub struct Position {
    um: PositionUM,
    step: PositionStep,
}

impl Position {
    pub fn new(um: PositionUM, step: PositionStep) -> Self {
        Position{um, step}
    }
    pub fn from_mm(xy:[f32;2], physical: &Physical) -> Self {
        let um = PositionUM::from_mm(xy);
        Self::from_um(um, physical)
    }
    pub fn from_um(um: PositionUM, physical: &Physical) -> Self {
        let rr = physical.get_motor_dist(&um);
        Position::new(um, rr)
    }
    fn dist(&self, um: &PositionUM) -> f64 {
        self.um.dist(um)
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