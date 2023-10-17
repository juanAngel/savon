use anyhow::Context;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Input WSD file
    #[arg(short, long, required = true)]
    pub input: String,

    /// Output Rust file
    #[arg(short, long, required = true)]
    pub output: String,
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

    let mut output: Box<dyn std::io::Write> = match &*args.output {
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
