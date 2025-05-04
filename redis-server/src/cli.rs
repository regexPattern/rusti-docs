use std::{env, path::PathBuf};

#[derive(Debug)]
pub struct Args {
    pub config_path: Option<PathBuf>,
}

pub fn parse() -> Args {
    let config_path = env::args().nth(1).map(PathBuf::from);
    Args { config_path }
}
