use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader, Write},
    iter::once,
    path::{Path, PathBuf},
};

pub mod error;
use error::Error;

use regex::Regex;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum Format {
    #[cfg_attr(feature = "clap", value(alias = "adoc"))]
    Asciidoc,
    #[cfg_attr(feature = "clap", value(alias = "md"))]
    Markdown,
}

impl Format {
    pub fn detect(path: impl AsRef<Path>) -> Option<Self> {
        path.as_ref()
            .extension()
            .map(|ext| match ext.to_str() {
                Some("adoc") => Some(Format::Asciidoc),
                Some("md") => Some(Format::Markdown),
                Some(_) | None => None,
            })
            .flatten()
    }
}

pub struct DocDoctor<R, W> {
    #[allow(dead_code)]
    format: Format,
    input: R,
    output: W,
    base_path: PathBuf,
    touched_paths: HashSet<PathBuf>,
}

impl<R, W> DocDoctor<R, W>
where
    R: BufRead,
    W: Write,
{
    pub fn new(format: Format, input: R, output: W, entry_path: PathBuf) -> Self {
        let base_path = entry_path.parent().unwrap().to_owned();
        let touched_paths = HashSet::from_iter(once(entry_path));

        Self {
            format,
            input,
            output,
            base_path,
            touched_paths,
        }
    }

    pub fn stitch(mut self) -> Result<(), Error> {
        for line in self.input.lines() {
            let line = line?;
            let exprs = Expr::parse(&line);
            for expr in exprs {
                expr.eval(
                    &self.base_path,
                    &mut self.output,
                    self.touched_paths.clone(),
                )?;
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
        let include_path = Regex::new(r#"\#\[docdoc:path="([^"\#]*)"\]"#).unwrap();
        let matches: Vec<_> = include_path.find_iter(line).collect();
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
    ) -> Result<(), Error> {
        use Expr::*;
        match self {
            Text(t) => Ok(writeln!(output, "{t}")?),
            IncludePath(p) => {
                let absolute_path = base_path.as_ref().join(p);
                if let Some(absolute_path) = touched_paths.replace(absolute_path.canonicalize()?) {
                    return Err(Error::ImportCycle(absolute_path));
                }

                let included_file = File::open(&absolute_path)?;
                let included_file = BufReader::new(included_file);
                let mut lines = included_file.lines();

                while let Some(line) = lines.next() {
                    let line = line?;
                    let exprs = Expr::parse(&line);
                    for expr in exprs {
                        expr.eval(
                            &absolute_path.parent().unwrap(),
                            output,
                            touched_paths.clone(),
                        )?;
                    }
                }

                Ok(())
            }
        }
    }
}
