use anyhow::Context;
use clap::Parser;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Input WSDL file
    #[arg(short, long, required = true)]
    pub input: String,

    /// Output Rust file (Will create next to input if not provided)
    #[arg(short, long, required = false)]
    pub output: Option<String>,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();

    let mut input: Box<dyn std::io::BufRead> = match &*args.input {
        "-" => Box::new(std::io::BufReader::new(std::io::stdin())),
        file => Box::new(std::io::BufReader::new(
            std::fs::File::open(file).context("Failed to open input file")?,
        )),
    };

    let output = if let Some(output) = args.output {
        output
    } else {
        derive_output_filename_from_input(&args.input)
    };

    let mut output: Box<dyn std::io::Write> = match &*output {
        "-" => Box::new(std::io::BufWriter::new(std::io::stdout())),
        file => Box::new(std::io::BufWriter::new(
            std::fs::File::create(file).context("Failed to open output file")?,
        )),
    };

    let mut data = Vec::new();
    input.read_to_end(&mut data)?;
    let wsdl = savon::wsdl::parse(&data).unwrap();
    let gen = savon::gen::gen(&wsdl).unwrap();
    let fmt = prettyplease::unparse(&syn::parse_quote!(#gen));

    output
        .write_all(fmt.as_bytes())
        .context("Failed to write output file")?;

    Ok(())
}

fn derive_output_filename_from_input(input: &str) -> String {
    let path = Path::new(input);
    let stem = path.file_stem().unwrap_or_default();
    let output_path = path.with_file_name(stem).with_extension("rs");
    output_path.to_string_lossy().into_owned()
}
