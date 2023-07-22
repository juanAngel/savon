use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run compilation tests
    Test {},
}

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Test {} => {
            println!("Bzzzzt.... Done");
        }
    }
}
