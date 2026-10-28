//! Source positions, source maps and diagnostics for Novis.
//!
//! Every later stage of the compiler depends on this crate and nothing else
//! depends on those stages, which keeps the dependency graph a line rather than
//! a web.
//!
//! ```
//! use nvs_diagnostics::{code, Diagnostic, Diagnostics, Renderer, SourceMap, Span};
//!
//! let mut map = SourceMap::new();
//! let file = map.add("greet.nvs", "<?nvs\necho $undefined;\n");
//!
//! let mut sink = Diagnostics::new();
//! sink.report(
//!     Diagnostic::error(code::E_UNDEFINED_VARIABLE, "undefined variable `$undefined`")
//!         .with_primary(Span::new(file, 11, 21), "not defined in this scope")
//!         .with_help("declare it before use, or check the spelling"),
//! );
//!
//! assert!(sink.has_errors());
//!
//! let mut out = Vec::new();
//! Renderer::new().render_all(sink.iter(), &map, &mut out).unwrap();
//! let text = String::from_utf8(out).unwrap();
//! assert!(text.contains("error[E0301]"));
//! assert!(text.contains("aborting due to 1 error"));
//! ```

pub mod argv;
mod diagnostic;
pub mod embedded;
pub mod record;
mod render;
mod source;
mod span;

pub use diagnostic::{Code, Diagnostic, Diagnostics, Label, LabelStyle, Severity, Suggestion};
pub use render::{AGENT_SHOW_LINE, Renderer};
pub use source::{
    MASK, MAX_SOURCE_LEN, PositionEncoding, SourceFile, SourceMap, canonical_key, plain_path,
};
pub use span::{BytePos, SourceId, Span, Spanned};

