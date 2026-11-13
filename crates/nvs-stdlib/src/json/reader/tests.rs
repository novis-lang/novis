use super::{Cut, LEVELS, Reader, Source, Text};
use crate::fuzz::{JsonRead, JsonToken, json_oracle, json_read, json_reader_disagreement};

/// `bytes` read from two chunks, cut before byte `at`.
struct Two<'a> {
    first: &'a [u8],
    second: Option<&'a [u8]>,
}

impl Source for Two<'_> {
    fn chunk(&self) -> &[u8] {
        self.first
    }

    fn advance(&mut self) -> bool {
        match self.second.take() {
            Some(second) => {
                self.first = second;
                true
            }
            None => false,
        }
    }
}

/// What `bytes` reads to when it is cut before byte `at`.
fn read_cut(bytes: &[u8], at: usize) -> JsonRead {
    let (first, second) = bytes.split_at(at);
    let source = Two {
        first,
        second: Some(second),
    };
    tokens(Reader::new(source, ceiling()))
}

/// Every event of `reader`, through the same rendering `json_read` uses.
fn tokens<S: Source>(reader: Reader<S>) -> JsonRead {
    crate::fuzz::tokens_of(reader)
}

/// The deepest `maxDepth` a call may ask for.
fn ceiling() -> u32 {
    u32::try_from(LEVELS).expect("the ceiling fits a u32")
}

/// The error `text` reads to, or `None` where it reads to its end.
fn error_of(text: &str) -> Option<String> {
    json_read(text.as_bytes(), None).error
}

