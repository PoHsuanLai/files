//! The `Exec` line of a desktop entry: quoting, and the field codes (Desktop Entry Specification,
//! "The Exec key"). Pure: text in, argument lists out.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::uri::file_uri;

/// A `%` code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Code {
    /// `%f`: one file.
    File,
    /// `%F`: a list of files.
    Files,
    /// `%u`: one address.
    Url,
    /// `%U`: a list of addresses.
    Urls,
    /// `%i`: `--icon` and the entry's icon.
    Icon,
    /// `%c`: the entry's name.
    Name,
    /// `%k`: the entry's own path.
    EntryPath,
    /// `%d %D %n %N %v %m`: deprecated, removed.
    Dropped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    Text(String),
    Code(Code),
}

/// One argument: the pieces that make it up.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Arg(Vec<Piece>);

/// A parsed `Exec` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecLine {
    args: Vec<Arg>,
}

/// Why an `Exec` value was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ExecError {
    /// Nothing to run.
    #[error("The app has no command to run.")]
    Empty,
    /// A quote is never closed.
    #[error("The app\u{2019}s command has a quote that is never closed.")]
    UnclosedQuote,
    /// A `%` code the specification does not have.
    #[error("The app\u{2019}s command uses an unknown code, \u{201c}%{0}\u{201d}.")]
    UnknownCode(char),
    /// A `%` at the very end.
    #[error("The app\u{2019}s command ends with a stray \u{201c}%\u{201d}.")]
    TrailingPercent,
    /// A code where the program's name should be.
    #[error("The app\u{2019}s command starts with a code instead of a program.")]
    CodeInProgram,
}

/// How many files a command takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Takes {
    /// It has no file code: the files cannot be passed.
    NoFiles,
    /// `%f` or `%u`: run it once per file.
    OneFile,
    /// `%F` or `%U`: run it once with all the files.
    ManyFiles,
}

/// What the codes expand to.
#[derive(Debug, Clone, Copy)]
pub struct Expand<'a> {
    /// The files, as paths. `%f` and `%u` take the first.
    pub files: &'a [PathBuf],
    /// The entry's `Icon`.
    pub icon: Option<&'a str>,
    /// The entry's `Name`.
    pub name: &'a str,
    /// The entry's file.
    pub entry_path: &'a Path,
}

#[derive(Clone, Copy)]
enum Quote {
    Out,
    In,
}

#[derive(Default)]
struct ArgBuilder {
    pieces: Vec<Piece>,
    text: String,
    started: bool,
}

impl ArgBuilder {
    fn code(&mut self, code: Code) {
        self.flush();
        self.pieces.push(Piece::Code(code));
        self.started = true;
    }

    fn flush(&mut self) {
        if !self.text.is_empty() {
            self.pieces
                .push(Piece::Text(std::mem::take(&mut self.text)));
        }
    }

    /// Ends the argument, if one was begun.
    fn finish(&mut self, args: &mut Vec<Arg>) {
        self.flush();
        if self.started || !self.pieces.is_empty() {
            args.push(Arg(std::mem::take(&mut self.pieces)));
        }
        self.started = false;
    }
}

fn code_of(letter: char) -> Result<Option<Code>, ExecError> {
    Ok(Some(match letter {
        '%' => return Ok(None),
        'f' => Code::File,
        'F' => Code::Files,
        'u' => Code::Url,
        'U' => Code::Urls,
        'i' => Code::Icon,
        'c' => Code::Name,
        'k' => Code::EntryPath,
        'd' | 'D' | 'n' | 'N' | 'v' | 'm' => Code::Dropped,
        other => return Err(ExecError::UnknownCode(other)),
    }))
}

