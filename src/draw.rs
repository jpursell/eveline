use crate::position::PositionMM;

pub fn square(position: &PositionMM, side_length: &f64) -> Vec<PositionMM> {
    vec![
        position.offset(side_length, &[0.0, 1.0]),
        position.offset(side_length, &[1.0, 1.0]),
        position.offset(side_length, &[1.0, 0.0]),
        position.offset(side_length, &[0.0, 0.0]),
    ]
}

fn rotational_matrix(theta) {
    todo!("finish this");

}
pub fn star(position: &PositionMM, side_length: &f64) -> Vec<PositionMM> {
    let n = 13;
    let p = position.copy();
    let r = side_length;
    let hist = Vec::new();
    let d = [-1, -1];
    let ang: f64 = 180.0_f64 - 180.0_f64 / n as f64;
    let rot_mat = todo!("finish");
}