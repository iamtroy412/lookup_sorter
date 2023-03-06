use clap::Parser;
use log::{info, warn, debug};
use std::path::PathBuf;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use anyhow::{Context, Result};
use serde::Serialize;
use std::net::IpAddr;
use dns_lookup::lookup_host;

/// A program for making DNS queries on a list of names, then trying to determine if they are on the F5
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File with list of names to query
    #[arg(short, long)]
    file: PathBuf,
}

#[derive(Debug, Serialize)]
struct Site {
    host: String,
    addrs: Vec<IpAddr>,
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
            addrs: Vec::new(),
        });
    }

    debug!("`&sites`: {:?}", &sites);
    debug!("As JSON: {}", serde_json::to_string_pretty(&sites).unwrap());

    for site in sites.iter_mut() {
        debug!("Running DNS lookup on `{}`...", &site.host);
        let addrs = match lookup_host(&site.host) {
            Ok(addrs) => {
                debug!("`&addrs`: {:?}", &addrs);
                site.addrs = addrs;
            },
            Err(e) => warn!("`&site.host`: {}", e),
        };
    }

    let mut output_file = File::create("output.txt").with_context(||
        format!("Failed to create `{}`", "output.txt"))?;
    
    serde_json::to_writer_pretty(&mut output_file, &sites).with_context(||
        format!("Failed to write to `{}`", "output.txt"))?;

    Ok(())
}
