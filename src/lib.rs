use std::{
    collections::HashSet,
    fs::File,
    io::{sink, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub mod error;
use error::Error;

use regex::Regex;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
/// Supported file formats
pub enum Format {
    /// A MarkDown document
    #[cfg_attr(feature = "clap", value(alias = "md"))]
    Markdown,
}

impl Format {
    /// Auto-detect file format based on its extension
    pub fn detect(path: impl AsRef<Path>) -> Option<Self> {
        path.as_ref()
            .extension()
            .and_then(|ext| match ext.to_str() {
                Some("md") => Some(Format::Markdown),
                Some(_) | None => None,
            })
    }
}

pub enum DocDoc {}

impl DocDoc {
    /// Traverse the import tree from the entry path, and return a list of all
    /// paths from which files are imported.
    pub fn list_imports(
        _format: Format,
        entry_path: impl AsRef<Path>,
    ) -> Result<HashSet<PathBuf>, Error> {
        let mut all_touched_paths = HashSet::new();
        Self::traverse(_format, sink(), entry_path, |paths| {
            all_touched_paths.extend(paths);
        })?;

        Ok(all_touched_paths)
    }

    /// Traverse the import tree from the entry path, and render all imports
    /// to the passed [Write].
    pub fn stitch(
        _format: Format,
        output: impl Write,
        entry_path: impl AsRef<Path>,
    ) -> Result<(), Error> {
        Self::traverse(_format, output, entry_path, |_| ())
    }

    fn traverse<F>(
        _format: Format,
        mut output: impl Write,
        entry_path: impl AsRef<Path>,
        mut func: F,
    ) -> Result<(), Error>
    where
        F: FnMut(HashSet<PathBuf>),
    {
        let file = File::open(&entry_path)?;
        let input = BufReader::new(file);
        let base_path = entry_path.as_ref().parent().unwrap().to_owned();
        let touched_paths = HashSet::from([entry_path.as_ref().to_owned()]);

        for line in input.lines() {
            let line = line?;
            let exprs = Expr::parse(&line);
            for expr in exprs {
                let paths = expr.eval(&base_path, &mut output, touched_paths.clone())?;
                func(paths);
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
enum Expr<'src> {
    Text(&'src str),
    IncludePath(&'src Path),
}

impl<'src> Expr<'src> {
    fn parse(line: &'src str) -> Vec<Self> {
        static REGEX: OnceLock<Regex> = OnceLock::new();
        let include_regex =
            REGEX.get_or_init(|| Regex::new(r#"\#\[docdoc:path="([^"\#]*)"\]"#).unwrap());

        let matches: Vec<_> = include_regex.find_iter(line).collect();
        if matches.is_empty() {
            return vec![Expr::Text(line)];
        }
        let mut exprs = Vec::new();
        let mut cursor = 0;

        for m in matches {
            if m.start() > cursor {
                exprs.push(Expr::Text(&line[cursor..m.start()]));
            }
            let path = &line[m.start() + r#"#[docdoc:path=""#.len()..m.end() - r#""]"#.len()];
            exprs.push(Expr::IncludePath(Path::new(path)));
            cursor = m.end();
        }

        if cursor < line.len() {
            exprs.push(Expr::Text(&line[cursor..]));
        }

        exprs
    }

    fn eval(
        &self,
        base_path: impl AsRef<Path>,
        output: &mut impl Write,
        mut touched_paths: HashSet<PathBuf>,
    ) -> Result<HashSet<PathBuf>, Error> {
        use Expr::*;
        match self {
            Text(t) => {
                writeln!(output, "{t}")?;
                Ok(touched_paths)
            }
            IncludePath(p) => {
                let absolute_path = base_path.as_ref().join(p);
                if let Some(absolute_path) = touched_paths.replace(absolute_path.canonicalize()?) {
                    return Err(Error::ImportCycle(absolute_path));
                }

                let included_file = File::open(&absolute_path)?;
                let included_file = BufReader::new(included_file);
                let mut all_touched_paths = touched_paths.clone();

                for line in included_file.lines() {
                    let line = line?;
                    let exprs = Expr::parse(&line);
                    for expr in exprs {
                        let touched_paths = expr.eval(
                            absolute_path.parent().unwrap(),
                            output,
                            touched_paths.clone(),
                        )?;
                        all_touched_paths.extend(touched_paths);
                    }
                }

                Ok(all_touched_paths)
            }
        }
    }
}
