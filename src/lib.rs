use std::{
    borrow::Cow,
    error::Error,
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
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
                writeln!(self.output, "{}", expr.eval(self.base_path.as_ref()))?;
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
        // dbg!(line);
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

    fn eval(&self, base_path: &Path) -> Cow<'_, str> {
        use std::fmt::Write;
        use Expr::*;
        match self {
            Text(t) => Cow::Borrowed(t),
            IncludePath(p) => {
                let base_path = base_path.join(p);
                let contents = fs::read_to_string(&base_path).unwrap();
                let mut lines = contents.lines().peekable();
                let mut rendered = String::new();

                while let Some(line) = lines.next() {
                    let exprs = Expr::parse(line);
                    for expr in exprs {
                        rendered.push_str(&expr.eval(&base_path.parent().unwrap()))
                    }
                    if lines.peek().is_some() {
                        writeln!(rendered).unwrap();
                    }
                }

                Cow::Owned(rendered)
            }
        }
    }
}
