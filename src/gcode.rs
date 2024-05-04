use anyhow::Result;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path, str::FromStr,
};

enum GCommand {
    FastMove,
    Move,
}

struct GCode {
    command: Option<GCommand>,
    comment: Option<String>,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
    f: Option<f64>,
}

impl FromStr for GCode {
    type Err = ();

    /// Parse gcode strings i.e. 
    /// (comment)
    /// G21 (comment)
    /// G1 F8000 (set speed)
    /// G0 X15.254 Y82.542
    /// G1 Z0
    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        let mut s = s.trim();
        let l_paren = s.find('(');
        if l_paren.is_some() {
            let r_paren = s.find(')');
            if r_paren.is_none() {
                return Err(())
            }



        }
        todo!()
    }
}

pub struct GCodeFile {
    codes: Vec<GCode>,
}

impl GCodeFile {
    pub fn read_file(path: &Path) -> Result<()> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            println!("{}", line);
        }
        Ok(())
    }
}
