use std::{
    error::Error,
    fs::File,
    io::{stdout, BufReader, Write},
    path::PathBuf,
    process::exit,
};

use clap::Parser;
use docdoctor::{DocDoctor, Format};

#[derive(Parser)]
struct Args {
    entry: PathBuf,
    #[arg(env, long, short = 'o')]
    output: Option<PathBuf>,
    #[arg(env, long, short = 'f')]
    format: Option<Format>,
}

fn main() {
    fn run() -> Result<(), Box<dyn Error>> {
        let args = Args::parse();
        let entry = match File::open(&args.entry) {
            Ok(f) => f,
            Err(e) => {
                return Err(format!(
                    "Error opening entry file at {}: {e}",
                    args.entry.to_string_lossy()
                )
                .into());
            }
        };

        let entry_base_path = match args.entry.parent() {
            Some(p) => p,
            None => return Err("Entry file is not within a folder".into()),
        };

        let output: Box<dyn Write> = match &args.output {
            Some(output) => match File::open(output) {
                Ok(f) => Box::new(f),
                Err(e) => {
                    return Err(format!(
                        "Error creating output file at {}: {e}",
                        output.to_string_lossy()
                    )
                    .into());
                }
            },
            None => Box::new(stdout()),
        };

        let doc_format = match args.format.or_else(|| Format::detect(&args.entry)) {
            Some(f) => f,
            None => return Err("Could not auto-detect entry document format. Please specify it using the `--format` argument".into()),
        };

        let doctor = DocDoctor::new(doc_format, BufReader::new(entry), output, entry_base_path);

        doctor.stitch().map_err(Into::into)
    }

    if let Err(e) = run() {
        eprintln!("DocDoctor ended with an error");
        eprintln!("{e}");
        exit(1);
    }
}
