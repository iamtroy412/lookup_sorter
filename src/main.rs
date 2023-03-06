use clap::Parser;
use log::{info, warn, debug};
use std::path::PathBuf;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::sync::Arc;
use anyhow::{Context, Result, Ok};

/// A program for making DNS queries on a list of names, then trying to determine if they are on the F5
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File with list of names to query
    #[arg(short, long)]
    file: PathBuf,
}

#[derive(Debug)]
struct Site {
    host: String,
}

fn main() -> Result<()> {
    env_logger::init();
    info!("Parsing command-line arguments");
    let args = Args::parse();

    debug!("`&args.file`: {:?}", &args.file);

    info!("Opening `{}` for reading", &args.file.display());
    let file = File::open(&args.file).with_context(||
        format!("Failed to open `{}`", &args.file.display()))?;
    
    let reader = BufReader::new(file);

    let mut sites = Vec::new();

    info!("Reading lines from `{}` into `Site` structs", &args.file.display());
    for line in reader.lines() {
        let line = line.with_context(||
            format!("Failed to read `{}`", &args.file.display()))?;

        debug!("`&line`: {:?}", &line);
        sites.push(Site {
            host: line.trim().to_string(),
        });
    }

    debug!("`&sites`: {:?}", &sites);

    Ok(())
}
