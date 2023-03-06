use clap::Parser;
use log::{info, warn, debug};

/// A program for making DNS queries on a list of names, then trying to determine if they are on the F5
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File with list of names to query
    #[arg(short, long)]
    file: String,
}

fn main() {
    env_logger::init();
    debug!("Parsing command-line arguments");
    let args = Args::parse();

    println!("{}", &args.file);
}
