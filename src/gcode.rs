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
    pub fn new() -> Self {
        Self::default()
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
}

impl TryFrom<MaybeAxisLimit> for AxisLimit {
    type Error = ();

    fn try_from(value: MaybeAxisLimit) -> Result<Self, Self::Error> {
        match value.val {
            Some(val) => Ok(Self { val }),
            None => Err(()),
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

pub enum PlotterInstruction {
    Move(PositionMM),
    PenUp,
    PenDown,
    Comment(String),
}

impl TryFrom<GCode> for PlotterInstruction {
    type Error = ();

    fn try_from(value: GCode) -> Result<Self, Self::Error> {
        match value.command {
            Some(command) => match command {
                GCommand::Move | GCommand::FastMove => todo!(),
                GCommand::UseMM | GCommand::AbsoluteDistance  | GCommand::AutoHoming => {
                    match value.comment {
                        Some(val) => Ok(PlotterInstruction::Comment(val)),
                        None => Err(())
                    }
                }
            }
            None => {
                match value.comment {
                    Some(val) => Ok(PlotterInstruction::Comment(val)),
                    None => Err(()),
                }
            }
        }
    }
}

pub struct GCodeProgram {
    codes: Vec<GCode>,
    x_limits: AxisLimit,
    y_limits: AxisLimit,
}

impl GCodeProgram {
    fn compute_limits(codes: &Vec<GCode>) -> Result<[AxisLimit; 2], ()> {
        let mut x_limits = MaybeAxisLimit::new();
        let mut y_limits = MaybeAxisLimit::new();
        for code in codes.iter() {
            code.update_limits(&mut x_limits, &mut y_limits);
        }
        let x_limits = AxisLimit::try_from(x_limits)?;
        let y_limits = AxisLimit::try_from(y_limits)?;
        Ok([x_limits, y_limits])
    }
    fn new(codes: Vec<GCode>) -> Result<Self, ()> {
        let [x_limits, y_limits] = GCodeProgram::compute_limits(&codes)?;
        Ok(GCodeProgram {
            codes,
            x_limits,
            y_limits,
        })
    }
    pub fn scale_axis(&mut self, limit: &AxisLimit, axis: &Axis) {
        let cur_limits = match axis {
            Axis::X => &self.x_limits,
            Axis::Y => &self.y_limits,
        };
        let transformer = cur_limits.transform_to(limit);
        for code in &mut self.codes {
            code.transform(&transformer, axis);
        }
        todo!("update limits and make sure they look right");
    }
    /// Transform code to be in center
    pub fn scale_keep_aspect(&mut self, x_limit: &AxisLimit, y_limit: &AxisLimit) {
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
        for code in &mut self.codes {
            code.transform(&x_transform, &Axis::X);
            code.transform(&y_transform, &Axis::Y);
        }
        todo!("update limits and make sure they look right");
    }
}

impl Display for GCodeProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GCodeProgram: npts: {}, x_limits: {}, y_limits: {}",
            self.codes.len(),
            self.x_limits,
            self.y_limits
        )
    }
}

impl GCodeProgram {
    pub fn read_file(path: &Path) -> Result<GCodeProgram, ()> {
        let file = File::open(path).expect("failed to open file");
        let mut reader = BufReader::new(file);
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).expect("failed to read");
        let mut codes = Vec::new();
        block_on(async {
            let input = stream::iter(buf.into_iter().map(Result::<_, Error>::Ok));
            let mut parser = Parser::new(input);
            let mut gcode = GCode::new();
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
                                    gcode.with_x(v);
                                }
                                'y' => {
                                    gcode.with_y(v);
                                }
                                'z' => {
                                    gcode.with_z(v);
                                }
                                'f' => (),
                                _ => {
                                    panic!("got {c}");
                                }
                            },
                            async_gcode::GCode::Execute => {
                                if gcode != GCode::default() {
                                    codes.push(gcode);
                                    gcode = GCode::new();
                                }
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
        GCodeProgram::new(codes)
    }
}
