use crate::position::PositionMM;

pub fn square(position: &PositionMM, side_length: &f64) -> Vec<PositionMM> {
    todo!("currently moves left up right down");
    vec![
        position.offset(side_length, &[-1.0, 0.0]),
        position.offset(side_length, &[-1.0, 1.0]),
        position.offset(side_length, &[0.0, 1.0]),
        position.offset(side_length, &[0.0, 0.0]),
    ]
}
