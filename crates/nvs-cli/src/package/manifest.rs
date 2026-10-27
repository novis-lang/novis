//! A package's `package.toml`, as far as `nvs pkg` reads it
//! (`rule:packaging/package-toml-is-a-tool-input`): its `version` alone, as
//! `rule:packaging/only-a-major-may-break-the-api` reads one, `MAJOR.MINOR.PATCH` and literally.
//!
//! Every other key is left alone and unchecked here. A missing file or a missing `version` is no
//! version, and a `version` that is not three plain numbers is an error naming the file.

use std::fmt;
use std::path::Path;
use std::process::ExitCode;
use std::str::FromStr;

use serde::Deserialize;

/// The file a package directory names itself in.
const FILE: &str = "package.toml";

/// A package version, `MAJOR.MINOR.PATCH`. Each part is a plain decimal number with no leading
/// zero, and nothing may follow the third.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) struct Version {
    pub(crate) major: u64,
    pub(crate) minor: u64,
    pub(crate) patch: u64,
}

impl FromStr for Version {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        let part = |piece: Option<&str>| {
            piece
                .filter(|p| {
                    !p.is_empty()
                        && p.bytes().all(|b| b.is_ascii_digit())
                        && (p.len() == 1 || !p.starts_with('0'))
                })
                .and_then(|p| p.parse::<u64>().ok())
        };
        let mut pieces = text.split('.');
        match (
            part(pieces.next()),
            part(pieces.next()),
            part(pieces.next()),
            pieces.next(),
        ) {
            (Some(major), Some(minor), Some(patch), None) => Ok(Self {
                major,
                minor,
                patch,
            }),
            _ => Err(format!(
                "`{text}` is not a version. A version is three numbers, such as `1.4.0`."
            )),
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The keys of `package.toml` this module reads.
#[derive(Deserialize)]
struct Manifest {
    version: Option<String>,
}

/// The `version` in `<dir>/package.toml`, or `None` when there is no such file or no such key.
/// A file that cannot be read or parsed, and a version that is not one, have already reported on
/// standard error.
pub(crate) fn version(dir: &Path) -> Result<Option<Version>, ExitCode> {
    let path = dir.join(FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            eprintln!("cannot read {}: {err}", path.display());
            return Err(ExitCode::FAILURE);
        }
    };
    let manifest: Manifest = toml::from_str(&text).map_err(|err| {
        eprintln!("{} is not valid TOML: {err}", path.display());
        ExitCode::FAILURE
    })?;
    manifest
        .version
        .map(|text| text.parse())
        .transpose()
        .map_err(|err: String| {
            eprintln!("{}: {err}", path.display());
            ExitCode::FAILURE
        })
}

#[cfg(test)]
mod tests {
    use super::Version;

    /// Three plain numbers parse, and anything else is not a version.
    #[test]
    fn a_version_is_three_plain_numbers() {
        let v: Version = "1.20.3".parse().expect("a version");
        assert_eq!((v.major, v.minor, v.patch), (1, 20, 3));
        assert_eq!(v.to_string(), "1.20.3");
        for bad in [
            "1.2",
            "1.2.3.4",
            "1.02.3",
            "+1.2.3",
            "1.2.3-beta",
            "",
            "a.b.c",
        ] {
            assert!(bad.parse::<Version>().is_err(), "{bad}");
        }
    }
}
