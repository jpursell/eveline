use std::{io, thread, time::Duration};

use crate::{
    motor::{Motor, Side, StepInstruction},
    physical::Physical,
    position::{Position, PositionStep, PositionMM},
    predictor::{Prediction, Predictor},
    scurve::{SCurve, SCurveSolver},
};

enum ControllerMode {
    Step,
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
    current_position_initialized: bool,
    motors: [Motor; 2],
    mode: ControllerMode,
    paper_origin: PositionMM,
    physical: Physical,
    move_status: MoveStatus,
    s_curve: SCurve,
    solver: SCurveSolver,
    predictor: Predictor,
}

impl Controller {
    pub fn new() -> Controller {
        let motors = [Side::Left, Side::Right].map(|s| Motor::new(s));
        let physical = Physical::new();
        println!("Physical: {}", physical);
        let max_acceleration = 1.0;
        let max_jerk = 1.0;
        let solver = SCurveSolver::new(&physical, max_acceleration, max_jerk);
        println!("solver: {}", solver);
        Controller {
            current_position: Position::default(),
            current_position_initialized: false,
            motors,
            mode: ControllerMode::Step,
            paper_origin: PositionMM::default(),
            solver,
            physical,
            move_status: MoveStatus::Stopped,
            s_curve: SCurve::default(),
            predictor: Predictor::default(),
        }
    }
    fn get_position_from_user() -> Result<PositionMM, ()> {
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
            let mut xy_f: [f64; 2] = [0.0, 0.0];
            for (s, f) in xy_s.iter().zip(xy_f.iter_mut()) {
                if let Ok(pf) = s.parse::<f64>() {
                    *f = pf;
                } else {
                    println!("Failed to parse \"{}\"", s);
                    return Err(());
                }
            }
            xy_f
        };
        Ok(PositionMM::new(xy))
    }
    fn set_current_position_from_user(&mut self) -> Result<(), ()> {
        println!("What's the current position in mm? provide \"x,y\"");
        for _ in 0..1 {
            if let Ok(mm) = Controller::get_position_from_user() {
                self.current_position = Position::from_mm(mm, &self.physical);
                self.current_position_initialized = true;
                println!("position set to {}", self.current_position);
                return Ok(());
            }
        }
        Err(())
    }
    fn set_paper_origin_from_user(&mut self) -> Result<(), ()> {
        println!(
            "What's the location of the lower left corner of the paper in mm? provide \"x,y\""
        );
        for _ in 0..1 {
            if let Ok(mm) = Controller::get_position_from_user() {
                self.paper_origin = mm;
                return Ok(());
            }
        }
        Err(())
    }
    /// Initialize move to new location. Set up s-curve and change status.
    fn init_move(&mut self, mm: &PositionMM) {
        println!("init_move");
        if self.current_position.very_close_to(mm, &self.physical) {
            self.move_status = MoveStatus::Stopped;
            return;
        }
        // init s-curve
        self.s_curve = self.solver.solve_curve(self.current_position.into(), *mm);
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
                self.implement_step_instructions(instructions);
            }
        }
    }
    fn implement_step_instructions(&mut self, instructions: [StepInstruction; 2]) {
        let mut step: PositionStep = self.current_position.get_step().to_owned();
        instructions
            .iter()
            .enumerate()
            .for_each(|(i, instruction)| {
                self.motors[i].step(instruction);
                step.step(i, instruction);
            });
        self.current_position = Position::from_step(step, &self.physical);
        println!("new position {}", self.current_position);
    }
    fn step(&mut self) {
        if !self.current_position_initialized {
            let _ = self.set_current_position_from_user();
            return;
        }
        self.implement_step_instructions([StepInstruction::StepUp; 2]);
        thread::sleep(Duration::from_secs_f64(0.5));
        self.implement_step_instructions([StepInstruction::StepDown; 2]);
        thread::sleep(Duration::from_secs_f64(0.5));
    }
    pub fn update(&mut self) {
        match self.mode {
            ControllerMode::Step => {
                self.step();
            }
            ControllerMode::Complete => {
                todo!()
            }
            ControllerMode::QueryPaper => {
                if let Ok(_) = self.set_paper_origin_from_user() {
                    self.mode = ControllerMode::QueryPosition;
                }
            }
            ControllerMode::QueryPosition => {
                if let Ok(_) = self.set_current_position_from_user() {
                    if self.current_position.very_close_to(&self.paper_origin, &self.physical) {
                        self.mode = ControllerMode::Complete;
                    } else {
                        self.init_move(&self.paper_origin.clone());
                        self.mode = ControllerMode::Moving;
                    }
                }
            }
            ControllerMode::Moving => {
                self.update_move();
                if self.move_status == MoveStatus::Stopped {
                    self.mode = ControllerMode::QueryPosition;
                }
            }
        }
    }
}
