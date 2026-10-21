//! The `Exec` line: quoting, field codes, and how many files a command takes.

use std::path::{Path, PathBuf};

use files_desktop::{ExecError, ExecLine, Expand, Takes};

const ENTRY: &str = "/usr/share/applications/app.desktop";

fn paths(files: &[&str]) -> Vec<PathBuf> {
    files.iter().map(PathBuf::from).collect()
}

fn expand(text: &str, files: &[&str], icon: Option<&str>) -> Vec<String> {
    let line = ExecLine::parse(text).unwrap();
    let files = paths(files);
    line.expand(&Expand {
        files: &files,
        icon,
        name: "My App",
        entry_path: Path::new(ENTRY),
    })
    .iter()
    .map(|arg| arg.to_string_lossy().into_owned())
    .collect()
}

/// One row: the Exec line, the files, the icon, and the argv it expands to.
type ExpandRow<'a> = (&'a str, &'a [&'a str], Option<&'a str>, &'a [&'a str]);

#[test]
fn field_codes_expand() {
    let rows: [ExpandRow; 18] = [
        (
            "gedit %F",
            &["/a/1", "/a/2"],
            None,
            &["gedit", "/a/1", "/a/2"],
        ),
        ("gedit %f", &["/a/1", "/a/2"], None, &["gedit", "/a/1"]),
        ("gedit %f", &[], None, &["gedit"]),
        ("gedit %F", &[], None, &["gedit"]),
        ("gedit %U", &[], None, &["gedit"]),
        (
            "app %u",
            &["/p/a b.txt"],
            None,
            &["app", "file:///p/a%20b.txt"],
        ),
        (
            "app %U",
            &["/p/1", "/p/é"],
            None,
            &["app", "file:///p/1", "file:///p/%C3%A9"],
        ),
        (
            "app --uri=%u",
            &["/p/a b"],
            None,
            &["app", "--uri=file:///p/a%20b"],
        ),
        ("app --file=%f", &[], None, &["app", "--file="]),
        ("app %i", &[], Some("ico"), &["app", "--icon", "ico"]),
        ("app %i", &[], None, &["app"]),
        ("app %c %k", &[], None, &["app", "My App", ENTRY]),
        ("app --name=%c", &[], None, &["app", "--name=My App"]),
        ("app %d %D %n %N %v %m x", &[], None, &["app", "x"]),
        ("app 100%% sure", &[], None, &["app", "100%", "sure"]),
        ("app %%f", &["/a"], None, &["app", "%f"]),
        ("a   b\tc", &[], None, &["a", "b", "c"]),
        (r#"sh -c "cat %f""#, &["/a"], None, &["sh", "-c", "cat /a"]),
    ];
    for (text, files, icon, want) in rows {
        assert_eq!(expand(text, files, icon), want, "{text}");
    }
}

#[test]
fn quoting_follows_the_specification() {
    let rows: [(&str, &[&str]); 8] = [
        (
            r#""/opt/My App/run" --flag "two words" plain"#,
            &["/opt/My App/run", "--flag", "two words", "plain"],
        ),
        (r#"app """#, &["app", ""]),
        (r#"app a"b c"d"#, &["app", "ab cd"]),
        (
            r#"sh -c "echo \"hi\" \$HOME \`x\` \\ end""#,
            &["sh", "-c", r#"echo "hi" $HOME `x` \ end"#],
        ),
        // A backslash before any other character stays.
        (r#"app "a\b""#, &["app", r"a\b"]),
        // Outside quotes a backslash is just a character.
        (r"app a\b", &["app", r"a\b"]),
        (r#"app "%%" "50%%""#, &["app", "%", "50%"]),
        (r#"app "a  b""#, &["app", "a  b"]),
    ];
    for (text, want) in rows {
        assert_eq!(expand(text, &[], None), want, "{text}");
    }
}

#[test]
fn a_command_that_cannot_be_read_is_refused() {
    let rows = [
        ("", ExecError::Empty),
        ("   ", ExecError::Empty),
        ("app %x", ExecError::UnknownCode('x')),
        ("app %", ExecError::TrailingPercent),
        (r#"app "never closed"#, ExecError::UnclosedQuote),
        ("%f app", ExecError::CodeInProgram),
    ];
    for (text, want) in rows {
        assert_eq!(ExecLine::parse(text), Err(want), "{text}");
    }
}

#[test]
fn a_command_says_how_many_files_it_takes() {
    let rows = [
        ("app", Takes::NoFiles),
        ("app %i %c %k", Takes::NoFiles),
        ("app %f", Takes::OneFile),
        ("app --x=%u", Takes::OneFile),
        ("app %F", Takes::ManyFiles),
        ("app %U", Takes::ManyFiles),
        ("app %f %F", Takes::ManyFiles),
        ("app %u %i", Takes::OneFile),
    ];
    for (text, want) in rows {
        assert_eq!(ExecLine::parse(text).unwrap().takes(), want, "{text}");
    }
}
