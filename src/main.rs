use clap::Parser;

/// A program for making DNS queries on a list of names, then trying to determine if they are on the F5
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File with list of names to query
    #[arg(short, long)]
    file: String,
}

fn main() {
    let args = Args::parse();

    println!("{}", &args.file);
}
