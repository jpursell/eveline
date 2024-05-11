// use anyhow::Result;
use std::{
    fmt::Display,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use async_gcode::{Error, Literal, Parser, RealValue};
use futures::stream;
use futures_executor::block_on;

use crate::position::PositionMM;

struct AxisTransformer {
    scale: f64,
    offset: f64,
}

impl AxisTransformer {
    fn new(scale: f64, offset: f64) -> Self {
        AxisTransformer { scale, offset }
    }
    fn transform(&self, val: &mut f64) {
        *val *= self.scale;
        *val += self.offset;
    }
}
struct AxisTransposer {
    offset: f64,
}

impl AxisTransposer {
    fn new(offset: f64) -> Self {
        Self { offset }
    }
    fn transpose(&self, val: &mut f64) {
        *val += self.offset;
    }
}

#[derive(Default)]
pub struct MaybeAxisLimit {
    val: Option<[f64; 2]>,
}

impl MaybeAxisLimit {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_none(&self) -> bool {
        self.val.is_none()
    }

    pub fn update(&mut self, val: &f64) {
        match &mut self.val {
            Some([val_min, val_max]) => {
                *val_min = val_min.min(*val);
                *val_max = val_max.max(*val);
            }
            None => {
                self.val = Some([*val; 2]);
            }
        }
    }
}

impl From<PositionMM> for MaybeAxisLimit {
    fn from(value: PositionMM) -> Self {
        let mut limit = MaybeAxisLimit::new();
        limit.update(value.x());
        limit.update(value.y());
        limit
    }
}

impl Display for MaybeAxisLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.val {
            Some([val_min, val_max]) => {
                write!(f, "MaybeAxisLimit: [{}, {}]", val_min, val_max)
            }
            None => {
                write!(f, "MaybeAxisLimit: None")
            }
        }
    }
}

#[derive(Default)]
pub struct AxisLimit {
    val: [f64; 2],
}

impl AxisLimit {
    pub fn new(val: [f64; 2]) -> Self {
        AxisLimit { val }
    }

    pub fn is_close_to(&self, other: &AxisLimit) -> bool {
        is_close::default().all_close(self.val, other.val)
    }

    pub fn is_inside_of(&self, other: &AxisLimit) -> bool {
        self.is_close_to(other) || self.val[0] > other.val[0] && self.val[1] < other.val[1]
    }

    pub fn offset(&mut self, val: &f64) {
        for x in &mut self.val{
            *x += val;
        }
    }

    fn transform_to(&self, other: &AxisLimit) -> AxisTransformer {
        let cur_val = &self.val;
        let other_val = &other.val;
        let cur_scale = cur_val[1] - cur_val[0];
        let other_scale = other_val[1] - other_val[0];
        // if cur limit is [1,2] and other is [3,5]
        // then scale becomes 2
        let scale = other_scale / cur_scale;
        // and offset is 1
        let offset = other_val[0] - cur_val[0] * scale;
        // multiply by scale first and then add offset
        AxisTransformer::new(scale, offset)
    }
    fn center_to(&self, other: &AxisLimit) -> Result<AxisTransposer, &'static str> {
        let cur_val = &self.val;
        let other_val = &other.val;
        let cur_scale = cur_val[1] - cur_val[0];
        let other_scale = other_val[1] - other_val[0];
        if cur_scale > other_scale {
            return Err("Too large to center")
        }
        let cur_center = (cur_val[1] + cur_val[0]) / 2.0;
        let other_center = (other_val[1] + other_val[0]) / 2.0;
        let offset = other_center - cur_center;
        Ok(AxisTransposer::new(offset))
    }
}

impl TryFrom<MaybeAxisLimit> for AxisLimit {
    type Error = &'static str;

    fn try_from(value: MaybeAxisLimit) -> Result<Self, Self::Error> {
        match value.val {
            Some(val) => Ok(Self { val }),
            None => Err("No value for axis limit"),
        }
    }
}

impl From<PositionMM> for AxisLimit {
    fn from(value: PositionMM) -> Self {
        let mut limit = MaybeAxisLimit::new();
        limit.update(value.x());
        limit.update(value.y());
        limit.try_into().unwrap()
    }
}

