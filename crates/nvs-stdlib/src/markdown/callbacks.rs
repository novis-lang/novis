//! `rule:core-classes/markdown-to-html`'s four callbacks: what a program
//! changes about the links, images, headings and code blocks of a document.
//!
//! [`Callbacks`] is the renderer's side, so the renderer is tested with plain
//! Rust closures and runs with no callbacks at the cost of a call that returns
//! [`Answer::Default`]. [`Callables`] is the program's side: each method calls
//! the program's callable through `nvs_runtime::call_callable`.
//!
//! # A `code` result is written as it is
//!
//! A `Core\Html\Markup` is trusted by its type, so the rebuild a document's
//! raw HTML takes must not reach it: a highlighter's `<span style>` would be
//! removed. The renderer writes a [`Trusted`] token in the block's place, a
//! text node of a random 128-bit prefix, the fragment's index and U+E001. The
//! document cannot write the prefix, because it is drawn for each render. The
//! tree is serialized, and rebuilt when it must be, and [`Trusted::spliced`]
//! then writes each fragment where its token is.
//!
//! **What it spends:** each fragment until the render ends, and one pass over
//! the answer to splice them. A render with no `code` result spends nothing.

use nvs_runtime::{Ctx, Fault, Tag, Value};

use crate::html::markup_slot;

/// What a callback returned.
pub(crate) enum Answer {
    /// The program passed no callback, or the callback did not run.
    Default,
    /// The callback returned `null`.
    Null,
    /// The callback returned this string, or this `Markup`'s source.
    Value(String),
}

/// The renderer's view of the four callbacks. Each default is what a call
/// with no callbacks renders.
pub(crate) trait Callbacks {
    /// The new URL for the link to `url`.
    fn link(&mut self, _url: &str) -> Result<Answer, Fault> {
        Ok(Answer::Default)
    }

    /// The new URL for the image at `url`.
    fn image(&mut self, _url: &str) -> Result<Answer, Fault> {
        Ok(Answer::Default)
    }

    /// Whether [`Self::heading`] runs, so the renderer reads a heading's text
    /// only for a program that needs it.
    fn wants_heading(&self) -> bool {
        false
    }

    /// The `id` after `md-` for a heading of `level` whose text is `text`.
    fn heading(&mut self, _level: usize, _text: &str) -> Result<Answer, Fault> {
        Ok(Answer::Default)
    }

    /// Whether [`Self::code`] runs, so the renderer keeps a block's language
    /// only for a program that needs it.
    fn wants_code(&self) -> bool {
        false
    }

    /// The HTML for a fenced block in `language` whose text is `code`.
    fn code(&mut self, _language: &str, _code: &str) -> Result<Answer, Fault> {
        Ok(Answer::Default)
    }
}

/// No callbacks: what `toHtml` renders when the program passes none.
pub(crate) struct NoCallbacks;

impl Callbacks for NoCallbacks {}

/// The program's callables, each `None` when it passed none.
pub(crate) struct Callables<'c> {
    pub ctx: &'c mut Ctx,
    pub link: Option<Value>,
    pub image: Option<Value>,
    pub heading: Option<Value>,
    pub code: Option<Value>,
}

/// `value`, or `None` when it is `null`: an omitted callback.
pub(crate) fn given(value: Value) -> Option<Value> {
    (value.tag() != Some(Tag::Null)).then_some(value)
}

