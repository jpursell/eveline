use nalgebra::{Point2, Rotation2, Vector2};
use ndarray::prelude::*;

use crate::position::PositionMM;

pub enum Pattern {
    Square,
    Star,
    Wave,
    Spiralgraph,
    HeartWave,
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
    let mut p: Point2<f64> = (*position).into();
    let r = side_length;
    let mut hist: Vec<PositionMM> = Vec::new();
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

pub fn wave(
    position: &PositionMM,
    spacing: &f64,
    length: &f64,
    amplitude: &f64,
    period: &f64,
) -> Vec<PositionMM> {
    let n = (length / spacing) as usize;
    let mut pts = Vec::new();
    let y_scale = 2.0 * std::f64::consts::PI / period;
    for i in 0..n {
        let x = i as f64 * spacing;
        let y = (x * y_scale).sin() * amplitude / 2.0;
        pts.push(PositionMM::new([x + position.x(), y + position.y()]));
    }
    pts
}

pub fn spiralgraph(position: &PositionMM, radius: &f64) -> Vec<PositionMM> {
    let n = 1800;
    let dr = 1.0_f64.to_radians();
    let a = -18.0;

    let mut pts = Vec::new();
    for i in 0..n {
        let t = i as f64 * dr;
        let r = radius * (a / 5.0 * t).sin();
        let x = r * t.cos();
        let y = r * t.sin();
        pts.push(PositionMM::new([x + position.x(), y + position.y()]));
    }
    pts
}

pub fn heart_wave(position: &PositionMM, size: &f64) -> Vec<PositionMM> {
    let a = 20.0;
    let x_arr: Array1<f64> = Array::linspace(-2.0, 2.0, 500);
    let y_arr: Array1<f64> = x_arr.mapv(|x| {
        x.abs().powf(2.0 / 3.0)
            + 0.9 * (3.3 - x.powi(2)).powf(0.5) * (a * std::f64::consts::PI * x).sin()
    });

    let mut pts = Vec::new();
    azip!((&x in &x_arr, &y in &y_arr){
        if !y.is_nan() {
            pts.push(PositionMM::new([x, y]));
        }
    });

    let cur_size = pts[pts.len() - 1].x() - pts[0].x();
    let scale = size / cur_size;

    let mut pts2 = Vec::new();
    for pt in &pts {
        let x = (pt.x() - pts[0].x()) * scale + position.x();
        let y = (pt.y() - pts[0].y()) * scale + position.y();
        pts2.push(PositionMM::new([x, y]));
    }
    pts2
}
