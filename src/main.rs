use std::{
    collections::HashSet,
    error::Error as StdError,
    fs::File,
    io::{self, stdout, Write},
    path::{Path, PathBuf},
    process::exit,
    time::Duration,
};

use clap::Parser;
use docdoc::{DocDoc, Format};
use notify::{RecursiveMode, Watcher};
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};

#[derive(Parser)]
struct Args {
    #[arg(help = "The path of the entry file")]
    entry: PathBuf,
    #[arg(
        env,
        long,
        short = 'o',
        help = "The path of the file to write the output to. Defaults to stdout."
    )]
    output: Option<PathBuf>,
    #[arg(
        env,
        long,
        short = 'f',
        help = "Specify doc format. DocDoc attempts to guess format if not set."
    )]
    format: Option<Format>,
    #[arg(
        env,
        long,
        short = 'w',
        help = "Watch doc import tree, starting at entry file."
    )]
    watch: bool,
}

fn main() {
    fn run() -> Result<(), Box<dyn StdError>> {
        let args = Args::parse();

        let doc_format = match args.format.or_else(|| Format::detect(&args.entry)) {
            Some(f) => f,
            None => return Err("Could not auto-detect entry document format. Please specify it using the `--format` argument".into()),
        };

        args.watch.then(clear_terminal);

        let output = output_writer(args.output.as_ref())?;
        DocDoc::stitch(doc_format, output, &args.entry)?;

        if args.watch {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut debouncer = new_debouncer(Duration::from_millis(100), None, tx)?;

            let watcher = debouncer.watcher();

            let mut import_paths = HashSet::new();

            reset_watcher(watcher, &mut import_paths, doc_format, &args.entry)?;

            for result in rx {
                match result {
                    Ok(events) if events.iter().any(|e| e.kind == DebouncedEventKind::Any) => {
                        clear_terminal();
                        if let Err(e) =
                            reset_watcher(watcher, &mut import_paths, doc_format, &args.entry)
                        {
                            eprintln!("Error resetting watcher: {e}");
                            continue;
                        };
                        let output = match output_writer(args.output.as_ref()) {
                            Ok(o) => o,
                            Err(e) => {
                                eprintln!("Error setting up output writer: {e}");
                                continue;
                            }
                        };
                        if let Err(e) = DocDoc::stitch(doc_format, output, &args.entry) {
                            eprintln!("Error stitching together documents: {e}");
                            continue;
                        }
                    }
                    Err(errs) => {
                        errs.into_iter()
                            .for_each(|e| eprintln!("Error from file watcher: {e}"));
                        return Err("File watcher yielded errors".into());
                    }
                    Ok(_) => { /* No interesting events. Do nothing. */ }
                }
            }
        }

        Ok(())
    }

    if let Err(e) = run() {
        eprintln!("DocDoc ended with an error");
        eprintln!("{e}");
        exit(1);
    }
}

fn output_writer(output_path: Option<impl AsRef<Path>>) -> io::Result<Box<dyn Write>> {
    match output_path {
        Some(op) => {
            let file = File::create(&op).or_else(|_| File::open(&op))?;
            Ok(Box::new(file))
        }
        None => Ok(Box::new(stdout())),
    }
}

fn reset_watcher(
    watcher: &mut dyn Watcher,
    import_paths: &mut HashSet<PathBuf>,
    doc_format: Format,
    entry_path: impl AsRef<Path>,
) -> Result<(), notify::Error> {
    for path in import_paths.iter() {
        watcher.unwatch(path)?;
    }
    *import_paths = DocDoc::list_imports(doc_format, entry_path);
    for path in import_paths.iter() {
        watcher.watch(path, RecursiveMode::NonRecursive)?;
    }
    Ok(())
}

fn clear_terminal() {
    print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
}
