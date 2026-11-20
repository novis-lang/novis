//! `rule:testing/mutation-testing`'s `--since <ref>`: the lines of a program
//! that changed since a git ref.
//!
//! # What runs
//!
//! [`changed`] runs the `git` program as a child process with no shell, in
//! the program's folder, four times:
//!
//! 1. `git rev-parse --show-toplevel` finds the checkout's root, which every
//!    path git prints is relative to. A folder outside a checkout stops here.
//! 2. `git rev-parse --verify --quiet --end-of-options <ref>^{commit}` judges
//!    the ref, so a ref git does not know is its own error.
//! 3. `git diff --no-ext-diff --no-color --no-textconv --no-relative
//!    --unified=0 --src-prefix=a/ --dst-prefix=b/ --end-of-options <ref> --`
//!    gives the added and modified lines of the working tree. The prefixes and
//!    `--no-relative` are spelled out so a user's `diff.noprefix` or
//!    `diff.relative` setting cannot move a path.
//! 4. `git ls-files -z --others --exclude-standard --full-name` gives the
//!    untracked files, and each counts as changed on every line.
//!
//! # A ref is never read as an option
//!
//! A ref that starts with `-` is refused before git runs: no branch, tag or
//! commit is named that way, and `--since=--output=x` would otherwise reach
//! git as an option that writes a file. `--end-of-options` keeps any other
//! ref a ref as well.
//!
//! # What it spends
//!
//! One range list per changed file, for the length of the run. A line that
//! was only deleted leaves no line in the working tree, so it adds no range.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The changed lines of each file, by canonical path. Lines count from 1,
/// and each range is inclusive.
#[derive(Debug, Default)]
pub(crate) struct Changed {
    files: BTreeMap<PathBuf, Lines>,
}

/// The changed lines of one file.
#[derive(Debug)]
enum Lines {
    /// The file is untracked, so every line is new.
    Whole,
    /// The ranges git's hunks added, in the order git printed them.
    Ranges(Vec<(usize, usize)>),
}

impl Changed {
    /// Whether any line from `first` to `last` of `file` changed. `file` is
    /// canonical, as [`std::fs::canonicalize`] gives it.
    pub(crate) fn touches(&self, file: &Path, first: usize, last: usize) -> bool {
        match self.files.get(file) {
            None => false,
            Some(Lines::Whole) => true,
            Some(Lines::Ranges(ranges)) => ranges
                .iter()
                .any(|&(start, end)| start <= last && first <= end),
        }
    }
}

/// The lines changed since `reference` in the checkout `folder` is in. `Err`
/// is the message the run stops with.
pub(crate) fn changed(folder: &Path, reference: &str) -> Result<Changed, String> {
    if reference.starts_with('-') {
        return Err(format!(
            "`--since={reference}` starts with `-`. A branch, tag or commit name never starts with `-`"
        ));
    }
    let root = git(folder, &["rev-parse", "--show-toplevel"]).map_err(|failed| match failed {
        Failed::Missing => missing(),
        Failed::Status(_) => format!(
            "`--since` needs a git checkout, and `{}` is not in one",
            folder.display()
        ),
    })?;
    let root = PathBuf::from(String::from_utf8_lossy(&root).trim());
    let commit = format!("{reference}^{{commit}}");
    git(
        folder,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            &commit,
        ],
    )
    .map_err(|failed| match failed {
        Failed::Missing => missing(),
        Failed::Status(_) => {
            format!("`--since={reference}` names no branch, tag or commit that git knows")
        }
    })?;
    let diff = git(
        folder,
        &[
            "diff",
            "--no-ext-diff",
            "--no-color",
            "--no-textconv",
            "--no-relative",
            "--unified=0",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            "--end-of-options",
            reference,
            "--",
        ],
    )
    .map_err(|failed| git_failed(failed, "diff"))?;
    let untracked = git(
        folder,
        &[
            "ls-files",
            "-z",
            "--others",
            "--exclude-standard",
            "--full-name",
        ],
    )
    .map_err(|failed| git_failed(failed, "ls-files"))?;

    let mut changed = Changed::default();
    for (path, ranges) in hunks(&String::from_utf8_lossy(&diff)) {
        if let Ok(path) = std::fs::canonicalize(root.join(path)) {
            changed.files.insert(path, Lines::Ranges(ranges));
        }
    }
    for path in untracked
        .split(|&byte| byte == 0)
        .filter(|path| !path.is_empty())
    {
        let path = root.join(String::from_utf8_lossy(path).as_ref());
        if let Ok(path) = std::fs::canonicalize(path) {
            changed.files.insert(path, Lines::Whole);
        }
    }
    Ok(changed)
}

/// Why one git command gave nothing.
enum Failed {
    /// No `git` program could be started.
    Missing,
    /// It ended with a status other than 0, and this is its first error line.
    Status(String),
}