impl Display for AxisLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AxisLimit: [{}, {}]", self.val[0], self.val[1])
    }
}

#[derive(Debug, PartialEq)]
enum GCommand {
    FastMove,
    Move,
    UseMM,
    AbsoluteDistance,
    AutoHoming,
}

#[derive(Debug, Default, PartialEq)]
struct GCode {
    command: Option<GCommand>,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
    f: Option<f64>,
    comment: Option<String>,
}

impl GCode {
    fn new() -> Self {
        Self::default()
    }
    pub fn transform(&mut self, transform: &AxisTransformer, axis: &Axis) {
        let val = match axis {
            Axis::X => &mut self.x,
            Axis::Y => &mut self.y,
        };
        match val {
            Some(inner_val) => {
                transform.transform(inner_val);
            }
            None => (),
        }
    }
    fn with_g(&mut self, val: f64) {
        self.command = match val {
            0.0 => Some(GCommand::FastMove),
            1.0 => Some(GCommand::Move),
            21.0 => Some(GCommand::UseMM),
            90.0 => Some(GCommand::AbsoluteDistance),
            28.0 => Some(GCommand::AutoHoming),
            _ => {
                panic!("got {val}");
            }
        };
    }
    fn with_x(&mut self, val: f64) {
        self.x = Some(val);
    }
    fn with_y(&mut self, val: f64) {
        self.y = Some(val);
    }
    fn with_z(&mut self, val: f64) {
        self.z = Some(val);
    }
    fn with_f(&mut self, val: f64) {
        self.f = Some(val);
    }
    fn with_comment(&mut self, val: String) {
        self.comment = Some(val);
    }
    fn update_limits(&self, x_limits: &mut MaybeAxisLimit, y_limits: &mut MaybeAxisLimit) {
        if self.x.is_some() {
            x_limits.update(&self.x.unwrap());
        }
        if self.y.is_some() {
            y_limits.update(&self.y.unwrap());
        }
    }
}

pub enum Axis {
    X,
    Y,
}

#[derive(Clone)]
pub enum PlotterInstruction {
    Move(PositionMM),
    PenUp,
    PenDown,
    Comment(String),
    NoOp,
}

impl PlotterInstruction {
    fn update_limits(&self, x_limits: &mut MaybeAxisLimit, y_limits: &mut MaybeAxisLimit) {
        match self {
            PlotterInstruction::Move(pos) => {
                x_limits.update(pos.x());
                y_limits.update(pos.y());
            }
            _ => (),
        }
    }
    fn transform(&mut self, transform: &AxisTransformer, axis: &Axis) {
        match self {
            PlotterInstruction::Move(pos) => {
                let inner_val: &mut f64 = match axis {
                    Axis::X => pos.x_mut(),
                    Axis::Y => pos.y_mut(),
                };
                transform.transform(inner_val);
            }
            _ => (),
        }
    }
    fn transpose(&mut self, transposer: &AxisTransposer, axis: &Axis) {
        match self {
            PlotterInstruction::Move(pos) => {
                let inner_val: &mut f64 = match axis {
                    Axis::X => pos.x_mut(),
                    Axis::Y => pos.y_mut(),
                };
                transposer.transpose(inner_val);
            }
            _ => (),
        }
    }
}

impl TryFrom<GCode> for PlotterInstruction {
    type Error = &'static str;

    fn try_from(value: GCode) -> Result<Self, Self::Error> {
        match value.command {
            Some(command) => match command {
                GCommand::Move | GCommand::FastMove => match value.z {
                    Some(z_val) => {
                        if value.x.is_some() | value.y.is_some() {
                            Err("Did not expect a 3D move")
                        } else if z_val < 0.0 {
                            Err("Did not expect negative z value")
                        } else if z_val == 0.0 {
                            Ok(PlotterInstruction::PenDown)
                        } else {
                            Ok(PlotterInstruction::PenUp)
                        }
                    }
                    None => {
                        if value.f.is_some() && value.x.is_none() && value.y.is_none() {
                            let feed = value.f.unwrap();
                            Ok(PlotterInstruction::Comment(String::from(format!(
                                "feed {feed}"
                            ))))
                        } else if value.x.is_none() {
                            Err("Move missing X")
                        } else if value.y.is_none() {
                            Err("Move missing Y")
                        } else {
                            Ok(PlotterInstruction::Move(PositionMM::new([
                                value.x.unwrap(),
                                value.y.unwrap(),
                            ])))
                        }
                    }
                },
                GCommand::UseMM => Ok(PlotterInstruction::Comment(String::from("Use mm"))),
                GCommand::AbsoluteDistance => Ok(PlotterInstruction::Comment(String::from(
                    "Absolute distance",
                ))),
                GCommand::AutoHoming => {
                    Ok(PlotterInstruction::Comment(String::from("Auto homing")))
                }
            },
            None => match value.comment {
                Some(val) => Ok(PlotterInstruction::Comment(val)),
                None => Ok(PlotterInstruction::NoOp),
            },
        }
    }
}