/// Declares one area of the [`code`] module: the range its codes take, and
/// each code with its card. Each file under `src/code/` is one invocation.
///
/// A declaration is the constant and its `Code::new(…)`, followed by
/// `.card("…")` once the code has its card
/// (`rule:tooling/a-diagnostic-code-carries-its-card`). The `.card` is read by
/// this macro and is not a method: the constant is the bare `Code`, and the
/// text is what [`code::card`] returns for it. A code without one is named in
/// the tests' list of codes still owing a card.
///
/// The macro writes the constants into the area's `declared` module, which
/// [`code`] re-exports whole, and the area's `AREA` entry, from which
/// [`code::ALL`] and [`code::card`] are built. Nobody else writes either, so a
/// code that lands is on the list at once and no hand-kept list can miss one —
/// which is what `nvs agent index` needs to print one line per code
/// (`rule:tooling/the-index-names-every-key-command-and-code`). It costs one
/// static slice of four-word entries, one of one-word entries and the cards'
/// text in the binary, per process and never per request.
macro_rules! codes {
    (
        range = $range:literal;
        $(
            $(#[$meta:meta])*
            pub const $name:ident: Code = Code::new($code:literal) $(.card($card:literal $(,)?))?;
        )*
    ) => {
        /// The constants this area declares, which [`crate::code`] re-exports.
        pub(super) mod declared {
            // Every other area's codes, so a doc comment here can link to a
            // code declared in another file.
            #[allow(unused_imports, reason = "read by intra-doc links alone")]
            use crate::code::*;

            $(
                $(#[$meta])*
                pub const $name: crate::Code = crate::Code::new($code);
            )*
        }

        /// This area's range, and its codes with their cards.
        pub(super) const AREA: super::Area = super::Area {
            range: $range,
            codes: &[$((declared::$name, codes!(@card $($card)?))),*],
        };
    };
    (@card) => {
        None
    };
    (@card $card:literal) => {
        Some($card)
    };
}

pub mod code;

#[cfg(test)]
mod tests {
    use super::{Code, code};

    /// The codes that still owe their card, in the order the `code` module
    /// declares them. It is empty, and [`no_code_still_owes_a_card`] keeps it
    /// so: a code lands with its card, as a `Core` member does, and is never
    /// added here.
    const CODES_STILL_OWING_A_CARD: &[&str] = &[];

    /// Every code carries its `rule:tooling/a-diagnostic-code-carries-its-card`
    /// card or is named in [`CODES_STILL_OWING_A_CARD`], and the two are held
    /// together both ways: a listed code that has its card, or that is not
    /// declared at all, fails as surely as an unlisted code without one. So
    /// the list only shrinks, and a code lands with its card as a `Core`
    /// member does.
    #[test]
    fn every_code_carries_its_card_or_is_listed() {
        let mut wrong = Vec::new();
        for &declared in code::ALL {
            let listed = CODES_STILL_OWING_A_CARD.contains(&declared.as_str());
            match (code::card(declared), listed) {
                (Some(card), false) if card.trim().is_empty() => {
                    wrong.push(format!("{declared}'s card is empty"));
                }
                (Some(_), true) => wrong.push(format!(
                    "{declared} now carries its card — delete it from `CODES_STILL_OWING_A_CARD`"
                )),
                (None, false) => wrong.push(format!("{declared} has no card")),
                (Some(_), false) | (None, true) => {}
            }
        }
        for owed in CODES_STILL_OWING_A_CARD {
            if !code::ALL.iter().any(|declared| declared.as_str() == *owed) {
                wrong.push(format!(
                    "{owed} is listed as owing a card but is not declared"
                ));
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    /// No declared code is without its card. With
    /// [`every_code_carries_its_card_or_is_listed`] this also holds
    /// [`CODES_STILL_OWING_A_CARD`] empty, since a listed code that has its
    /// card fails that test.
    #[test]
    fn no_code_still_owes_a_card() {
        let owing: Vec<&str> = code::ALL
            .iter()
            .filter(|&&declared| code::card(declared).is_none())
            .map(|declared| declared.as_str())
            .collect();
        assert!(
            owing.is_empty(),
            "these codes have no card: {}",
            owing.join(", ")
        );
    }

    /// Every `E01xx` number `code/parser.rs` declares, read out of its source.
    fn parser_band() -> Vec<u32> {
        include_str!("code/parser.rs")
            .split("Code::new(\"E01")
            .skip(1)
            .filter_map(|rest| rest.get(..2))
            .map(|digits| digits.parse::<u32>().expect("a code is four digits"))
            .collect()
    }

    /// The pipeline's three sit one past the highest code that predates them,
    /// not in the holes at `E0124`–`E0126`, which belong to other codes. That
    /// is what an allocation owes
    /// `conventions.md` § *A diagnostic*: the band's highest plus one, never
    /// its lowest hole. The claim is about where the three were allocated, so
    /// it holds whatever is allocated above them afterwards — which is the
    /// separate invariant the test below carries.
    #[test]
    fn the_pipeline_codes_are_the_next_free_parser_band_numbers() {
        let pipeline = [
            code::E_PIPELINE_RIGHT_SIDE_HAS_NO_HOLE,
            code::E_PIPELINE_RIGHT_SIDE_REPEATS_THE_HOLE,
            code::E_HOLE_OUTSIDE_A_PIPELINE,
        ];
        assert_eq!(pipeline.map(Code::as_str), ["E0129", "E0130", "E0131"]);

        assert_eq!(
            parser_band().into_iter().filter(|n| *n < 29).max(),
            Some(28),
            "the pipeline's three no longer follow on from the band's highest, \
             so they read as filling a hole rather than as the next free numbers",
        );
    }

    /// The band's highest number is the newest code in it, which is what makes
    /// "the next free one is the highest plus one" answerable by reading the
    /// registry. A code allocated into a hole leaves this test failing.
    #[test]
    fn the_newest_parser_code_is_the_bands_highest_number() {
        assert_eq!(code::E_METHOD_REF_DROPPED.as_str(), "E0137");

        let mut band = parser_band();
        assert!(band.len() > 3, "the band did not parse: {band:?}");
        band.sort_unstable();
        let mut unique = band.clone();
        unique.dedup();
        assert_eq!(band, unique, "two parser-band codes share a number");
        assert_eq!(
            band.pop(),
            Some(37),
            "the newest code is no longer the band's highest, so the number after \
             it is no longer the next free one",
        );
    }

    #[test]
    fn the_for_and_catch_codes_still_belong_to_their_owners() {
        assert_eq!(code::E_FOR_INIT_MIXES_DECL_AND_EXPR.as_str(), "E0124");
        assert_eq!(code::E_FOR_INIT_TWO_DECLARATIONS.as_str(), "E0125");
        assert_eq!(code::E_CATCH_ARM_NOT_AN_EXPRESSION.as_str(), "E0126");
    }
}