fn missing() -> String {
    "`--since` runs the `git` program, and no `git` was found on `PATH`".to_owned()
}

fn git_failed(failed: Failed, command: &str) -> String {
    match failed {
        Failed::Missing => missing(),
        Failed::Status(line) => format!("`git {command}` failed: {line}"),
    }
}

/// Runs `git` with `args` in `folder`, and gives its standard output.
fn git(folder: &Path, args: &[&str]) -> Result<Vec<u8>, Failed> {
    let out = Command::new("git")
        .args(args)
        .current_dir(folder)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|_| Failed::Missing)?;
    if out.status.success() {
        return Ok(out.stdout);
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    Err(Failed::Status(
        stderr
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("")
            .trim()
            .to_owned(),
    ))
}

/// The added line ranges of each file in a `--unified=0` diff, by its path
/// relative to the checkout's root. A deleted file has none.
fn hunks(diff: &str) -> BTreeMap<String, Vec<(usize, usize)>> {
    let mut files: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
    let mut current: Option<String> = None;
    for line in diff.lines() {
        if line.starts_with("diff --git ") {
            current = None;
        } else if let Some(path) = line.strip_prefix("+++ ") {
            let path = unquote(path.trim_end_matches('\t'));
            current = path.strip_prefix("b/").map(str::to_owned);
        } else if let (Some(header), Some(path)) = (line.strip_prefix("@@ "), &current)
            && let Some(range) = added(header)
        {
            files.entry(path.clone()).or_default().push(range);
        }
    }
    files
}

/// The added lines a hunk header such as `-3,2 +4,3 @@` names, or `None`
/// when the hunk only deletes.
fn added(header: &str) -> Option<(usize, usize)> {
    let new = header
        .split_whitespace()
        .find_map(|part| part.strip_prefix('+'))?;
    let (start, count) = match new.split_once(',') {
        Some((start, count)) => (start.parse().ok()?, count.parse().ok()?),
        None => (new.parse().ok()?, 1),
    };
    (count > 0).then(|| (start, start + count - 1))
}

/// A path as git prints it, with the quotes and C escapes git adds to a
/// path with an unusual byte taken off.
fn unquote(path: &str) -> String {
    let Some(inner) = path
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    else {
        return path.to_owned();
    };
    let mut bytes = Vec::with_capacity(inner.len());
    let mut chars = inner.bytes().peekable();
    while let Some(byte) = chars.next() {
        if byte != b'\\' {
            bytes.push(byte);
            continue;
        }
        match chars.next() {
            Some(b'n') => bytes.push(b'\n'),
            Some(b't') => bytes.push(b'\t'),
            Some(b'r') => bytes.push(b'\r'),
            Some(digit @ b'0'..=b'7') => {
                let mut value = u32::from(digit - b'0');
                for _ in 0..2 {
                    if let Some(&next @ b'0'..=b'7') = chars.peek() {
                        value = value * 8 + u32::from(next - b'0');
                        chars.next();
                    }
                }
                bytes.push(u8::try_from(value).unwrap_or(b'?'));
            }
            Some(other) => bytes.push(other),
            None => bytes.push(b'\\'),
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_unified_zero_diff_gives_each_files_added_ranges() {
        let diff = "diff --git a/src/Cart.nvs b/src/Cart.nvs\n\
                    index 1..2 100644\n\
                    --- a/src/Cart.nvs\n\
                    +++ b/src/Cart.nvs\n\
                    @@ -3 +3 @@ class Cart\n\
                    -a\n\
                    +b\n\
                    @@ -10,0 +11,2 @@\n\
                    +c\n\
                    +d\n\
                    @@ -20,2 +21,0 @@\n\
                    -e\n\
                    -f\n\
                    diff --git a/gone.nvs b/gone.nvs\n\
                    deleted file mode 100644\n\
                    --- a/gone.nvs\n\
                    +++ /dev/null\n\
                    @@ -1 +0,0 @@\n\
                    -x\n\
                    diff --git \"a/sp\\303\\244ce.nvs\" \"b/sp\\303\\244ce.nvs\"\n\
                    --- \"a/sp\\303\\244ce.nvs\"\n\
                    +++ \"b/sp\\303\\244ce.nvs\"\n\
                    @@ -1 +1 @@\n";
        let files = hunks(diff);
        assert_eq!(files["src/Cart.nvs"], vec![(3, 3), (11, 12)]);
        assert!(!files.contains_key("gone.nvs"));
        assert_eq!(files["sp\u{e4}ce.nvs"], vec![(1, 1)]);
    }

    #[test]
    fn a_ref_that_starts_with_a_dash_is_refused_before_git_runs() {
        // The folder does not exist, so a git that ran would fail another way.
        let error = changed(Path::new("no such folder"), "--output=x").unwrap_err();
        assert!(error.contains("starts with `-`"), "{error}");
    }
}
