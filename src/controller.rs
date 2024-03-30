use std::{io, time::Instant};

use crate::{motor::{Motor, Side}, physical::Physical, position::{Position, PositionUM}, scurve::SCurve};


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
}

impl Controller {
    pub fn new() -> Controller {
        let motors = [Side::Left, Side::Right].map(|s| Motor::new(s));
        let home_status = HomeStatus::QueryPaper;
        Controller {
            current_position: Position::default(),
            motors,
            home_status,
            paper_origin: PositionUM::default(),
            physical: Physical::new(),
            move_status: MoveStatus::Stopped,
            s_curve: SCurve::default(),
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
                let Ok(f) = s.parse::<f32>() else {
                    println!("Failed to parse \"{}\"", s);
                    return Err(());
                };
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
    /// Move current position in steps to (x, y)
    fn init_move(&mut self, um: &PositionUM) {
        if *um == self.current_position {
            self.move_status == MoveStatus::Stopped;
            return;
        }
        // init s-curve
        self.s_curve = SCurve::new(self.current_position.into(), *um,Instant::now(), &self.physical);
        self.move_status = MoveStatus::Moving;
    }
    /// Move current position in steps to (x, y)
    fn update_move(&mut self) {
        let now = Instant::now();
        self.move_status = self.s_curve.get_move_status(&now);
        if self.move_status == MoveStatus::Stopped {
            return;
        }
        let desired = self.s_curve.get_desired(&now, &self.physical);
        todo!();
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
                        self.init_move(&self.paper_origin);
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