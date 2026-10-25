//! `pathMappings`: the editor's folders paired with the adapter's
//! (`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`).
//!
//! A mapping is `{ "localRoot": <editor folder>, "remoteRoot": <adapter folder> }`, the shape
//! VS Code's own adapters read. A path the editor sends — `program`, `cwd`, a `setBreakpoints`
//! source — is mapped to the adapter's before it is used, and a path the adapter sends back — a
//! `stackTrace` frame's source — is mapped to the editor's. The `skipFiles` globs are written in
//! the editor's terms like the rest of `launch.json`, so a file is mapped to the editor's path
//! before a glob matches it.
//!
//! A root matches whole parts of a path, never part of a folder name, and the longest root that
//! matches wins. On Windows a root matches with no regard to case, and `/` and `\` are the same
//! separator, and the rest of a mapped path takes the separator its new root uses. A path no root
//! matches is kept as it is, so a launch with no mappings maps nothing.
//!
//! **What it spends:** the launch's roots, two strings per mapping and the adapter root's
//! canonical form, for the session.

use serde::Deserialize;

/// One `pathMappings` entry as `launch.json` writes it.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PathMapping {
    /// The folder as the editor names it.
    pub(crate) local_root: String,
    /// The same folder as the adapter's program reads it.
    pub(crate) remote_root: String,
}

/// The launch's mappings, ready to map a path either way.
#[derive(Clone, Debug, Default)]
pub(crate) struct PathMap {
    roots: Vec<Roots>,
}

#[derive(Clone, Debug)]
struct Roots {
    editor: String,
    adapter: String,
    /// `adapter` canonicalised, which is how a frame's file names it.
    canonical: String,
}

impl PathMap {
    /// The map of `mappings`. A mapping with an empty root is skipped, because it would match
    /// every path.
    pub(crate) fn new(mappings: &[PathMapping]) -> Self {
        let roots = mappings
            .iter()
            .filter(|mapping| !mapping.local_root.is_empty() && !mapping.remote_root.is_empty())
            .map(|mapping| Roots {
                editor: mapping.local_root.clone(),
                adapter: mapping.remote_root.clone(),
                canonical: shown(&super::file_key(&mapping.remote_root)).to_owned(),
            })
            .collect();
        Self { roots }
    }

    /// The adapter's path for a path the editor sent.
    pub(crate) fn to_adapter(&self, path: &str) -> String {
        replace(
            path,
            self.roots
                .iter()
                .map(|roots| (roots.editor.as_str(), roots.adapter.as_str())),
        )
    }

    /// The editor's path for a file the adapter names, which may carry Windows's `\\?\` prefix.
    pub(crate) fn to_editor(&self, file: &str) -> String {
        let file = shown(file);
        replace(
            file,
            self.roots.iter().flat_map(|roots| {
                [
                    (roots.canonical.as_str(), roots.editor.as_str()),
                    (roots.adapter.as_str(), roots.editor.as_str()),
                ]
            }),
        )
    }
}

/// A file key as an editor opens it: Windows's `\\?\` prefix, which a canonical path carries,
/// left off.
pub(crate) fn shown(file: &str) -> &str {
    file.strip_prefix(r"\\?\").unwrap_or(file)
}

/// `path` with the longest `from` root that matches it replaced by its `to` root.
fn replace<'a>(path: &str, roots: impl Iterator<Item = (&'a str, &'a str)>) -> String {
    let best = roots
        .filter_map(|(from, to)| rest(path, from).map(|rest| (from.len(), to, rest)))
        .max_by_key(|&(length, ..)| length);
    match best {
        Some((_, to, "")) => to.to_owned(),
        Some((_, to, rest)) => {
            let to = to.trim_end_matches(separator);
            let between = separator_of(to);
            let rest: String = rest
                .chars()
                .map(|c| if separator(c) { between } else { c })
                .collect();
            format!("{to}{between}{rest}")
        }
        None => path.to_owned(),
    }
}

/// What follows `root` in `path`, without the separator, or `None` where `root` does not name
/// whole leading parts of `path`.
fn rest<'a>(path: &'a str, root: &str) -> Option<&'a str> {
    let root = root.trim_end_matches(separator);
    let head = path.get(..root.len())?;
    let same = head.chars().zip(root.chars()).all(|(a, b)| {
        a == b || (cfg!(windows) && (a.eq_ignore_ascii_case(&b) || (separator(a) && separator(b))))
    });
    if !same {
        return None;
    }
    let tail = &path[root.len()..];
    if tail.is_empty() {
        return Some("");
    }
    tail.strip_prefix(separator)
        .map(|tail| tail.trim_start_matches(separator))
}

fn separator(c: char) -> bool {
    c == '/' || (cfg!(windows) && c == '\\')
}

/// The separator a mapped path goes on with: the one its root uses, or the platform's.
fn separator_of(root: &str) -> char {
    if root.contains('/') {
        '/'
    } else if root.contains('\\') {
        '\\'
    } else {
        std::path::MAIN_SEPARATOR
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> PathMap {
        let mappings: Vec<PathMapping> = pairs
            .iter()
            .map(|&(local, remote)| PathMapping {
                local_root: local.to_owned(),
                remote_root: remote.to_owned(),
            })
            .collect();
        PathMap::new(&mappings)
    }

    #[test]
    fn the_longest_root_maps_whole_parts_of_a_path_both_ways() {
        let paths = map(&[("/work", "/srv"), ("/work/blog", "/srv/apps/blog/")]);
        assert_eq!(
            paths.to_adapter("/work/blog/main.nvs"),
            "/srv/apps/blog/main.nvs"
        );
        assert_eq!(
            paths.to_adapter("/work/shop/main.nvs"),
            "/srv/shop/main.nvs"
        );
        assert_eq!(paths.to_adapter("/workshop/main.nvs"), "/workshop/main.nvs");
        assert_eq!(paths.to_adapter("/work"), "/srv");
        assert_eq!(
            paths.to_editor("/srv/apps/blog/main.nvs"),
            "/work/blog/main.nvs"
        );
        assert_eq!(
            paths.to_editor("/elsewhere/main.nvs"),
            "/elsewhere/main.nvs"
        );
    }

    #[test]
    fn no_mapping_and_an_empty_root_map_nothing() {
        assert_eq!(
            PathMap::default().to_adapter("/work/main.nvs"),
            "/work/main.nvs"
        );
        assert_eq!(
            map(&[("", "/srv")]).to_adapter("/work/main.nvs"),
            "/work/main.nvs"
        );
    }

    #[cfg(windows)]
    #[test]
    fn on_windows_a_root_matches_with_no_regard_to_case_or_separator() {
        let paths = map(&[(r"c:\Work", "D:/srv")]);
        assert_eq!(
            paths.to_adapter(r"C:\work\blog\main.nvs"),
            "D:/srv/blog/main.nvs"
        );
        assert_eq!(
            paths.to_editor(r"\\?\d:/SRV/blog/main.nvs"),
            r"c:\Work\blog\main.nvs"
        );
    }
}
