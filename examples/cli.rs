use anyhow::Context;
use clap::{App, Arg};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let matches = App::new("Savon CLI")
        .args(&[
            Arg::with_name("input")
                .help("Input WSD file")
                .required(true)
                .takes_value(true),
            Arg::with_name("output")
                .help("Output Rust file")
                .required(true)
                .takes_value(true),
        ])
        .get_matches();

    let input = matches.value_of("input");
    let output = matches.value_of("output");

    let mut input: Box<dyn std::io::BufRead> = match input {
        Some("-") | None => Box::new(std::io::BufReader::new(std::io::stdin())),
        Some(file) => Box::new(std::io::BufReader::new(
            std::fs::File::open(file).context("Failed to open input file")?,
        )),
    };

    let mut output: Box<dyn std::io::Write> = match output {
        Some("-") | None => Box::new(std::io::BufWriter::new(std::io::stdout())),
        Some(file) => Box::new(std::io::BufWriter::new(
            std::fs::File::create(file).context("Failed to open output file")?,
        )),
    };

    let mut data = Vec::new();
    input.read_to_end(&mut data)?;
    let wsdl = savon::wsdl::parse(&data).unwrap();
    let gen = savon::gen::gen(&wsdl).unwrap();

    output
        .write_all(gen.as_bytes())
        .context("Failed to write output file")?;

    Ok(())
}
