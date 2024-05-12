use std::{io, path::PathBuf};

use log::{error, info};

use crate::{
    draw::{heart_wave, spiralgraph, square, star, wave},
    gcode::{Axis, AxisLimit, PlotterInstruction, PlotterProgram},
    motor::{Motor, Side, StepInstruction},
    physical::Physical,
    position::{Position, PositionMM, PositionStep},
    predictor::{Prediction, Predictor},
    scurve::{SCurve, SCurveSolver},
};

enum ControllerMode {
    Ask,
    MoveTo,
    QueryPaper,
    QueryPosition,
    InitProgram,
    RunProgram,
    LoadPattern,
    ScaleProgram,
    CenterProgram,
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
    paper_limits: Option<[AxisLimit; 2]>,
    physical: Physical,
    move_status: MoveStatus,
    s_curve: SCurve,
    solver: SCurveSolver,
    predictor: Predictor,
    wait_count: usize,
    program: Option<PlotterProgram>,
}

impl Controller {
    pub fn new(gcode_path: Option<PathBuf>) -> Controller {
        let motors = [Side::Left, Side::Right].map(Motor::new);
        let physical = Physical::new();
        info!("Physical: {physical}");
        let max_acceleration = 1e4;
        let max_jerk = 1e9;
        let solver = SCurveSolver::new(&physical, max_acceleration, max_jerk);
        info!("solver: {solver}");
        let gcode_program = Controller::load_gcode(&gcode_path);
        Controller {
            current_position: Position::default(),
            current_position_initialized: false,
            motors,
            mode: ControllerMode::QueryPosition,
            paper_limits: None,
            solver,
            physical,
            move_status: MoveStatus::Stopped,
            s_curve: SCurve::default(),
            predictor: Predictor::default(),
            wait_count: 0,
            program: gcode_program,
        }
    }

    // TODO: implement better timing info

    fn load_gcode(gcode_path: &Option<PathBuf>) -> Option<PlotterProgram> {
        if gcode_path.is_none() {
            return None;
        }
        let gcode_file = PlotterProgram::read_gcode_file(gcode_path.as_ref().unwrap());
        match gcode_file {
            Err(msg) => {
                error!("{msg}");
                error!("Invalid gcode program");
                None
            }
            Ok(gcode_file) => {
                info!("read: {}", gcode_path.as_ref().unwrap().to_str().unwrap());
                info!("{}", gcode_file);
                Some(gcode_file)
            }
        }
    }

    fn get_scalar_from_user() -> Result<f64, &'static str> {
        let mut input = String::new();
        if let Err(error) = io::stdin().read_line(&mut input) {
            log::error!("error: {error}");
            return Err("Failed to read from standard in");
        }

