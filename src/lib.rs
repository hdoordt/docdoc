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
pub enum Format {
    #[cfg_attr(feature = "clap", value(alias = "md"))]
    Markdown,
}

impl Format {
    pub fn detect(path: impl AsRef<Path>) -> Option<Self> {
        path.as_ref()
            .extension()
            .map(|ext| match ext.to_str() {
                Some("md") => Some(Format::Markdown),
                Some(_) | None => None,
            })
            .flatten()
    }
}

pub struct DocDoc;

impl DocDoc {
    pub fn list_imports(_format: Format, entry_path: PathBuf) -> Result<HashSet<PathBuf>, Error> {
        let file = File::open(&entry_path)?;
        let input = BufReader::new(file);
        let base_path = entry_path.parent().unwrap().to_owned();
        let touched_paths = HashSet::from([entry_path]);
        let mut all_touched_paths = HashSet::new();
        for line in input.lines() {
            let line = line?;
            let exprs = Expr::parse(&line);
            for expr in exprs {
                let paths = expr.eval(&base_path, &mut sink(), touched_paths.clone())?;
                all_touched_paths.extend(paths);
            }
        }
        Ok(all_touched_paths)
    }

    pub fn stitch(
        _format: Format,
        mut output: impl Write,
        entry_path: PathBuf,
    ) -> Result<(), Error> {
        let file = File::open(&entry_path)?;
        let input = BufReader::new(file);
        let base_path = entry_path.parent().unwrap().to_owned();
        let touched_paths = HashSet::from([entry_path]);
        for line in input.lines() {
            let line = line?;
            let exprs = Expr::parse(&line);
            for expr in exprs {
                expr.eval(&base_path, &mut output, touched_paths.clone())?;
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
                let mut lines = included_file.lines();
                let mut all_touched_paths = touched_paths.clone();
                while let Some(line) = lines.next() {
                    let line = line?;
                    let exprs = Expr::parse(&line);
                    for expr in exprs {
                        let touched_paths = expr.eval(
                            &absolute_path.parent().unwrap(),
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
