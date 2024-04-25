use nalgebra::{Point2, Rotation2};

use crate::position::PositionMM;

pub fn square(position: &PositionMM, side_length: &f64) -> Vec<PositionMM> {
    vec![
        position.offset(side_length, &[0.0, 1.0]),
        position.offset(side_length, &[1.0, 1.0]),
        position.offset(side_length, &[1.0, 0.0]),
        position.offset(side_length, &[0.0, 0.0]),
    ]
}

// fn rotational_matrix(theta) {
//     todo!("finish this");
// }

pub fn star(position: &PositionMM, side_length: &f64) -> Vec<PositionMM> {
    let n = 13;
    let mut p:Point2<f64> = (*position).into();
    let r = side_length;
    let hist:Vec<PositionMM> = Vec::new();
    let mut d = Point2::<f64>::new(-1.0, -1.0);
    let ang: f64 = 180.0_f64 - 180.0_f64 / n as f64;
    let rot_mat = Rotation2::new(ang.to_radians());
    for _ in 0..n {
        d = rot_mat * d;
        todo!("isometry?")
        p += d * *r;
    }
    hist
}