use nalgebra::{Point2, Rotation2, Vector2};

use crate::position::PositionMM;

pub enum Pattern {
    Square,
    Star,
}

pub fn square(position: &PositionMM, side_length: &f64) -> Vec<PositionMM> {
    vec![
        position.offset(side_length, &[0.0, 1.0]),
        position.offset(side_length, &[1.0, 1.0]),
        position.offset(side_length, &[1.0, 0.0]),
        position.offset(side_length, &[0.0, 0.0]),
    ]
}

pub fn star(position: &PositionMM, side_length: &f64) -> Vec<PositionMM> {
    let n = 13;
    let mut p:Point2<f64> = (*position).into();
    let r = side_length;
    let mut hist:Vec<PositionMM> = Vec::new();
    let mut d = Vector2::<f64>::new(-1.0, -1.0);
    let ang: f64 = 180.0_f64 - 180.0_f64 / n as f64;
    let rot_mat = Rotation2::new(ang.to_radians());
    for _ in 0..n {
        d = rot_mat * d;
        p += d * *r;
        hist.push(p.into());
    }

    let mut ptr = 0;
    for _ in 0..n {
        ptr += 2;
        ptr %= n;
        hist.push(hist[ptr]);
    }
    hist
}