impl ExecLine {
    /// Reads `text`, which is the `Exec` value with its key-file escapes already taken.
    ///
    /// Arguments are split at spaces; a double quote holds spaces in one argument, and inside it a
    /// backslash escapes `"`, `` ` ``, `$` and `\`. `%%` is a literal percent sign.
    pub fn parse(text: &str) -> Result<ExecLine, ExecError> {
        let mut args = Vec::new();
        let mut current = ArgBuilder::default();
        let mut quote = Quote::Out;
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match (quote, c) {
                (Quote::Out, ' ' | '\t' | '\n') => current.finish(&mut args),
                (Quote::Out, '"') => {
                    quote = Quote::In;
                    current.started = true;
                }
                (Quote::In, '"') => quote = Quote::Out,
                (Quote::In, '\\') => match chars.peek().copied() {
                    Some(next @ ('"' | '`' | '$' | '\\')) => {
                        chars.next();
                        current.text.push(next);
                    }
                    Some(_) | None => current.text.push('\\'),
                },
                (Quote::Out | Quote::In, '%') => {
                    let letter = chars.next().ok_or(ExecError::TrailingPercent)?;
                    match code_of(letter)? {
                        Some(code) => current.code(code),
                        None => current.text.push('%'),
                    }
                }
                (Quote::Out | Quote::In, other) => current.text.push(other),
            }
        }
        if matches!(quote, Quote::In) {
            return Err(ExecError::UnclosedQuote);
        }
        current.finish(&mut args);
        let Some(program) = args.first() else {
            return Err(ExecError::Empty);
        };
        if program
            .0
            .iter()
            .any(|piece| matches!(piece, Piece::Code(_)))
        {
            return Err(ExecError::CodeInProgram);
        }
        Ok(ExecLine { args })
    }

    /// How many files the command takes.
    pub fn takes(&self) -> Takes {
        let codes = self.args.iter().flat_map(|arg| arg.0.iter());
        let mut takes = Takes::NoFiles;
        for piece in codes {
            match piece {
                Piece::Code(Code::Files | Code::Urls) => return Takes::ManyFiles,
                Piece::Code(Code::File | Code::Url) => takes = Takes::OneFile,
                Piece::Code(Code::Icon | Code::Name | Code::EntryPath | Code::Dropped)
                | Piece::Text(_) => {}
            }
        }
        takes
    }

    /// The argument list, program first.
    ///
    /// A list code (`%F`, `%U`) that is an argument on its own becomes one argument per file; a
    /// single code (`%f`, `%u`) on its own with no file is left out, as `%i` is without an icon.
    /// Inside a longer argument a code is replaced by its first value, or by nothing.
    pub fn expand(&self, with: &Expand<'_>) -> Vec<OsString> {
        let mut argv = Vec::new();
        for arg in &self.args {
            arg.expand_into(with, &mut argv);
        }
        argv
    }
}

impl Arg {
    fn expand_into(&self, with: &Expand<'_>, out: &mut Vec<OsString>) {
        match self.0.as_slice() {
            [Piece::Code(Code::Files)] => {
                out.extend(with.files.iter().map(|p| p.as_os_str().to_owned()));
            }
            [Piece::Code(Code::Urls)] => {
                out.extend(with.files.iter().map(|p| OsString::from(file_uri(p))));
            }
            [Piece::Code(Code::File)] => {
                out.extend(with.files.first().map(|p| p.as_os_str().to_owned()));
            }
            [Piece::Code(Code::Url)] => {
                out.extend(with.files.first().map(|p| OsString::from(file_uri(p))));
            }
            [Piece::Code(Code::Icon)] => {
                if let Some(icon) = with.icon {
                    out.push(OsString::from("--icon"));
                    out.push(OsString::from(icon));
                }
            }
            [Piece::Code(Code::Dropped)] => {}
            _ => out.push(self.joined(with)),
        }
    }

    fn joined(&self, with: &Expand<'_>) -> OsString {
        let mut joined = OsString::new();
        for piece in &self.0 {
            match piece {
                Piece::Text(text) => joined.push(text),
                Piece::Code(code) => joined.push(code.text(with)),
            }
        }
        joined
    }
}

impl Code {
    /// What the code is inside a longer argument.
    fn text(self, with: &Expand<'_>) -> OsString {
        let first = with.files.first();
        let text = match self {
            Code::File | Code::Files => first.map(|p| p.as_os_str().to_owned()),
            Code::Url | Code::Urls => first.map(|p| OsString::from(file_uri(p))),
            Code::Icon => with.icon.map(OsString::from),
            Code::Name => Some(OsString::from(with.name)),
            Code::EntryPath => Some(with.entry_path.as_os_str().to_owned()),
            Code::Dropped => None,
        };
        text.unwrap_or_default()
    }
}