#[test]
fn the_reader_accepts_and_refuses_what_rfc_8259_says() {
    let accepted = [
        "null",
        " true ",
        "false",
        "0",
        "-0",
        "1.5e3",
        "-1.25E-2",
        "0.0e+0",
        "18446744073709551615",
        "-9223372036854775808",
        "123456789012345678901234567890",
        "\"a\\\"\\\\\\/\\b\\f\\n\\r\\t\\u00e9\\ud83d\\ude00\"",
        "[]",
        "{}",
        r#"[1,[2,{"a":[]}]]"#,
        r#"{"a":1,"b":{"c":null}}"#,
        "\n\t\r 1 \n",
        "1e-99999999999999",
    ];
    for text in accepted {
        assert_eq!(error_of(text), None, "{text:?} is a JSON document");
    }
    let refused = [
        ("", "EOF while parsing a value at line 1 column 0"),
        ("[1,]", "trailing comma at line 1 column 4"),
        (r#"{"a":1,}"#, "trailing comma at line 1 column 8"),
        ("{a:1}", "key must be a string at line 1 column 2"),
        ("[1 2]", "expected `,` or `]` at line 1 column 4"),
        (r#"{"a" 1}"#, "expected `:` at line 1 column 6"),
        (r#"{"a":1 "b":2}"#, "expected `,` or `}` at line 1 column 8"),
        ("01", "invalid number at line 1 column 2"),
        ("1.", "EOF while parsing a value at line 1 column 2"),
        ("1.e5", "invalid number at line 1 column 3"),
        ("1.\n", "invalid number at line 2 column 0"),
        ("1e", "EOF while parsing a value at line 1 column 2"),
        ("1e+x", "invalid number at line 1 column 4"),
        ("-", "EOF while parsing a value at line 1 column 1"),
        ("-a", "invalid number at line 1 column 2"),
        ("1e400", "number out of range at line 1 column 5"),
        ("1e99999999999", "number out of range at line 1 column 12"),
        ("tru", "EOF while parsing a value at line 1 column 3"),
        ("trux", "expected ident at line 1 column 4"),
        ("\"abc", "EOF while parsing a string at line 1 column 4"),
        (r#""\x""#, "invalid escape at line 1 column 3"),
        (r#""\u12""#, "EOF while parsing a string at line 1 column 6"),
        (r#""\u12zz""#, "invalid escape at line 1 column 7"),
        (
            "\"a\nb\"",
            "control character (\\u0000-\\u001F) found while parsing a string at line 2 column 0",
        ),
        ("1 2", "trailing characters at line 1 column 3"),
        ("[1\n,]", "trailing comma at line 2 column 2"),
        ("[", "EOF while parsing a list at line 1 column 1"),
        ("{", "EOF while parsing an object at line 1 column 1"),
        (r#"{"a""#, "EOF while parsing an object at line 1 column 4"),
        (r#"{"a":"#, "EOF while parsing a value at line 1 column 5"),
        ("[1,", "EOF while parsing a value at line 1 column 3"),
        ("]", "expected value at line 1 column 1"),
    ];
    for (text, message) in refused {
        assert_eq!(
            error_of(text).as_deref(),
            Some(message),
            "{text:?} is refused"
        );
        assert_eq!(
            json_oracle(text.as_bytes()).error.as_deref(),
            Some(message),
            "the oracle refuses {text:?} the same way"
        );
    }
}

#[test]
fn a_lone_surrogate_and_invalid_utf8_are_refused() {
    let refused: [(&[u8], &str); 5] = [
        (
            br#""\udc00""#,
            "lone leading surrogate in hex escape at line 1 column 7",
        ),
        (
            br#""\ud800""#,
            "unexpected end of hex escape at line 1 column 8",
        ),
        (
            b"\"\\ud800\\u0041\"",
            "lone leading surrogate in hex escape at line 1 column 13",
        ),
        (b"\"\xff\"", "invalid unicode code point at line 1 column 2"),
        (
            b"{\"k\xc3\":1}",
            "invalid unicode code point at line 1 column 4",
        ),
    ];
    for (bytes, message) in refused {
        let read = json_read(bytes, None);
        assert_eq!(read.error.as_deref(), Some(message), "{bytes:?} is refused");
        assert_eq!(read, json_oracle(bytes), "the oracle agrees on {bytes:?}");
    }
    let pair = json_read(b"\"\\ud83d\\ude00\"", None);
    assert_eq!(pair.tokens, [JsonToken::Str("\u{1F600}".to_owned())]);
}

#[test]
fn a_document_cut_at_any_byte_reads_the_same_from_chunks() {
    let documents: [&[u8]; 8] = [
        b"{\"name\":\"Ada\",\"tags\":[\"a\\n\\u00e9\",\"\\ud83d\\ude00\"],\"n\":-12.5e-3,\"ok\":true,\"none\":null}",
        b"[18446744073709551616, 0, -0, 9223372036854775807, 123456789012345678901234567890123]",
        b"  [ 1 ,\n 2 ,\r\n {\"a\" : [ ] } ]  ",
        b"[1,2,]",
        br#"{"a":"\ud800x"}"#,
        b"\"\xc3\xa9\xff\"",
        b"[1.5e99999999999]",
        b"{\"k\":tru}",
    ];
    for bytes in documents {
        let whole = json_read(bytes, None);
        for at in 0..=bytes.len() {
            assert_eq!(read_cut(bytes, at), whole, "{bytes:?} cut before byte {at}");
        }
        for size in 1..4 {
            assert_eq!(
                tokens(Reader::new(Cut::new(bytes, size), ceiling())),
                whole,
                "{bytes:?} in chunks of {size}"
            );
        }
    }
}

#[cfg_attr(
    miri,
    ignore = "reads a document 1023 levels deep, which Miri takes minutes for"
)]
#[test]
fn the_reader_reads_the_deepest_document_on_a_small_stack() {
    let open = LEVELS - 1;
    let mut text = String::new();
    for level in 0..open {
        text.push_str(if level % 2 == 0 { "[" } else { "{\"k\":" });
    }
    text.push('1');
    for level in (0..open).rev() {
        text.push(if level % 2 == 0 { ']' } else { '}' });
    }
    let deeper = format!("[{text}]");
    let (read, refused) = std::thread::Builder::new()
        .stack_size(64 << 10)
        .spawn(move || {
            let read = tokens(Reader::new(Text(text.as_bytes()), ceiling()));
            let refused = tokens(Reader::new(Text(deeper.as_bytes()), ceiling()));
            (read, refused)
        })
        .expect("a test thread is spawnable")
        .join()
        .expect("the reading thread does not panic");
    assert_eq!(read.error, None, "the ceiling is readable");
    assert_eq!(read.tokens.len(), 2 * open + (open / 2) + 1);
    assert_eq!(
        refused.error.as_deref(),
        Some("nesting is deeper than the `maxDepth` of 1024 at line 1 column 3068"),
    );
}

#[cfg_attr(miri, ignore = "reads every saved input, which Miri takes minutes for")]
#[test]
fn the_reader_and_the_oracle_agree_on_every_saved_json_seed() {
    let dir = nvs_repo::path("fuzz/seeds/json");
    let mut seeds: Vec<_> = std::fs::read_dir(&dir)
        .expect("fuzz/seeds/json is readable")
        .map(|entry| entry.expect("a readable seed").path())
        .collect();
    seeds.sort();
    let mut failed = Vec::new();
    for seed in &seeds {
        let bytes = std::fs::read(seed).expect("a readable seed");
        if let Some(why) = json_reader_disagreement(&bytes) {
            failed.push(format!("{}: {why}", seed.display()));
        }
    }
    assert!(seeds.len() > 100, "the saved seeds are found");
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}
