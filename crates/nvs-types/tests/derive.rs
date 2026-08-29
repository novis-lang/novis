//! ADR 0071 §§ 2 and 7's three refusals: a field whose declared type has no
//! wire form, a class that hand-writes both codec halves, and a class whose
//! derive would generate an empty contract.
//!
//! All three are diagnostics at the **declaration** — the property that wrote
//! the type, or the attribute that has no effect — rather than at the
//! `Core\Json::decodeAs<T>` that would later fail on the same class. The
//! reachability half is asked after the whole program has been walked
//! (`nvs_types::derive::resolve_field_types`), so the order the files declare
//! two classes in cannot change the answer; the fixture below that names a
//! class declared *after* the deriving one is what pins that.

mod common;

use common::check_src;
use nvs_diagnostics::{Code, Diagnostics, code};

/// Whether `diags` reported `want`. By code rather than by `has_errors`, for
/// `routes.rs`'s reason: a fixture written to trip one rule routinely trips a
/// second, and "some error was reported" is satisfied by the wrong one.
fn reported(diags: &Diagnostics, want: Code) -> bool {
    diags.iter().any(|d| d.code == Some(want))
}

#[test]
fn a_derive_field_whose_type_has_no_codec_is_refused_where_it_is_declared() {
    // A class with no codec of its own. `Handle` is declared *after* the class
    // that names it, which is the case a check made where the property is
    // walked would get wrong.
    let diags = check_src(
        "<?nvs
#[Core\\Json\\Derive]
class Row {
    public Handle $h;
    public function constructor(Handle $h) { $this->h = $h; }
}
class Handle {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
}
",
    );
    assert!(
        reported(&diags, code::E_DERIVE_FIELD_NOT_CODEC_REACHABLE),
        "{diags:?}"
    );

    // The same file with one attribute added: § 2's "another class that itself
    // has a codec", still declared second.
    let diags = check_src(
        "<?nvs
#[Core\\Json\\Derive]
class Row {
    public Handle $h;
    public function constructor(Handle $h) { $this->h = $h; }
}
#[Core\\Json\\Derive]
class Handle {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");

    // § 7's hand-written half is a codec too — one declared half is enough,
    // since the derive generates the other.
    let diags = check_src(
        "<?nvs
#[Core\\Json\\Derive]
class Row {
    public Handle $h;
    public function constructor(Handle $h) { $this->h = $h; }
}
class Handle {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
    public function toJson(): mixed { return $this->n; }
}
",
    );
    assert!(
        !reported(&diags, code::E_DERIVE_FIELD_NOT_CODEC_REACHABLE),
        "{diags:?}"
    );

    // `bytes` is not a `string` (ADR 0009) and JSON has no spelling for it.
    let diags = check_src(
        "<?nvs
#[Core\\Json\\Derive]
class Blob {
    public bytes $b;
    public function constructor(bytes $b) { $this->b = $b; }
}
",
    );
    assert!(
        reported(&diags, code::E_DERIVE_FIELD_NOT_CODEC_REACHABLE),
        "{diags:?}"
    );

    // The scalars, `?T` of one, and an `array<T>` of one: § 2's list, none of
    // which this refusal may reach. A `?int` is reachable exactly when `int`
    // is, which is what makes the check ask the null-stripped type.
    let diags = check_src(
        "<?nvs
#[Core\\Json\\Derive]
class Wide {
    public string $name;
    public ?int $rank;
    public bool $live;
    public float $ratio;
    public mixed $extra;
    public array<string> $tags;
    public function constructor(
        string $name,
        ?int $rank,
        bool $live,
        float $ratio,
        mixed $extra,
        array<string> $tags,
    ) {
        $this->name = $name;
        $this->rank = $rank;
        $this->live = $live;
        $this->ratio = $ratio;
        $this->extra = $extra;
        $this->tags = $tags;
    }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_class_that_hand_writes_both_codec_halves_is_refused() {
    // § 7: the attribute generates nothing, and an attribute with no effect is
    // a mistake rather than a no-op.
    let diags = check_src(
        "<?nvs
#[Core\\Json\\Derive]
class Row {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
    public function toJson(): mixed { return $this->n; }
    public static function fromJson(mixed $value): Row { return new Row(0); }
}
",
    );
    assert!(reported(&diags, code::E_DERIVE_BOTH_HALVES), "{diags:?}");

    // One half is the common real case — a custom representation with a
    // mechanical inverse — and it is not refused, in either direction.
    let diags = check_src(
        "<?nvs
#[Core\\Json\\Derive]
class Row {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
    public function toJson(): mixed { return $this->n; }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");

    let diags = check_src(
        "<?nvs
#[Core\\Json\\Derive]
class Row {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
    public static function fromJson(mixed $value): Row { return new Row(0); }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");

    // Both halves and no attribute is an ordinary class, and this pass has
    // nothing to say about it.
    let diags = check_src(
        "<?nvs
class Row {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
    public function toJson(): mixed { return $this->n; }
    public static function fromJson(mixed $value): Row { return new Row(0); }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_derive_on_a_class_with_no_declared_properties_is_refused() {
    // § 2's field list is the declared property list, so no property is an
    // empty wire contract.
    let diags = check_src("<?nvs\n#[Core\\Json\\Derive]\nclass Marker {}\n");
    assert!(reported(&diags, code::E_DERIVE_NO_FIELDS), "{diags:?}");

    // Reached the other way: every declared property skipped in writing
    // leaves the same empty contract behind.
    let diags = check_src(
        "<?nvs
#[Core\\Json\\Derive]
class Hidden {
    #[Core\\Json\\Field(skip: true)]
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
}
",
    );
    assert!(reported(&diags, code::E_DERIVE_NO_FIELDS), "{diags:?}");

    // A `static` property is class storage, not instance storage (ADR 0008),
    // so it is not a field either — and a class holding only one derives the
    // same empty contract.
    let diags = check_src(
        "<?nvs
#[Core\\Json\\Derive]
class Counter {
    public static int $seen = 0;
}
",
    );
    assert!(reported(&diags, code::E_DERIVE_NO_FIELDS), "{diags:?}");

    // One declared property is a contract, and nothing is reported.
    let diags = check_src(
        "<?nvs
#[Core\\Json\\Derive]
class Row {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");

    // A class with no property and no attribute is not this pass's business:
    // ADR 0071 § 8's "a program with no derive attribute pays nothing at all".
    let diags = check_src("<?nvs\nclass Marker {}\n");
    assert!(!diags.has_errors(), "{diags:?}");
}
