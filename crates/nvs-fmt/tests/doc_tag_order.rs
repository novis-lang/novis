//! The order a doc comment's tags go out in, as
//! `rule:tooling/fmt-orders-the-doc-tags` states it, and the doc tag errors
//! that do not stop the formatter.
//!
//! Each case is one file and the canonical text it formats to, as in
//! `use_block.rs`, so a failure prints the whole disagreement.

use nvs_diagnostics::SourceMap;
use nvs_fmt::format;

/// `source`, formatted, or a panic carrying the refusal if it does not parse.
fn formatted(source: &str) -> String {
    let mut map = SourceMap::new();
    let id = map.add("case.nvs".to_string(), source);
    format(map.file(id)).unwrap_or_else(|refusal| panic!("{refusal}"))
}

/// A class whose one method carries `doc`, its lines already indented.
fn method(doc: &str) -> String {
    format!(
        "\
<?nvs
final class Payments
{{
{doc}    public static function charge(int $cents, string $card): string
    {{
        return 'ch-' . $cents;
    }}
}}
"
    )
}

#[test]
fn the_tags_follow_the_prose_in_one_order() {
    let source = method(
        "    /// @example docs/examples/charge.nvs
    /// @see Payments::refund
    /// @throws CardDeclined when the bank declines the card
    /// Charges a card.
    /// @return the id of the charge
    /// @param $cents the amount, in cents
    /// @param $card the card number
",
    );
    // A line after a tag is that tag's text, so `Charges a card.` stays with
    // `@throws`. The prose here is the comment's first line, which is a tag.
    let ordered = method(
        "    /// @param $cents the amount, in cents
    /// @param $card the card number
    /// @return the id of the charge
    /// @throws CardDeclined when the bank declines the card
    /// Charges a card.
    /// @see Payments::refund
    /// @example docs/examples/charge.nvs
",
    );
    assert_eq!(formatted(&source), ordered);

    let source = method(
        "    /// Charges a card.
    /// @see Payments::refund
    /// @see Payments::history
    ///
    /// @return the id of the charge
",
    );
    let ordered = method(
        "    /// Charges a card.
    ///
    /// @return the id of the charge
    /// @see Payments::refund
    /// @see Payments::history
",
    );
    assert_eq!(formatted(&source), ordered);
}

#[test]
fn params_follow_the_signature() {
    let source = method(
        "    /// Charges a card.
    ///
    /// @param $card the card number
    /// @param $cents the amount, in cents
",
    );
    let ordered = method(
        "    /// Charges a card.
    ///
    /// @param $cents the amount, in cents
    /// @param $card the card number
",
    );
    assert_eq!(formatted(&source), ordered);
}

#[test]
fn an_unknown_param_goes_after_the_known_ones() {
    let source = method(
        "    /// Charges a card.
    ///
    /// @param $amount the old name of the amount
    /// @param $card the card number
    /// @param $currency a parameter that is gone
    /// @param $cents the amount, in cents
",
    );
    let ordered = method(
        "    /// Charges a card.
    ///
    /// @param $cents the amount, in cents
    /// @param $card the card number
    /// @param $amount the old name of the amount
    /// @param $currency a parameter that is gone
",
    );
    assert_eq!(formatted(&source), ordered);
}

#[test]
fn a_tag_keeps_its_continuation_lines() {
    let source = method(
        "    /// Charges a card.
    ///
    /// @return the id of the charge,
    ///     such as `ch-1250`
    /// @param $card the card number,
    ///   written without spaces
    /// @param $cents the amount, in cents
",
    );
    let ordered = method(
        "    /// Charges a card.
    ///
    /// @param $cents the amount, in cents
    /// @param $card the card number,
    ///   written without spaces
    /// @return the id of the charge,
    ///     such as `ch-1250`
",
    );
    assert_eq!(formatted(&source), ordered);
}

#[test]
fn ordering_the_tags_is_idempotent() {
    let source = method(
        "    /// Charges a card.
    /// @throws CardDeclined when the bank declines the card
    ///
    ///
    /// @param $card the card number
    /// @return the id of the charge
    /// @param $cents the amount, in cents
",
    );
    let once = formatted(&source);
    assert_eq!(formatted(&once), once);
    assert_eq!(
        once,
        method(
            "    /// Charges a card.
    ///
    /// @param $cents the amount, in cents
    /// @param $card the card number
    /// @return the id of the charge
    /// @throws CardDeclined when the bank declines the card
"
        )
    );
}

#[test]
fn a_doc_tag_error_does_not_stop_formatting() {
    // `@param` with no text, a type before the name, and `@returns`: each is a
    // doc tag error the parse reports, and the file is still formatted.
    // `@returns` is not one of the five tags, so its line is prose and stays
    // above the tags.
    let source = method(
        "    /// Charges a card.
    ///
    /// @returns the id of the charge
    /// @param $card
    /// @param int $cents the amount, in cents
",
    )
    .replace("return 'ch-'", "return  'ch-'");
    let laid_out = method(
        "    /// Charges a card.
    ///
    /// @returns the id of the charge
    ///
    /// @param int $cents the amount, in cents
    /// @param $card
",
    );
    assert_eq!(formatted(&source), laid_out);
}
