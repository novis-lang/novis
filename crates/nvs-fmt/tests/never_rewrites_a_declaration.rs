//! What a declaration keeps, as `rule:tooling/fmt-never-inserts-visibility` and
//! `rule:tooling/fmt-never-reorders-members` state it.
//!
//! Their own file rather than cases beside the base style's: both are rules
//! about what the formatter refuses to do to a class, and each is a claim that
//! goes on holding as later rules land. A member's own layout is the base
//! style's business; which members there are, in what order, and what each one
//! says it is, are the author's.

use nvs_diagnostics::SourceMap;
use nvs_fmt::format;

/// `source`, formatted, or a panic carrying the refusal if it does not parse.
fn formatted(source: &str) -> String {
    let mut map = SourceMap::new();
    let id = map.add("case.nvs".to_string(), source);
    format(map.file(id)).unwrap_or_else(|refusal| panic!("{refusal}"))
}

#[test]
fn no_visibility_keyword_is_ever_inserted() {
    // `rule:core-api/written-visibility` makes each of these a compile error,
    // and `nvs fmt` is not a compiler: a file that does not compile still does
    // not compile after it, and formatting on save never quietly writes the
    // `public` PHP would have implied.
    let source = "\
<?nvs
class Queue
{
    int $depth = 0;
    static string $name = 'queue';

    function drain(): void
    {
    }
}
";
    assert_eq!(formatted(source), source);
}

#[test]
fn class_members_keep_their_declaration_order() {
    // `rule:tooling/fmt-never-reorders-members`: declaration order is
    // observable — a derived codec encodes in it — so no grouping by kind or by
    // visibility happens here, however unconventional the order looks.
    let source = "\
<?nvs
class Report
{
    public function render(): string
    {
        return self::HEADER;
    }

    private int $rows = 0;

    public const string HEADER = 'rows';

    protected static function empty(): bool
    {
        return true;
    }
}
";
    assert_eq!(formatted(source), source);
}
