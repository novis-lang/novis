//! `rule:packaging/the-argv-lives-in-imagepath`'s Windows `ImagePath`: a binary and its argv encoded
//! into the one string the SCM keeps, and [`decode`], `CommandLineToArgvW`'s rules, which reads it back.
//!
//! The pair lives in a library crate so the fuzz workspace can hold their round trip
//! (`fuzz/fuzz_targets/image_path.rs`); `nvs-cli` is a binary, which nothing can depend on. It is
//! here because this is the lowest crate of the workspace with no platform dependency, and it is
//! pure string work: nothing in it touches the SCM, and it compiles and is tested on every host.

/// The binary and the argv, encoded into one string under `CommandLineToArgvW`'s rules.
///
/// The binary path is quoted **unconditionally**, whether or not it currently contains a space —
/// the rule that still holds after somebody moves the installation, which is when the
/// unquoted-path finding usually appears.
pub fn image_path(exe: &str, argv: &[String]) -> String {
    let mut out = quote(exe, true);
    for word in argv {
        out.push(' ');
        out.push_str(&quote(word, false));
    }
    out
}

/// One argument, encoded so `CommandLineToArgvW` gives it back unchanged.
///
/// The rule that makes this worth confining to one function: a backslash is literal *except* in
/// the run immediately before a quote, where each one is doubled — including the run at the end of
/// the argument, which sits before the closing quote this encoder adds. A directory path ending in
/// `\` is therefore the case a hand-written joiner gets wrong.
fn quote(argument: &str, force: bool) -> String {
    if !force && !argument.is_empty() && !argument.contains([' ', '\t', '"']) {
        return argument.to_owned();
    }
    let mut out = String::from('"');
    let mut backslashes = 0;
    for character in argument.chars() {
        match character {
            '\\' => backslashes += 1,
            '"' => {
                for _ in 0..backslashes * 2 + 1 {
                    out.push('\\');
                }
                out.push('"');
                backslashes = 0;
            }
            _ => {
                for _ in 0..backslashes {
                    out.push('\\');
                }
                backslashes = 0;
                out.push(character);
            }
        }
    }
    for _ in 0..backslashes * 2 {
        out.push('\\');
    }
    out.push('"');
    out
}

/// `CommandLineToArgvW`'s rules, so [`image_path`]'s round trip is a property a test and a fuzz
/// target can assert rather than a claim.
///
/// Two deliberate narrowings, both outside what [`image_path`] produces. The `""`-inside-a-quoted-run
/// form of a literal quote is read as a close followed by a reopen rather than as one quote, and
/// [`quote`] never emits it, since it writes `\"`. The first word is read under the same rules as
/// the rest, where Windows reads it as a quoted path with no escapes; the two agree for any binary
/// path, which holds no quote and does not end in `\`.
pub fn decode(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quoted = false;
    let mut backslashes = 0usize;
    for character in line.chars() {
        match character {
            '\\' => {
                backslashes += 1;
                started = true;
            }
            '"' => {
                for _ in 0..backslashes / 2 {
                    current.push('\\');
                }
                if backslashes % 2 == 1 {
                    current.push('"');
                } else {
                    quoted = !quoted;
                }
                backslashes = 0;
                started = true;
            }
            ' ' | '\t' if !quoted => {
                for _ in 0..backslashes {
                    current.push('\\');
                }
                backslashes = 0;
                if started {
                    out.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            _ => {
                for _ in 0..backslashes {
                    current.push('\\');
                }
                backslashes = 0;
                current.push(character);
                started = true;
            }
        }
    }
    for _ in 0..backslashes {
        current.push('\\');
    }
    if started {
        out.push(current);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{decode, image_path};

    fn words(list: &[&str]) -> Vec<String> {
        list.iter().map(|word| (*word).to_owned()).collect()
    }

    /// The SCM stores one string and the process gets it back through `CommandLineToArgvW`, so the
    /// encoder's round trip is the property: the named cases first, then every word of up to four
    /// characters over the alphabet that decides the encoding.
    #[test]
    fn an_image_path_round_trips_through_command_line_to_argv() {
        let exe = r"C:\Program Files\Novis\nvs.exe";
        let named = [
            // A trailing backslash immediately before the closing quote, which is the case a
            // hand-written joiner gets wrong.
            words(&["serve", "--config", r"C:\app dir\"]),
            // An embedded quote, and a backslash run in front of one.
            words(&["run", r"C:\app\index.nvs", "a\"b", "c\\\\\"d"]),
            // A space, an empty word, a lone backslash and a tab.
            words(&["serve", r"C:\with space\nvs.toml", "", "\\", "a\tb"]),
        ];
        for argv in named {
            let line = image_path(exe, &argv);
            let decoded = decode(&line);
            let (image, rest) = decoded.split_first().expect("an image path");
            assert_eq!(image, exe, "{line}");
            assert_eq!(rest, argv.as_slice(), "{line}");
        }

        let alphabet = ['a', '\\', '"', ' ', '\t'];
        let mut every = vec![String::new()];
        let mut last = vec![String::new()];
        for _ in 0..4 {
            last = last
                .iter()
                .flat_map(|word| {
                    alphabet
                        .iter()
                        .map(move |character| format!("{word}{character}"))
                })
                .collect();
            every.extend(last.iter().cloned());
        }
        for word in &every {
            let argv = vec![word.clone(), "next".to_owned(), word.clone()];
            let line = image_path(exe, &argv);
            assert_eq!(decode(&line)[1..], argv[..], "{word:?} as {line}");
        }
    }

    /// The binary path is quoted although it contains no space, which is what keeps the
    /// unquoted-path finding away after the installation moves.
    #[test]
    fn the_binary_path_is_quoted_with_no_space_in_it() {
        let line = image_path(r"C:\nvs\nvs.exe", &words(&["serve"]));
        assert_eq!(line, r#""C:\nvs\nvs.exe" serve"#);
        assert_eq!(decode(&line), words(&[r"C:\nvs\nvs.exe", "serve"]));
    }
}
