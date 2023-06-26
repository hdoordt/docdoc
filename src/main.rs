use std::{
    error::Error,
    fs::File,
    io::{stdout, BufReader, Write},
    path::PathBuf,
    process::exit,
};

use clap::Parser;
use docdoc::{DocDoc, Format};

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

        let output: Box<dyn Write> = match &args.output {
            Some(output) => match File::create(&output).or_else(|_| File::open(&output)) {
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

        let docdoc = DocDoc::new(doc_format, BufReader::new(entry), output, args.entry);

        docdoc.stitch().map_err(Into::into)
    }

    if let Err(e) = run() {
        eprintln!("DocDoc ended with an error");
        eprintln!("{e}");
        exit(1);
    }
}
