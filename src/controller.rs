use std::{io, thread};

use crate::{
    motor::{Motor, Side},
    physical::Physical,
    position::{Position, PositionUM},
    predictor::{Prediction, Predictor},
    scurve::{SCurve, SCurveSolver},
};

enum HomeStatus {
    QueryPaper,
    QueryPosition,
    Moving,
    Complete,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum MoveStatus {
    Stopped,
    Moving,
}

pub struct Controller {
    current_position: Position,
    motors: [Motor; 2],
    home_status: HomeStatus,
    paper_origin: PositionUM,
    physical: Physical,
    move_status: MoveStatus,
    s_curve: SCurve,
    solver: SCurveSolver,
    predictor: Predictor,
}

impl Controller {
    pub fn new() -> Controller {
        let motors = [Side::Left, Side::Right].map(|s| Motor::new(s));
        let home_status = HomeStatus::QueryPaper;
        let physical = Physical::new();
        let max_acceleration = 1.0;
        let max_jerk = 1.0;
        let solver = SCurveSolver::new(&physical, max_acceleration, max_jerk);
        println!("solver: {}", solver);
        Controller {
            current_position: Position::default(),
            motors,
            home_status,
            paper_origin: PositionUM::default(),
            solver,
            physical,
            move_status: MoveStatus::Stopped,
            s_curve: SCurve::default(),
            predictor: Predictor::default(),
        }
    }
    fn get_position_from_user() -> Result<PositionUM, ()> {
        let mut input = String::new();
        if let Err(error) = io::stdin().read_line(&mut input) {
            println!("error: {error}");
            return Err(());
        }
        let xy = {
            let input = input.trim();
            let Some(xy_s) = input.split_once(",") else {
                println!("Did not get expected format");
                return Err(());
            };
            let xy_s = [xy_s.0, xy_s.1];
            println!("got {}, {}", xy_s[0], xy_s[1]);
            let mut xy_f: [f32; 2] = [0.0, 0.0];
            for (s, f) in xy_s.iter().zip(xy_f.iter_mut()) {
                if let Ok(pf) = s.parse::<f32>() {
                    *f = pf;
                } else {
                    println!("Failed to parse \"{}\"", s);
                    return Err(());
                }
            }
            xy_f
        };
        Ok(PositionUM::from_mm(xy))
    }
    fn set_current_position_from_user(&mut self) -> Result<(), ()> {
        println!("What's the current position in mm? provide \"x,y\"");
        for _ in 0..4 {
            if let Ok(um) = Controller::get_position_from_user() {
                self.current_position = Position::from_um(um, &self.physical);
                return Ok(());
            }
        }
        Err(())
    }
    fn set_paper_origin_from_user(&mut self) -> Result<(), ()> {
        println!(
            "What's the location of the lower left corner of the paper in mm? provide \"x,y\""
        );
        for _ in 0..4 {
            if let Ok(um) = Controller::get_position_from_user() {
                self.paper_origin = um;
                return Ok(());
            }
        }
        Err(())
    }
    /// Initialize move to new location. Set up s-curve and change status.
    fn init_move(&mut self, um: &PositionUM) {
        println!("init_move");
        if *um == self.current_position {
            self.move_status = MoveStatus::Stopped;
            return;
        }
        // init s-curve
        self.s_curve = self.solver.solve_curve(self.current_position.into(), *um);
        println!("s-curve {}", self.s_curve);
        self.predictor = Predictor::new();
        self.move_status = MoveStatus::Moving;
    }
    /// Move current position in steps to (x, y)
    fn update_move(&mut self) {
        self.move_status = self.s_curve.get_move_status();
        if self.move_status == MoveStatus::Stopped {
            return;
        }
        let desired = self.s_curve.get_desired(&self.solver);
        match self.predictor.predict(&self.current_position, &desired) {
            Prediction::Wait(duration) => {
                thread::sleep(duration);
            }
            Prediction::MoveMotors(instructions) => {
                instructions
                    .iter()
                    .zip(self.motors.iter_mut())
                    .for_each(|(instruction, motor)| motor.step(instruction));
            }
        }
    }
    pub fn update(&mut self) {
        match self.home_status {
            HomeStatus::Complete => {
                todo!()
            }
            HomeStatus::QueryPaper => {
                if let Ok(_) = self.set_paper_origin_from_user() {
                    self.home_status = HomeStatus::QueryPosition;
                }
            }
            HomeStatus::QueryPosition => {
                if let Ok(_) = self.set_current_position_from_user() {
                    if self.current_position == self.paper_origin {
                        self.home_status = HomeStatus::Complete;
                    } else {
                        self.init_move(&self.paper_origin.clone());
                        self.home_status = HomeStatus::Moving;
                    }
                }
            }
            HomeStatus::Moving => {
                self.update_move();
                if self.move_status == MoveStatus::Stopped {
                    self.home_status = HomeStatus::QueryPosition;
                }
            }
        }
    }
}
