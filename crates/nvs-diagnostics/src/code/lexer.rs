//! The lexer's codes, `E00xx`.
//!
//! A new code here takes the highest number in this file plus one, never a
//! hole: a hole is a retired number. A code outside `E00xx` fails the
//! crate's tests.

codes! {
    range = "E00";

    /// A character that cannot begin any token.
    pub const E_UNEXPECTED_CHAR: Code = Code::new("E0001").card(
        "The file contains a character that is not part of Novis syntax. \
         Delete it, or put it inside a string or a comment.",
    );
    /// A string, comment or heredoc that runs to end of file.
    pub const E_UNTERMINATED: Code = Code::new("E0002").card(
        "A string, comment or heredoc starts here and is never closed, so it runs to the end \
         of the file. Add the closing quote, the `*/` or the heredoc's closing label.",
    );
    /// A numeric literal the lexer cannot interpret.
    pub const E_INVALID_NUMBER: Code = Code::new("E0003").card(
        "This number is not written correctly. Check that every digit is allowed in its base: \
         a `0b` number uses only `0` and `1`, and a `0x` number needs at least one digit.",
    );
    /// A `\u{...}` escape with no hex digit or no closing `}`. Every other
    /// backslash sequence is left as it is written.
    pub const E_INVALID_ESCAPE: Code = Code::new("E0004").card(
        "A `\\u{...}` escape in this string needs one or more hex digits and a closing `}`, \
         such as `\\u{1F600}`. Write `\\\\u` for a backslash followed by the letter `u`.",
    );
    /// A heredoc whose closing identifier is missing or misindented.
    pub const E_BAD_HEREDOC: Code = Code::new("E0005").card(
        "The closing label of this heredoc is missing, or it is indented more than a line of \
         the heredoc's text. Put the label on a line of its own, indented no further than the text.",
    );
    /// Input that is not valid UTF-8.
    pub const E_INVALID_UTF8: Code = Code::new("E0006").card(
        "The file is not valid UTF-8 text. Save it again with the UTF-8 encoding.",
    );
    /// A duration such as `1h30m` that does not follow
    /// `rule:types/duration`'s grammar —
    /// out of order, a repeated unit, a fractional count, or longer than
    /// `Core\Time\Duration` can hold. A unit written in the wrong case is a
    /// spelling rather than a shape, and is `E_RESERVED_SPELLING_CASE`.
    pub const E_BAD_DURATION: Code = Code::new("E0007").card(
        "This duration is not written correctly, or it is too long to store. Write a whole \
         number before each unit, and write the units from the largest to the smallest, each \
         unit once. For example, write `1h30m`, not `30m1h` or `1.5h`.",
    );
    /// A bidirectional control that opens a directional scope and never closes
    /// it inside the source span that opened it, per
    /// `rule:security/bidi-boundaries`
    /// — a comment, a string literal or an inline-HTML run, and each line
    /// of a multi-line one. There is no suppression.
    pub const E_UNBALANCED_BIDI: Code = Code::new("E0008").card(
        "A comment, string or HTML text contains a bidirectional control character that is \
         never closed. This character changes the direction that text is shown in, so the code \
         can look different from what runs. Delete it, or close it before the line ends.",
    );
    /// An `<?nvs` open tag in a file that opens with `#!` and is therefore
    /// already in code mode, before any `?>` has left it, per
    /// `rule:tooling/shebang-opens-code-mode`
    /// . Reported by `nvs_syntax`'s lexer, which consumes the tag and keeps
    /// lexing code rather than leaving `<` `?` `nvs` for the parser.
    pub const E_TAG_IN_SHEBANG_FILE: Code = Code::new("E0009").card(
        "This file starts with `#!`, so it is Novis code from the first line. \
         Delete this `<?nvs` tag.",
    );
    /// A `<?nvs` open tag inside an html template, whose body is one expression
    /// and has no code mode to enter (`rule:core-classes/html-template`); the
    /// output tag `<?= … ?>` is the only hole a template has.
    pub const E_CODE_BLOCK_IN_HTML_TEMPLATE: Code = Code::new("E0010").card(
        "An html template cannot contain a `<?nvs` code block. \
         Write `<?= … ?>` to put the value of one expression into the template.",
    );

    /// A string literal whose text contains the `ESC` control byte, written raw
    /// or as an escape (`\e`, `\x1b`, `\033`, `\u{1b}`), per
    /// `rule:tooling/an-escape-in-source-text-is-a-compile-error`. Reported by
    /// `nvs_types`'s checker, at the one character or escape that writes it.
    pub const E_ESCAPE_CHARACTER_IN_STRING: Code = Code::new("E0011").card(
        "This string contains the `ESC` character, which starts a terminal escape sequence. \
         A program cannot write terminal escape sequences in its source code. \
         To add colour or style to terminal output, use `Core\\Cli\\Text`.",
    );
}
