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

#[derive(Default)]
struct AxisLimit {
    val: Option<[f64; 2]>,
}

impl AxisLimit {
    fn new() -> Self {
        Self::default()
    }

    fn update(&mut self, val: &f64) {
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

impl Display for AxisLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.val {
            Some([val_min, val_max]) => {
                write!(f, "AxisLimit: [{}, {}]", val_min, val_max)
            }
            None => {
                write!(f, "AxisLimit: None")
            }
        }
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
}

impl GCode {
    fn new() -> Self {
        Self::default()
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
    fn update_limits(&self, x_limits: &mut AxisLimit, y_limits: &mut AxisLimit) {
        if self.x.is_some() {
            x_limits.update(&self.x.unwrap());
        }
        if self.y.is_some() {
            y_limits.update(&self.y.unwrap());
        }
    }
}

pub struct GCodeProgram {
    codes: Vec<GCode>,
    x_limits: AxisLimit,
    y_limits: AxisLimit,
}

impl GCodeProgram {
    fn new(codes: Vec<GCode>) -> Self {
        let mut x_limits = AxisLimit::new();
        let mut y_limits = AxisLimit::new();
        for code in codes.iter() {
            code.update_limits(&mut x_limits, &mut y_limits);
        }
        GCodeProgram {
            codes,
            x_limits,
            y_limits,
        }
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
    pub fn read_file(path: &Path) -> Self {
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
