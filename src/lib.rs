use std::{
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader, Write},
    path::Path,
};

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

pub struct DocDoctor<R, W, P> {
    #[allow(dead_code)]
    format: Format,
    input: R,
    output: W,
    base_path: P,
}

impl<R, W, P> DocDoctor<R, W, P>
where
    R: BufRead,
    W: Write,
    P: AsRef<Path>,
{
    pub fn new(format: Format, input: R, output: W, base_path: P) -> Self {
        Self {
            format,
            input,
            output,
            base_path,
        }
    }

    pub fn stitch(mut self) -> Result<(), Box<dyn Error>> {
        for line in self.input.lines() {
            let line = line?;
            let exprs = Expr::parse(&line);
            for expr in exprs {
                expr.eval(&self.base_path, &mut self.output)?;
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

    fn eval(&self, base_path: impl AsRef<Path>, output: &mut impl Write) -> io::Result<()> {
        use Expr::*;
        match self {
            Text(t) => writeln!(output, "{t}"),
            IncludePath(p) => {
                let absolute_path = base_path.as_ref().join(p);
                let included_file = File::open(&absolute_path)?;
                let included_file = BufReader::new(included_file);
                let mut lines = included_file.lines();

                while let Some(line) = lines.next() {
                    let line = line?;
                    let exprs = Expr::parse(&line);
                    for expr in exprs {
                        expr.eval(&absolute_path.parent().unwrap(), output)?;
                    }
                }

                Ok(())
            }
        }
    }
}
