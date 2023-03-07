use clap::Parser;
use log::{info, warn, debug};
use reqwest::header::HeaderMap;
use reqwest::redirect::Policy;
use std::path::PathBuf;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use anyhow::{Context, Result};
use serde::Serialize;
use std::net::IpAddr;
use dns_lookup::lookup_host;
use std::time::Duration;

/// A program for making DNS queries on a list of names, then grabbing their request headers.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File with list of names to query
    #[arg(short, long)]
    input_file: PathBuf,
    // Output JSON file with results
    #[arg(short, long)]
    output_file: PathBuf,
}

#[derive(Debug, Serialize)]
struct Site {
    host: String,
    addrs: Vec<IpAddr>,
    #[serde(with = "http_serde::header_map")]
    headers: HeaderMap,
}

// build_sites returns a vector of Site structs, one for each name in the input file.
fn build_sites(input_path: &PathBuf) -> Result<Vec<Site>, anyhow::Error> {
    info!("Opening `{}` for reading", &input_path.display());
    let file = File::open(input_path).with_context(||
        format!("Failed to open `{}`", &input_path.display()))?;
    
    let reader = BufReader::new(file);

    let mut sites = Vec::new();

    info!("Reading lines from `{}` into `Site` structs", &input_path.display());
    for line in reader.lines() {
        let line = line.with_context(||
            format!("Failed to read `{}`", &input_path.display()))?;

        debug!("`&line`: {:?}", &line);
        sites.push(Site {
            host: line.trim().to_string(),
            addrs: Vec::new(),
            headers: HeaderMap::new(),
        });
    }

    debug!("`&sites`: {:?}", &sites);
    Ok(sites)
}

// look_and_connect takes a Site struct, resolves the IP address of the name, and then
// tries to connect to it and record the headers.
fn look_and_connect(site: &Site) -> Result<(Vec<IpAddr>, HeaderMap), anyhow::Error> {
    let mut addresses = Vec::<IpAddr>::new();
    let mut headers = HeaderMap::new();
    
    info!("Running DNS lookup on `{}`...", &site.host);
    match lookup_host(&site.host) {
        Ok(addrs) => {
            debug!("`&addrs`: {:?}", &addrs);
            addresses = addrs;
            let client = &reqwest::blocking::Client::builder().redirect(Policy::none()).build()?;
            info!("Connecting to `{}`...", &site.host);
            match client.get(format!("http://{}", &site.host)).timeout(Duration::from_secs(3)).send(){
                Ok(resp) => {
                    debug!("`&response.headers`: {:?}", &resp.headers());
                    headers = resp.headers().clone();
                },
                Err(err) => {
                    warn!("Unable to make connection: {:?}", &err);
                }
            };
        },
        Err(e) => warn!("`&site.host`: {} Error: {}", &site.host, e),
    };

    Ok((addresses, headers))

}

fn main() -> Result<()> {
    env_logger::init();
    info!("Parsing command-line arguments");
    let args = Args::parse();

    debug!("`&args.input_file`: {:?}", &args.input_file);
    debug!("`&args.output_file`: {:?}", &args.output_file);

    let mut sites = build_sites(&args.input_file)?;
    
    // Before going through the work of making the DNS query,
    // make sure that we're able to open the output file for writing.
    info!("Opening `{}` for writing", &args.output_file.display());
    let mut output_file = File::create(&args.output_file).with_context(||
        format!("Failed to create `{}`", &args.output_file.display()))?;

    for site in sites.iter_mut() {
        (site.addrs, site.headers) = look_and_connect(site)?;
    }

    // Write the resulting structure to an output file as JSON.
    info!("Writing Site structs to `{}`", &args.output_file.display());
    serde_json::to_writer_pretty(&mut output_file, &sites).with_context(||
        format!("Failed to write to `{}`", &args.output_file.display()))?;

    Ok(())
}
