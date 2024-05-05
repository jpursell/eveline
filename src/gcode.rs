// use anyhow::Result;
use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
    str::FromStr,
};

use async_gcode::{Error, Literal, Parser, RealValue};
use futures::stream;
use futures_executor::block_on;

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
            _ => {panic!("got {val}");}
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
}

impl FromStr for GCode {
    type Err = ();

    /// Parse gcode strings i.e.
    /// (comment)
    /// G21 (comment)
    /// G1 F8000 (set speed)
    /// G0 X15.254 Y82.542
    /// G1 Z0
    fn from_str(s: &str) -> Result<Self, ()> {
        let mut s = s.trim();
        let l_paren = s.find('(');
        if l_paren.is_some() {
            let r_paren = s.find(')');
            if r_paren.is_none() {
                panic!();
                // return Err(())
            }
        }
        todo!()
    }
}

pub struct GCodeFile {
    codes: Vec<GCode>,
}

impl GCodeFile {
    pub fn read_file(path: &Path) {
        // let path = path.to_str().unwrap();
        // for code in parse(path) {
        //     println!("{}", code);
        // }
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
                                    dbg!(&gcode);
                                    codes.push(gcode);
                                    gcode = GCode::new();
                                }
                            }
                        },
                        Err(_) => todo!(),
                    }
                } else {
                    break;
                }
            }
        })

        // for line in reader.lines() {
        //     let line = line?;
        //     println!("{}", line);
        // }
    }
}
