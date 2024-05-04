use anyhow::Result;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

struct GCode {}

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
