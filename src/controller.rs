use std::{io, path::PathBuf, thread, time::Duration};

use log::{error, info};

use crate::{
    draw::{heart_wave, spiralgraph, square, star, wave, Pattern},
    gcode::{Axis, AxisLimit, PlotterInstruction, PlotterProgram},
    motor::{Motor, Side, StepInstruction},
    physical::Physical,
    position::{Position, PositionMM, PositionStep},
    predictor::{Prediction, Predictor},
    scurve::{SCurve, SCurveSolver},
};

enum ControllerMode {
    Ask,
    Step,
    SmallMove,
    MoveTo,
    QueryPaper,
    QueryPosition,
    Moving,
    InitProgram,
    RunProgram,
    Spiralgraph,
    HeartWave,
    Square,
    Star,
    Wave,
    ScaleGCode,
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
    gcode_path: Option<PathBuf>,
    program: Option<PlotterProgram>,
}

impl Controller {
    pub fn new(gcode_path: Option<PathBuf>) -> Controller {
        let motors = [Side::Left, Side::Right].map(|s| Motor::new(s));
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
            gcode_path,
            program: gcode_program,
        }
    }

    // TODO: implement set_gcode_start_point
    // TODO: implement draw_gcode_instruction
    // TODO: change run_gcode to use draw_code_instruction and move to next until end
    // TODO: implement len_lift pause
    // TODO: implement better timing info

    fn load_gcode(gcode_path: &Option<PathBuf>) -> Option<PlotterProgram> {
        if gcode_path.is_none() {
            return None;
        }
        let gcode_file = PlotterProgram::read_gcode_file(gcode_path.as_ref().unwrap());
        match gcode_file {
            Err(_) => {
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

    fn get_scalar_from_user() -> Result<f64, ()> {
        let mut input = String::new();
        if let Err(error) = io::stdin().read_line(&mut input) {
            log::error!("error: {error}");
            return Err(());
        }

        let input = input.trim();
        let side = input.parse::<f64>();
        if side.is_err() {
            log::error!("Could not parse");
            return Err(());
        }
        Ok(side.unwrap())
    }
    fn get_position_from_user() -> Result<PositionMM, ()> {
        let mut input = String::new();
        if let Err(error) = io::stdin().read_line(&mut input) {
            log::error!("error: {error}");
            return Err(());
        }
        let xy = {
            let input = input.trim();
            let Some(xy_s) = input.split_once(",") else {
                log::error!("Did not get expected format");
                return Err(());
            };
            let xy_s = [xy_s.0, xy_s.1];
            info!("got {}, {}", xy_s[0], xy_s[1]);
            let mut xy_f: [f64; 2] = [0.0, 0.0];
            for (s, f) in xy_s.iter().zip(xy_f.iter_mut()) {
                if let Ok(pf) = s.parse::<f64>() {
                    *f = pf;
                } else {
                    log::error!("Failed to parse \"{}\"", s);
                    return Err(());
                }
            }
            xy_f
        };
        Ok(PositionMM::new(xy))
    }

    fn get_char_from_user() -> Result<char, ()> {
        let mut input = String::new();
        if let Err(error) = io::stdin().read_line(&mut input) {
            log::error!("error: {error}");
            return Err(());
        }

        let input = input.trim();
        let first_char = input.chars().next();
        if first_char.is_none() {
            println!("Nothing entered");
            return Err(());
        }
        let first_char = first_char.unwrap().to_lowercase().next();
        if first_char.is_none() {
            return Err(());
        }
        Ok(first_char.unwrap())
    }

    fn set_mode_from_user(&mut self) {
        println!("What should we do? (M)ove, (R)un gcode, (S)quare, s(T)ar, (W)ave, spiral(G)raph, (H)eartwave, set paper (L)imits, or set (P)osition");
        let first_char = Controller::get_char_from_user();
        if first_char.is_err() {
            return;
        }
        self.mode = match first_char.unwrap() {
            'm' => ControllerMode::MoveTo,
            's' => ControllerMode::Square,
            't' => ControllerMode::Star,
            'w' => ControllerMode::Wave,
            'h' => ControllerMode::HeartWave,
            'g' => ControllerMode::Spiralgraph,
            'p' => ControllerMode::QueryPosition,
            'r' => ControllerMode::InitProgram,
            'c' => ControllerMode::ScaleGCode,
            'l' => ControllerMode::QueryPaper,
            _ => {
                println!("Unknown mode.");
                ControllerMode::Ask
            }
        };
    }
    fn set_current_position_from_user(&mut self) -> Result<(), ()> {
        println!("What's the current position in mm? provide \"x,y\"");
        for _ in 0..1 {
            if let Ok(mm) = Controller::get_position_from_user() {
                self.current_position = Position::from_mm(mm, &self.physical);
                self.current_position_initialized = true;
                info!("position set to {}", self.current_position);
                return Ok(());
            }
        }
        Err(())
    }
    fn set_paper_limits_from_user(&mut self) -> Result<(), ()> {
        println!("Paper X min,max?");
        let x_limit = Controller::get_position_from_user()?.into();
        println!("Paper Y min,max?");
        let y_limit = Controller::get_position_from_user()?.into();
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
    fn step(&mut self) {
        if !self.current_position_initialized {
            let _ = self.set_current_position_from_user();
            return;
        }
        println!("move L/H. current {}", self.current_position);
        for _ in 0..1000 {
            self.implement_step_instructions([StepInstruction::Hold, StepInstruction::StepShorter]);
            thread::sleep(Duration::from_secs_f64(0.005));
        }
        thread::sleep(Duration::from_secs_f64(1.0));
        println!("move S/H. current {}", self.current_position);
        for _ in 0..1000 {
            self.implement_step_instructions([StepInstruction::Hold, StepInstruction::StepLonger]);
            thread::sleep(Duration::from_secs_f64(0.005));
        }
        thread::sleep(Duration::from_secs_f64(2.0));
    }
    fn small_move(&mut self) {
        if !self.current_position_initialized {
            let _ = self.set_current_position_from_user();
            return;
        }
        let amount = 32.0;
        let direction = [0.0, 1.0];
        let new_position: PositionMM = self.current_position.offset(&amount, &direction);
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
        let direction = [0.0, -1.0];
        let new_position: PositionMM = self.current_position.offset(&amount, &direction);
        info!(
            "move back to {} from {}",
            new_position, self.current_position
        );
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

    fn create_square_pattern(&self) -> Result<Vec<PositionMM>, ()> {
        println!("How long should square sides be?");
        let square_side_length = Controller::get_scalar_from_user();
        if square_side_length.is_err() {
            return Err(());
        }
        let coords = square(&self.current_position.into(), &square_side_length.unwrap());
        Ok(coords)
    }

    fn create_star_pattern(&self) -> Result<Vec<PositionMM>, ()> {
        println!("How long should star lines be?");
        let size = Controller::get_scalar_from_user();
        if size.is_err() {
            return Err(());
        }
        let coords = star(&self.current_position.into(), &size.unwrap());
        Ok(coords)
    }

    fn create_wave_pattern(&self) -> Result<Vec<PositionMM>, ()> {
        println!("Spacing?");
        let spacing = Controller::get_scalar_from_user();
        if spacing.is_err() {
            return Err(());
        }
        println!("Length?");
        let length = Controller::get_scalar_from_user();
        if length.is_err() {
            return Err(());
        }
        println!("Amplitude?");
        let amplitude = Controller::get_scalar_from_user();
        if amplitude.is_err() {
            return Err(());
        }
        println!("Period?");
        let period = Controller::get_scalar_from_user();
        if period.is_err() {
            return Err(());
        }
        let coords = wave(
            &self.current_position.into(),
            &spacing.unwrap(),
            &length.unwrap(),
            &amplitude.unwrap(),
            &period.unwrap(),
        );
        Ok(coords)
    }

    fn create_spiralgraph_pattern(&self) -> Result<Vec<PositionMM>, ()> {
        println!("Radius?");
        let radius = Controller::get_scalar_from_user();
        if radius.is_err() {
            return Err(());
        }
        let coords = spiralgraph(&self.current_position.into(), &radius.unwrap());
        Ok(coords)
    }

    fn create_heartwave_pattern(&self) -> Result<Vec<PositionMM>, ()> {
        println!("Size?");
        let size = Controller::get_scalar_from_user();
        if size.is_err() {
            return Err(());
        }
        let coords = heart_wave(&self.current_position.into(), &size.unwrap());
        Ok(coords)
    }

    fn draw_pattern(&mut self, pattern: Pattern) {
        let pattern = match pattern {
            Pattern::Square => self.create_square_pattern(),
            Pattern::Star => self.create_star_pattern(),
            Pattern::Wave => self.create_wave_pattern(),
            Pattern::Spiralgraph => self.create_spiralgraph_pattern(),
            Pattern::HeartWave => self.create_heartwave_pattern(),
        };
        if pattern.is_err() {
            return;
        }
        let pattern = pattern.unwrap();
        for new_position in &pattern {
            if !self.physical.in_bounds(new_position) {
                println!("Point outside of bounds: {new_position}");
                return;
            }
        }

        for new_position in &pattern {
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
    }

    fn run_instruction(&mut self, instruction: &PlotterInstruction) {
        match instruction {
            PlotterInstruction::Move(new_position) => {
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

    fn get_axis_limit_from_user() -> Result<AxisLimit, ()> {
        Controller::get_position_from_user().map(|p| AxisLimit::from(p))
    }
    fn scale_gcode(&mut self) {
        if self.program.is_none() {
            println!("No program loaded!");
            return;
        }
        println!("What should the x limits be? (val,val)");
        let x_limits: Result<AxisLimit, ()> = Controller::get_axis_limit_from_user();
        if x_limits.is_err() {
            return;
        }
        println!("What should the y limits be? (val,val)");
        let y_limits: Result<AxisLimit, ()> = Controller::get_axis_limit_from_user();
        if y_limits.is_err() {
            return;
        }
        println!("Preserve aspect Ratio? (y,n)");
        let reply = Controller::get_char_from_user();
        if reply.is_err() {
            return;
        }
        match reply.unwrap() {
            'y' => {
                let prog = &mut self.program.as_mut().unwrap();
                prog.scale_axis(&x_limits.unwrap(), &Axis::X);
                prog.scale_axis(&y_limits.unwrap(), &Axis::Y);
            }
            'n' => {
                let prog = &mut self.program.as_mut().unwrap();
                prog.scale_keep_aspect(&x_limits.unwrap(), &y_limits.unwrap());
            }

            x => {
                error!("got {x}");
                return;
            }
        }
    }

    pub fn update(&mut self) {
        match self.mode {
            ControllerMode::Ask => {
                self.set_mode_from_user();
            }
            ControllerMode::Step => {
                self.step();
            }
            ControllerMode::SmallMove => {
                self.small_move();
            }
            ControllerMode::MoveTo => {
                self.move_to();
                self.mode = ControllerMode::Ask;
            }
            ControllerMode::QueryPaper => {
                if let Ok(_) = self.set_paper_limits_from_user() {
                    self.mode = ControllerMode::Ask;
                }
            }
            ControllerMode::QueryPosition => {
                if let Ok(_) = self.set_current_position_from_user() {
                    self.mode = ControllerMode::Ask;
                }
            }
            ControllerMode::Moving => {
                self.update_move();
                if self.move_status == MoveStatus::Stopped {
                    self.mode = ControllerMode::QueryPosition;
                }
            }
            ControllerMode::Square => {
                self.draw_pattern(Pattern::Square);
                self.mode = ControllerMode::Ask;
            }
            ControllerMode::Star => {
                self.draw_pattern(Pattern::Star);
                self.mode = ControllerMode::Ask;
            }
            ControllerMode::Wave => {
                self.draw_pattern(Pattern::Wave);
                self.mode = ControllerMode::Ask;
            }
            ControllerMode::Spiralgraph => {
                self.draw_pattern(Pattern::Spiralgraph);
                self.mode = ControllerMode::Ask;
            }
            ControllerMode::HeartWave => {
                self.draw_pattern(Pattern::HeartWave);
                self.mode = ControllerMode::Ask;
            }
            ControllerMode::InitProgram => match self.program.as_mut() {
                Some(ref mut program) => {
                    program.reset();
                    todo!("check gcode bounds is in paper limits (also make sure paper limits set)");
                    self.mode = ControllerMode::RunProgram;
                }
                None => {
                    info!("No Program Loaded");
                    self.mode = ControllerMode::Ask;
                }
            },
            ControllerMode::RunProgram => {
                if self.program.is_none() {
                    info!("No GCode Loaded");
                    self.mode = ControllerMode::Ask;
                    return;
                }
                match self.program.as_mut().unwrap().next() {
                    Some(instruction) => self.run_instruction(&instruction),
                    None => {
                        self.mode = ControllerMode::Ask;
                    }
                }
            }
            ControllerMode::ScaleGCode => {
                self.scale_gcode();
            }
        }
    }
}
