use clap::Parser;
use log::{info, warn, debug};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::fs::File;

/// A program for making DNS queries on a list of names, then grabbing their request headers.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File with list of names to query
    #[arg(short, long)]
    input_file: PathBuf,
    /// Output JSON file with results
    #[arg(short, long)]
    output_file: PathBuf,
    /// File with list of subnets to check against
    #[arg(short, long)]
    subnet_file: PathBuf,
}

fn main() -> Result<()> {
    env_logger::init();
    info!("Parsing command-line arguments");
    let args = Args::parse();

    debug!("`&args.input_file`: {:?}", &args.input_file);
    debug!("`&args.output_file`: {:?}", &args.output_file);

    let mut sites = lookup_sorter::build_sites(&args.input_file)?;

    let subnets = lookup_sorter::build_subnets(&args.subnet_file)?;
    
    // Before going through the work of making the DNS query,
    // make sure that we're able to open the output file for writing.
    info!("Opening `{}` for writing", &args.output_file.display());
    let mut output_file = File::create(&args.output_file).with_context(||
        format!("Failed to create `{}`", &args.output_file.display()))?;

    for site in sites.iter_mut() {
        (site.addrs, site.headers) = lookup_sorter::look_and_connect(site)?;
        match lookup_sorter::bigip_by_header(&site.headers) {
            true => {
                site.bigip = Some("BigIP by HEADERS".to_owned());
            },
            false => {
                site.bigip = None;
            }
        }

        // Quick test
        // Note this will overwrite the previous bigip value.
        // Just using this for testing functionality and will adjust logic
        match lookup_sorter::bigip_by_ip(&site.addrs, &subnets) {
            true => {
                site.bigip = Some("BigIP by IP".to_owned());
            },
            false => {
                site.bigip = None;
            }
        }
    }

    // Write the resulting structure to an output file as JSON.
    info!("Writing Site structs to `{}`", &args.output_file.display());
    serde_json::to_writer_pretty(&mut output_file, &sites).with_context(||
        format!("Failed to write to `{}`", &args.output_file.display()))?;

    Ok(())
}