pub struct PlotterProgram {
    instructions: Vec<PlotterInstruction>,
    x_limits: AxisLimit,
    y_limits: AxisLimit,
    current_position: usize,
}

impl Iterator for PlotterProgram {
    type Item = PlotterInstruction;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_position >= self.instructions.len() {
            return None;
        }
        let instruction = Some(self.instructions[self.current_position].clone());
        self.current_position += 1;
        instruction
    }
}

impl PlotterProgram {
    fn compute_limits(
        instructions: &Vec<PlotterInstruction>,
    ) -> Result<[AxisLimit; 2], &'static str> {
        let mut x_limits = MaybeAxisLimit::new();
        let mut y_limits = MaybeAxisLimit::new();
        for instruction in instructions.iter() {
            instruction.update_limits(&mut x_limits, &mut y_limits);
        }
        let x_limits = AxisLimit::try_from(x_limits)?;
        let y_limits = AxisLimit::try_from(y_limits)?;
        Ok([x_limits, y_limits])
    }
    fn update_limits(&mut self) -> Result<(), &'static str> {
        let [xlim, ylim] = PlotterProgram::compute_limits(&self.instructions)?;
        self.x_limits = xlim;
        self.y_limits = ylim;
        Ok(())
    }
    fn new(instructions: Vec<PlotterInstruction>) -> Result<Self, &'static str> {
        let [x_limits, y_limits] = PlotterProgram::compute_limits(&instructions)?;
        Ok(PlotterProgram {
            instructions,
            x_limits,
            y_limits,
            current_position: 0,
        })
    }
    pub fn within_limits(&self, limits: &[AxisLimit; 2]) -> bool {
        self.x_limits.is_inside_of(&limits[0]) && self.y_limits.is_inside_of(&limits[1])
    }
    pub fn reset(&mut self) {
        self.current_position = 0;
    }
    pub fn len(&self) -> usize {
        self.instructions.len()
    }
    pub fn current_position(&self) -> usize {
        self.current_position
    }
    fn get_limit(&self, axis: &Axis) -> &AxisLimit {
        match axis {
            Axis::X => &self.x_limits,
            Axis::Y => &self.y_limits,
        }
    }
    pub fn scale_axis(&mut self, limit: &AxisLimit, axis: &Axis) -> Result<(), &'static str> {
        let cur_limits = self.get_limit(axis);
        let transformer = cur_limits.transform_to(limit);
        for instruction in &mut self.instructions {
            instruction.transform(&transformer, axis);
        }
        self.update_limits()
            .expect("Limit calculation failed after scaling");
        let cur_limits = self.get_limit(axis);
        if !cur_limits.is_close_to(limit) {
            Err("Scale failed")
        } else {
            Ok(())
        }
    }
    /// Transform code to be in center
    pub fn center_keep_aspect(
        &mut self,
        x_limit: &AxisLimit,
        y_limit: &AxisLimit,
    ) -> Result<(), &'static str> {
        let x_transpose = self.x_limits.center_to(x_limit)?;
        let y_transpose = self.y_limits.center_to(y_limit)?;
        for instruction in &mut self.instructions {
            instruction.transpose(&x_transpose, &Axis::X);
            instruction.transpose(&y_transpose, &Axis::Y);
        }
        self.update_limits()?;
        if !self.x_limits.is_inside_of(x_limit) {
            Err("No in X bounds")
        } else if !self.y_limits.is_inside_of(y_limit) {
            Err("No in Y bounds")
        } else {
            Ok(())
        }
    }
    /// Transform code to be in center
    pub fn scale_keep_aspect(
        &mut self,
        x_limit: &AxisLimit,
        y_limit: &AxisLimit,
    ) -> Result<(), &'static str> {
        let mut x_transform = self.x_limits.transform_to(x_limit);
        let mut y_transform = self.y_limits.transform_to(y_limit);
        let scale = x_transform.scale.min(y_transform.scale);
        let (adjust, cur, other) = if x_transform.scale > y_transform.scale {
            (&mut x_transform, &self.x_limits, &x_limit)
        } else {
            (&mut y_transform, &self.y_limits, &y_limit)
        };
        adjust.scale = scale;
        let cur_middle = (cur.val[0] + cur.val[1]) / 2.0;
        let other_middle = (other.val[0] + other.val[1]) / 2.0;
        adjust.offset = other_middle - cur_middle * scale;
        for instruction in &mut self.instructions {
            instruction.transform(&x_transform, &Axis::X);
            instruction.transform(&y_transform, &Axis::Y);
        }
        self.update_limits()?;
        if !self.x_limits.is_inside_of(x_limit){
            Err("Not in x")
        } else if !self.y_limits.is_inside_of(y_limit){
            Err("Not in Y")
        } else {
            Ok(())
        }
    }

    pub fn read_gcode_file(path: &Path) -> Result<PlotterProgram, &'static str> {
        let file = File::open(path).expect("failed to open file");
        let mut reader = BufReader::new(file);
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).expect("failed to read");
        let mut codes = Vec::new();
        block_on(async {
            let input = stream::iter(buf.into_iter().map(Result::<_, Error>::Ok));
            let mut parser = Parser::new(input);
            let mut gcode = GCode::new();
            // keep a x/y state to fill out any potential move commands
            // that only have one of these
            let mut pos = [None; 2];
            let mut pos_set = [false; 2];
            loop {
                if let Some(res) = parser.next().await {
                    match res {
                        Ok(gc) => match gc {
                            async_gcode::GCode::BlockDelete => todo!(),
                            async_gcode::GCode::LineNumber(_) => todo!(),
                            async_gcode::GCode::Word(
                                c,
                                RealValue::Literal(Literal::RealNumber(v)),
                            ) => match c {
                                'g' => {
                                    gcode.with_g(v);
                                }
                                'x' => {
                                    pos[0] = Some(v);
                                    pos_set[0] = true;
                                    gcode.with_x(v);
                                }
                                'y' => {
                                    pos[1] = Some(v);
                                    pos_set[1] = true;
                                    gcode.with_y(v);
                                }
                                'z' => {
                                    gcode.with_z(v);
                                }
                                'f' => {
                                    gcode.with_f(v);
                                }
                                _ => {
                                    panic!("got {c}");
                                }
                            },
                            async_gcode::GCode::Execute => {
                                if gcode != GCode::default() {
                                    if pos_set[0] && !pos_set[1] {
                                        gcode.with_y(pos[1].expect(
                                            "Expecting to not start with a single axis move",
                                        ))
                                    }
                                    if pos_set[1] && !pos_set[0] {
                                        gcode.with_x(pos[0].expect(
                                            "Expecting to not start with a single axis move",
                                        ))
                                    }
                                    codes.push(gcode);
                                    gcode = GCode::new();
                                    pos_set = [false; 2];
                                }
                            }
                            async_gcode::GCode::Comment(msg) => {
                                gcode.with_comment(msg);
                            }
                        },
                        Err(e) => {
                            log::error!("Got error: {e:?}");
                            continue;
                        }
                    }
                } else {
                    break;
                }
            }
        });
        let mut instructions = Vec::new();
        for code in codes {
            let instruction = PlotterInstruction::try_from(code)?;
            match instruction {
                PlotterInstruction::NoOp => {
                    continue;
                }
                _ => (),
            }
            instructions.push(instruction)
        }
        let program = PlotterProgram::new(instructions)?;
        Ok(program)
    }
}

impl Display for PlotterProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Program: npts: {}, x_limits: {}, y_limits: {}",
            self.instructions.len(),
            self.x_limits,
            self.y_limits
        )
    }
}