        let input = input.trim();
        let side = input.parse::<f64>();
        if side.is_err() {
            return Err("Could not parse");
        }
        Ok(side.unwrap())
    }
    fn get_position_from_user() -> Result<PositionMM, &'static str> {
        let mut input = String::new();
        if let Err(error) = io::stdin().read_line(&mut input) {
            error!("{error}");
            return Err("stdin: read_line failed");
        }
        let xy = {
            let input = input.trim();
            let Some(xy_s) = input.split_once(',') else {
                return Err("Did not get expected format");
            };
            let xy_s = [xy_s.0, xy_s.1];
            info!("got {}, {}", xy_s[0], xy_s[1]);
            let mut xy_f: [f64; 2] = [0.0, 0.0];
            for (s, f) in xy_s.iter().zip(xy_f.iter_mut()) {
                if let Ok(pf) = s.parse::<f64>() {
                    *f = pf;
                } else {
                    error!("Failed to parse \"{}\"", s);
                    return Err("Failed to parse");
                }
            }
            xy_f
        };
        Ok(PositionMM::new(xy))
    }

    fn get_char_from_user() -> Result<char, &'static str> {
        let mut input = String::new();
        if let Err(error) = io::stdin().read_line(&mut input) {
            log::error!("error: {error}");
            return Err("read line error");
        }

        let input = input.trim();
        let first_char = input.chars().next();
        if first_char.is_none() {
            return Err("Nothing entered");
        }
        let first_char = first_char.unwrap().to_lowercase().next();
        if first_char.is_none() {
            return Err("Nothing entered for first char");
        }
        Ok(first_char.unwrap())
    }

    fn set_mode_from_user(&mut self) {
        println!("What should we do? (M)ove, (C)enter program, sc(A)le program, (R)un gcode, l(O)ad pattern, set paper (L)imits, or set (P)osition");
        let first_char = Controller::get_char_from_user();
        if first_char.is_err() {
            return;
        }
        self.mode = match first_char.unwrap() {
            'm' => ControllerMode::MoveTo,
            'o' => ControllerMode::LoadPattern,
            'p' => ControllerMode::QueryPosition,
            'r' => ControllerMode::InitProgram,
            'c' => ControllerMode::CenterProgram,
            'a' => ControllerMode::ScaleProgram,
            'l' => ControllerMode::QueryPaper,
            _ => {
                println!("Unknown mode.");
                ControllerMode::Ask
            }
        };
    }
    fn set_current_position_from_user(&mut self) -> Result<(), &'static str> {
        println!("What's the current position in mm? provide \"x,y\"");
        let mm = Controller::get_position_from_user()?;
        self.current_position = Position::from_mm(mm, &self.physical);
        self.current_position_initialized = true;
        info!("position set to {}", self.current_position);
        Ok(())
    }
    fn set_paper_limits_from_user(&mut self) -> Result<(), &'static str> {
        println!("Paper X min,max?");
        let x_limit = Controller::get_position_from_user()?.into();
        println!("Paper Y min,max?");
        let mut y_limit = Controller::get_position_from_user()?.into();
        self.physical.adjust_paper_y_limit(&mut y_limit);
        self.paper_limits = Some([x_limit, y_limit]);
        Ok(())
    }
    fn get_jog_from_user(&mut self) -> Result<PositionMM, ()> {
        println!("Where to? provide \"x,y\"");
        for _ in 0..1 {
            if let Ok(mm) = Controller::get_position_from_user() {
                return Ok(mm);
            }
        }
        Err(())
    }
    /// Initialize move to new location. Set up s-curve and change status.
    fn init_move(&mut self, mm: &PositionMM) {
        if self.current_position.very_close_to(mm, &self.physical) {
            self.move_status = MoveStatus::Stopped;
            return;
        }
        // init s-curve
        self.s_curve = self.solver.solve_curve(self.current_position.into(), *mm);
        info!("s-curve {}", self.s_curve);
        self.predictor = Predictor::new();
        self.move_status = MoveStatus::Moving;
        self.wait_count = 0;
    }
    /// Move current position in steps to (x, y)
    fn update_move(&mut self) {
        self.move_status = self.s_curve.get_move_status();
        if self.move_status == MoveStatus::Stopped {
            info!("new position: {}", self.current_position);
            return;
        }
        let desired = self.s_curve.get_desired(&self.solver, &self.physical);
        match self.predictor.predict(&self.current_position, &desired) {
            Prediction::Wait(_duration) => {
                self.wait_count += 1;
                // thread::sleep(duration);
            }
            Prediction::MoveMotors(instructions) => {
                // print!("{},", self.wait_count);
                self.wait_count = 0;
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
        // info!("new position {}", self.current_position);
    }
    fn move_to(&mut self) {
        if !self.current_position_initialized {
            let _ = self.set_current_position_from_user();
            return;
        }
        let new_position = self.get_jog_from_user();
        if new_position.is_err() {
            return;
        }
        let new_position = new_position.unwrap();
        info!("move from {} to {}", self.current_position, new_position);
        self.init_move(&new_position);
        loop {
            match self.move_status {
                MoveStatus::Stopped => {
                    break;
                }
                MoveStatus::Moving => {
                    self.update_move();
                }
            }
        }
    }

    fn create_square_pattern(&self) -> Result<PlotterProgram, &'static str> {
        println!("How long should square sides be?");
        let square_side_length = Controller::get_scalar_from_user()?;
        square(&self.current_position.into(), &square_side_length)
    }

    fn create_star_pattern(&self) -> Result<PlotterProgram, &'static str> {
        println!("How long should star lines be?");
        let size = Controller::get_scalar_from_user()?;
        star(&self.current_position.into(), &size)
    }

    fn create_wave_pattern(&self) -> Result<PlotterProgram, &'static str> {
        println!("Spacing?");
        let spacing = Controller::get_scalar_from_user()?;
        println!("Length?");
        let length = Controller::get_scalar_from_user()?;
        println!("Amplitude?");
        let amplitude = Controller::get_scalar_from_user()?;
        println!("Period?");
        let period = Controller::get_scalar_from_user()?;
        wave(
            &self.current_position.into(),
            &spacing,
            &length,
            &amplitude,
            &period,
        )
    }

    fn create_spiralgraph_pattern(&self) -> Result<PlotterProgram, &'static str> {
        println!("Radius?");
        let radius = Controller::get_scalar_from_user()?;
        spiralgraph(&self.current_position.into(), &radius)
    }

    fn create_heartwave_pattern(&self) -> Result<PlotterProgram, &'static str> {
        println!("Size?");
        let size = Controller::get_scalar_from_user()?;
        heart_wave(&self.current_position.into(), &size)
    }

    fn load_pattern(&mut self) -> Result<(), &'static str> {
        println!("(S)quare, s(T)ar, (W)ave, spiral(G)raph, (H)eartwave?");
        let pattern = match Controller::get_char_from_user()? {
            's' => self.create_square_pattern(),
            't' => self.create_star_pattern(),
            'w' => self.create_wave_pattern(),
            'g' => self.create_spiralgraph_pattern(),
            'h' => self.create_heartwave_pattern(),
            x => {
                error!("Got char {x}");
                return Err("Got unknown option");
            }
        }?;
        self.program = Some(pattern);
        Ok(())
    }

    fn run_instruction(&mut self, instruction: &PlotterInstruction) {
        match instruction {
            PlotterInstruction::Move(new_position) => {
                self.init_move(new_position);
                loop {
                    match self.move_status {
                        MoveStatus::Stopped => {
                            break;
                        }
                        MoveStatus::Moving => {
                            self.update_move();
                        }
                    }
                }
            }
            PlotterInstruction::PenUp => {
                println!("Remove pen and hit enter");
                let _ = Controller::get_char_from_user();
            }
            PlotterInstruction::PenDown => {
                println!("Insert pen and hit enter");
                let _ = Controller::get_char_from_user();
            }
            PlotterInstruction::Comment(c) => {
                info!("comment: {c}");
            }
            PlotterInstruction::NoOp => (),
        }
    }

    fn get_axis_limit_from_user() -> Result<AxisLimit, &'static str> {
        Controller::get_position_from_user().map(AxisLimit::from)
    }
    fn center_program(&mut self) -> Result<(), &'static str> {
        if self.program.is_none() {
            return Err("No program loaded!");
        }
        println!("Center to paper limits? (y/n)");
        match Controller::get_char_from_user()? {
            'y' => match self.paper_limits.as_ref() {
                Some([x_limits, y_limits]) => {
                    let prog = &mut self.program.as_mut().unwrap();
                    prog.center_keep_aspect(x_limits, y_limits)?;
                    Ok(())
                }
                None => Err("Paper limits not set"),
            },
            'n' => {
                println!("What should the x limits be? (val,val)");
                let x_limits: AxisLimit = Controller::get_axis_limit_from_user()?;
                println!("What should the y limits be? (val,val)");
                let y_limits: AxisLimit = Controller::get_axis_limit_from_user()?;
                let prog = &mut self.program.as_mut().unwrap();
                prog.center_keep_aspect(&x_limits, &y_limits)?;
                Ok(())
            }
            x => {
                error!("got unsupported char {x}");
                Err("Got unsupported char")
            }
        }
    }
    fn scale_program(&mut self) -> Result<(), &'static str> {
        if self.program.is_none() {
            return Err("No program loaded!");
        }
        println!("What should the x limits be? (val,val)");
        let x_limits: AxisLimit = Controller::get_axis_limit_from_user()?;
        println!("What should the y limits be? (val,val)");
        let y_limits: AxisLimit = Controller::get_axis_limit_from_user()?;
        println!("Preserve aspect Ratio? (y,n)");
        let reply = Controller::get_char_from_user()?;
        match reply {
            'y' => {
                let prog = &mut self.program.as_mut().unwrap();
                prog.scale_axis(&x_limits, &Axis::X)?;
                prog.scale_axis(&y_limits, &Axis::Y)?;
                Ok(())
            }
            'n' => {
                let prog = &mut self.program.as_mut().unwrap();
                prog.scale_keep_aspect(&x_limits, &y_limits)?;
                Ok(())
            }

            x => {
                error!("got {x}");
                Err("got unexpected char")
            }
        }
    }

    pub fn init_program(&mut self) -> Result<(), &'static str> {
        match self.program.as_mut() {
            Some(ref mut program) => {
                match &self.paper_limits {
                    Some(paper_limits) => {
                        if !program.within_limits(paper_limits) {
                            return Err("Program not within paper limits");
                        }
                    }
                    None => return Err("Set paper limits first"),
                }
                program.reset();
                Ok(())
            }
            None => Err("No Program Loaded"),
        }
    }

    pub fn update(&mut self) {
        match self.mode {
            ControllerMode::Ask => {
                self.set_mode_from_user();
            }
            ControllerMode::MoveTo => {
                self.move_to();
                self.mode = ControllerMode::Ask;
            }
            ControllerMode::QueryPaper => match self.set_paper_limits_from_user() {
                Ok(_) => {
                    self.mode = ControllerMode::Ask;
                }
                Err(msg) => {
                    error!("{msg}");
                }
            },
            ControllerMode::QueryPosition => match self.set_current_position_from_user() {
                Ok(_) => {
                    self.mode = ControllerMode::Ask;
                }
                Err(msg) => {
                    error!("{msg}");
                }
            },
            ControllerMode::LoadPattern => {
                match self.load_pattern() {
                    Ok(_) => {
                        info!("Pattern loaded");
                    }
                    Err(msg) => {
                        error!("{msg}");
                    }
                }
                self.mode = ControllerMode::Ask;
            }
            ControllerMode::InitProgram => match self.init_program() {
                Ok(_) => {
                    self.mode = ControllerMode::RunProgram;
                }
                Err(msg) => {
                    error!("{msg}");
                    self.mode = ControllerMode::Ask;
                }
            },
            ControllerMode::RunProgram => match self.program.as_mut() {
                Some(program) => {
                    info!(
                        "Run instruction {}/{}",
                        program.current_position(),
                        program.len()
                    );
                    match program.next() {
                        Some(instruction) => self.run_instruction(&instruction),
                        None => {
                            self.mode = ControllerMode::Ask;
                        }
                    }
                }
                None => {
                    info!("No GCode Loaded");
                    self.mode = ControllerMode::Ask;
                }
            },
            ControllerMode::ScaleProgram => match self.scale_program() {
                Ok(_) => {
                    self.mode = ControllerMode::Ask;
                }
                Err(msg) => {
                    error!("{msg}");
                }
            },
            ControllerMode::CenterProgram => match self.center_program() {
                Ok(_) => {
                    self.mode = ControllerMode::Ask;
                }
                Err(msg) => {
                    error!("{msg}");
                }
            },
        }
    }
}
