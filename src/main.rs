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
    entry: PathBuf,
    #[arg(env, long, short = 'o')]
    output: Option<PathBuf>,
    #[arg(env, long, short = 'f')]
    format: Option<Format>,
    #[arg(env, long, short = 'w')]
    watch: bool,
}

fn main() {
    fn run() -> Result<(), Box<dyn StdError>> {
        let args = Args::parse();

        let doc_format = match args.format.or_else(|| Format::detect(&args.entry)) {
            Some(f) => f,
            None => return Err("Could not auto-detect entry document format. Please specify it using the `--format` argument".into()),
        };

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
                        reset_watcher(watcher, &mut import_paths, doc_format, &args.entry)?;
                        let output = output_writer(args.output.as_ref())?;
                        DocDoc::stitch(doc_format, output, &args.entry)?;
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
) -> Result<(), Box<dyn StdError>> {
    for path in import_paths.iter() {
        watcher.unwatch(path)?;
    }
    *import_paths = DocDoc::list_imports(doc_format, entry_path)?;
    for path in import_paths.iter() {
        watcher.watch(path, RecursiveMode::NonRecursive)?;
    }
    Ok(())
}