impl Callables<'_> {
    /// `callable` called with `texts` and then `int`, if one is given, and its
    /// result read as a string or a `Markup`.
    fn call(
        &mut self,
        callable: Option<Value>,
        int: Option<i64>,
        texts: &[&str],
    ) -> Result<Answer, Fault> {
        let Some(callable) = callable else {
            return Ok(Answer::Default);
        };
        let mut args = Vec::with_capacity(3);
        args.extend(int.map(Value::int));
        let owned = args.len();
        args.extend(texts.iter().map(|text| Value::text(text)));
        let answer = nvs_runtime::call_callable(self.ctx, callable, &args);
        for arg in &args[owned..] {
            #[expect(
                unsafe_code,
                reason = "each text argument is a fresh value this frame owns, and \
                          `call_callable` took its own reference"
            )]
            // SAFETY: as above, and nothing reads `arg` after this.
            unsafe {
                arg.release();
            }
        }
        let answer = answer?;
        let read = if answer.tag() == Some(Tag::Null) {
            Ok(Answer::Null)
        } else if let Some(text) = answer.as_text() {
            Ok(Answer::Value(text.to_owned()))
        } else {
            markup_slot(answer, r"the `Markup` a `Core\Markdown` callback returned")
                .map(|slot| Answer::Value(slot.as_text().unwrap_or_default().to_owned()))
        };
        #[expect(
            unsafe_code,
            reason = "`call_callable` answered with a reference this frame owns"
        )]
        // SAFETY: as above, and `read` holds a copy of its text.
        unsafe {
            answer.release();
        }
        read
    }
}

impl Callbacks for Callables<'_> {
    fn link(&mut self, url: &str) -> Result<Answer, Fault> {
        self.call(self.link, None, &[url])
    }

    fn image(&mut self, url: &str) -> Result<Answer, Fault> {
        self.call(self.image, None, &[url])
    }

    fn wants_heading(&self) -> bool {
        self.heading.is_some()
    }

    fn heading(&mut self, level: usize, text: &str) -> Result<Answer, Fault> {
        let level = i64::try_from(level).expect("a heading's level is 1 to 6");
        self.call(self.heading, Some(level), &[text])
    }

    fn wants_code(&self) -> bool {
        self.code.is_some()
    }

    fn code(&mut self, language: &str, code: &str) -> Result<Answer, Fault> {
        self.call(self.code, None, &[language, code])
    }
}

/// The ends of a [`Trusted`] token, U+E000 before and U+E001 after.
const OPEN: char = '\u{E000}';
const CLOSE: char = '\u{E001}';

/// The `code` results of one render, and the token each is written as.
#[derive(Default)]
pub(crate) struct Trusted {
    /// U+E000 and 32 random hex digits, drawn at the first fragment.
    prefix: String,
    fragments: Vec<String>,
}

impl Trusted {
    /// The token that stands where `html` is written.
    pub(crate) fn token(&mut self, html: String) -> String {
        if self.prefix.is_empty() {
            let mut bytes = [0_u8; 16];
            {
                use rand::Rng as _;
                rand::rng().fill_bytes(&mut bytes);
            }
            self.prefix.push(OPEN);
            for byte in bytes {
                self.prefix.push_str(&format!("{byte:02x}"));
            }
        }
        self.fragments.push(html);
        format!("{}{}{CLOSE}", self.prefix, self.fragments.len() - 1)
    }

    /// `out` with each token replaced by its fragment. A token the rebuild
    /// dropped, such as one inside a raw `<script>`, writes nothing.
    pub(crate) fn spliced(self, out: String) -> String {
        if self.fragments.is_empty() {
            return out;
        }
        let extra: usize = self.fragments.iter().map(String::len).sum();
        let mut result = String::with_capacity(out.len() + extra);
        let mut rest = out.as_str();
        while let Some(at) = rest.find(&self.prefix) {
            result.push_str(&rest[..at]);
            let after = &rest[at + self.prefix.len()..];
            let digits = after.bytes().take_while(u8::is_ascii_digit).count();
            let fragment = after[digits..]
                .starts_with(CLOSE)
                .then(|| after[..digits].parse::<usize>().ok())
                .flatten()
                .and_then(|index| self.fragments.get(index));
            if let Some(fragment) = fragment {
                result.push_str(fragment);
                rest = &after[digits + CLOSE.len_utf8()..];
            } else {
                result.push_str(&self.prefix);
                rest = after;
            }
        }
        result.push_str(rest);
        result
    }
}
