//! `rule:core-classes/derive-field-list` and `rule:core-classes/derive-generates-what-is-missing`'s three refusals: a field whose declared type has no
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

use common::{check_src, check_src_declared, check_src_table};
use nvs_diagnostics::{Code, Diagnostics, code};
use nvs_stdlib::CodecTy;
use nvs_types::derive;
use nvs_types::enums::EnumTable;
use nvs_types::expr_table::ExprInfo;
use nvs_types::ty::{ShapeField, TypeInterner};

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

    // `bytes` is not a `string` (`rule:types/bytes`) and JSON has no spelling for it.
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

    // A `static` property is class storage, not instance storage (`rule:statements/static-is-a-member-modifier`),
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
    // `rule:core-classes/derive-generates-what-is-missing`'s "a program with no derive attribute pays nothing at all".
    let diags = check_src("<?nvs\nclass Marker {}\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn db_derive_and_db_field_are_on_the_attribute_roster() {
    // `rule:core-classes/derive-attribute`'s table is `ATTRIBUTES`' one home, and these are the two
    // rows it has always carried that the compiler did not.
    assert!(
        derive::ATTRIBUTES.contains(&derive::DB_DERIVE),
        "no `Db\\Derive`"
    );
    assert!(
        derive::ATTRIBUTES.contains(&derive::DB_FIELD),
        "no `Db\\Field`"
    );

    // Matched *nominally* after `nvs_hir::resolve_ref`, so the `use`d short
    // form and the qualified spelling are one attribute reached two ways —
    // and `#[Db\Field]` is read on the property under either. Neither name is
    // a declared shape alias, which is what § 1's carve-out is for.
    let diags = check_src(
        "<?nvs
use Core\\Db\\Derive;
use Core\\Db\\Field;

#[Derive]
class Row {
    #[Field(name: \"email_address\")]
    public tainted string $email;
    public function constructor(tainted string $email) { $this->email = $email; }
}

#[Core\\Db\\Derive]
class Qualified {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");

    // The refusals of §§ 2 and 7 reach the second format unchanged: a class
    // with no declared property derives an empty row mapping.
    let diags = check_src("<?nvs\n#[Core\\Db\\Derive]\nclass Marker {}\n");
    assert!(reported(&diags, code::E_DERIVE_NO_FIELDS), "{diags:?}");

    // A userland alias that happens to be spelled `Derive` resolves to a
    // different `QName` and is not this attribute — the same class is silent.
    let diags = check_src("<?nvs\ntype Derive = {};\n#[Derive]\nclass Marker {}\n");
    assert!(!diags.has_errors(), "{diags:?}");

    // § 7 one-directionally: `Core\\Db\\Codec` declares `fromRow` alone, so a
    // class that writes it has left the attribute nothing to generate.
    let diags = check_src(
        "<?nvs
#[Core\\Db\\Derive]
class Own {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
    public static function fromRow(mixed $row): static { return new Own(1); }
}
",
    );
    assert!(reported(&diags, code::E_DERIVE_BOTH_HALVES), "{diags:?}");
}

#[test]
fn a_db_derive_field_whose_type_has_no_column_mapping_is_refused_where_declared() {
    // `rule:core-classes/db-column-types`'s type map has no row for an object: a row is a flat list
    // of columns, so a nested class is refused at the property that declared
    // it even when that class carries a `#[Db\Derive]` of its own. This is
    // where the two formats' maps first disagree — § 2 admits exactly this
    // field for JSON.
    let nested = "<?nvs
#[Core\\{0}\\Derive]
class Row {
    public Handle $h;
    public function constructor(Handle $h) { $this->h = $h; }
}
#[Core\\{0}\\Derive]
class Handle {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
}
";
    let diags = check_src(&nested.replace("{0}", "Db"));
    assert!(
        reported(&diags, code::E_DERIVE_FIELD_NOT_CODEC_REACHABLE),
        "{diags:?}"
    );
    let diags = check_src(&nested.replace("{0}", "Json"));
    assert!(!diags.has_errors(), "{diags:?}");

    // And they disagree the other way round on `bytes`, which is a `BLOB`
    // column and has no JSON spelling at all (`rule:types/bytes`). `decimal` and § 9's
    // date and time classes are columns too.
    let scalars = "<?nvs
#[Core\\{0}\\Derive]
class Row {
    public bytes $blob;
    public decimal $total;
    public Core\\Time\\Instant $at;
    public function constructor(bytes $blob, decimal $total, Core\\Time\\Instant $at)
    {
        $this->blob = $blob;
        $this->total = $total;
        $this->at = $at;
    }
}
";
    let diags = check_src(&scalars.replace("{0}", "Db"));
    assert!(!diags.has_errors(), "{diags:?}");
    let diags = check_src(&scalars.replace("{0}", "Json"));
    assert!(
        reported(&diags, code::E_DERIVE_FIELD_NOT_CODEC_REACHABLE),
        "{diags:?}"
    );

    // The refusal is at the declaration and not at the `queryAs<T>` that would
    // later run, so § 3's escape hatch takes the same property off the mapping
    // and the file compiles — with the *row* attribute, not the JSON one.
    let diags = check_src(
        "<?nvs
#[Core\\Db\\Derive]
class Row {
    #[Core\\Db\\Field(skip: true)]
    public Handle $cache;
    public int $n;
    public function constructor(Handle $cache, int $n)
    {
        $this->cache = $cache;
        $this->n = $n;
    }
}
class Handle {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn the_json_and_db_derives_share_one_pass() {
    // One class carrying both attributes has two contracts, read off one walk
    // of its members — § 3's two `Field` spellings are independent overrides
    // of the same property, which is why they are two attributes at all.
    let (diags, exprs) = check_src_table(
        "<?nvs
#[Core\\Json\\Derive]
#[Core\\Db\\Derive]
class Row {
    #[Core\\Json\\Field(name: \"email_address\")]
    #[Core\\Db\\Field(name: \"email\")]
    public tainted string $address;
    public int $n;
    public function constructor(tainted string $address, int $n)
    {
        $this->address = $address;
        $this->n = $n;
    }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    let keys = |codec: Option<&derive::DerivedCodec>| {
        codec
            .expect("the class derives this format")
            .fields
            .iter()
            .map(|field| field.key.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(keys(exprs.codec("Row")), ["email_address", "n"]);
    assert_eq!(keys(exprs.db_codec("Row")), ["email", "n"]);

    // Every rule is stated once and asked of both, so one broken property is
    // reported once per contract — each naming the attribute that is wrong
    // about it. A second pass agreeing with the first is what this pins
    // against.
    let diags = check_src(
        "<?nvs
#[Core\\Json\\Derive]
#[Core\\Db\\Derive]
class Late {
    public lateinit int $n;
    public function constructor() {}
}
",
    );
    let named: Vec<&str> = diags
        .iter()
        .filter(|d| d.code == Some(code::E_DERIVE_LATEINIT_FIELD))
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(named.len(), 2, "{diags:?}");
    assert!(
        named.iter().any(|m| m.contains(r"#[Json\Derive]")),
        "{named:?}"
    );
    assert!(
        named.iter().any(|m| m.contains(r"#[Db\Derive]")),
        "{named:?}"
    );

    // § 3's option roster reaches the second format too, and the message names
    // the attribute the reader wrote.
    let diags = check_src(
        "<?nvs
#[Core\\Db\\Derive]
class Row {
    #[Core\\Db\\Field(rename: \"n\")]
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
}
",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_DERIVE_FIELD_ATTRIBUTE)
                && d.message.contains(r"#[Db\Field]")),
        "{diags:?}"
    );
}

/// The three ways `rule:core-classes/db-column-types`'s map answers "no" for a written `T`, all of
/// them at the call — `nvs_types::derive::check_row_sites` is the pass, and
/// its doc owns why the third cannot move to the declaration.
#[test]
fn query_as_over_a_class_with_no_db_derive_is_refused_at_the_call() {
    // A plain class is well formed and is not a row mapping. The refusal is at
    // the `queryAs`, not at the class, because the class is wrong for nothing
    // else it does.
    let plain = check_src(
        "<?nvs
class Person {
    public uint $id;
    public function constructor(uint $id) { $this->id = $id; }
}
class People {
    public static function everyone(Core\\Db\\Transaction $tx): Core\\Db\\Rows<Person>
    {
        return $tx->queryAs<Person>(\"select id from people\", []);
    }
}
",
    );
    assert!(
        plain
            .iter()
            .any(|d| d.code == Some(code::E_QUERY_AS_NOT_A_ROW_CLASS)),
        "{plain:?}"
    );

    // The near miss, and the reason the message names the attribute rather
    // than "a codec": the document half is a different map over different
    // sources, and a class carrying only it has nothing that reads columns.
    let json_only = check_src(
        "<?nvs
#[Core\\Json\\Derive]
class Person {
    public uint $id;
    public function constructor(uint $id) { $this->id = $id; }
}
class People {
    public static function everyone(Core\\Db\\Transaction $tx): Core\\Db\\Rows<Person>
    {
        return $tx->queryAs<Person>(\"select id from people\", []);
    }
}
",
    );
    assert!(
        json_only
            .iter()
            .any(|d| d.code == Some(code::E_QUERY_AS_NOT_A_ROW_CLASS)),
        "{json_only:?}"
    );
}

#[test]
fn query_as_over_a_list_form_is_refused_at_the_call() {
    // `Core\Json::decodeAs<array<T>>` is a JSON array document and is the
    // reason `written_class_of` reads the shape at all; `rule:core-classes/db-statement-members`'s member
    // already answers `Rows` of one, so the same spelling here asks for the
    // plural twice.
    let listed = check_src(
        "<?nvs
#[Core\\Db\\Derive]
class Person {
    public uint $id;
    public function constructor(uint $id) { $this->id = $id; }
}
class People {
    public static function everyone(Core\\Db\\Transaction $tx): Core\\Db\\Rows<array<Person>>
    {
        return $tx->queryAs<array<Person>>(\"select id from people\", []);
    }
}
",
    );
    assert!(
        listed
            .iter()
            .any(|d| d.code == Some(code::E_QUERY_AS_NOT_A_ROW_CLASS)),
        "{listed:?}"
    );

    // The same call written the one way that is a row, asserted beside it so
    // that a rule refusing every `queryAs` fails here.
    let single = check_src(
        "<?nvs
#[Core\\Db\\Derive]
class Person {
    public uint $id;
    public function constructor(uint $id) { $this->id = $id; }
}
class People {
    public static function everyone(Core\\Db\\Transaction $tx): Core\\Db\\Rows<Person>
    {
        return $tx->queryAs<Person>(\"select id from people\", []);
    }
}
",
    );
    assert!(!single.has_errors(), "{single:?}");
}

#[test]
fn query_as_over_a_class_with_an_opaque_field_is_refused_at_the_call() {
    // `rule:core-classes/derive-field-list`'s escape hatch takes a property off the mapping, and the
    // class stays well formed — `a_db_derive_field_with_no_column_mapping…`
    // above is that refusal, made at the declaration. What it cannot say is
    // that the constructor still demands the parameter, so a row arrives one
    // value short of building one. That is this call's error and only this
    // call's.
    let skipped = check_src(
        "<?nvs
#[Core\\Db\\Derive]
class Row {
    #[Core\\Db\\Field(skip: true)]
    public Handle $cache;
    public int $n;
    public function constructor(Handle $cache, int $n)
    {
        $this->cache = $cache;
        $this->n = $n;
    }
}
class Handle {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
}
class Rowsource {
    public static function all(Core\\Db\\Transaction $tx): Core\\Db\\Rows<Row>
    {
        return $tx->queryAs<Row>(\"select n from t\", []);
    }
}
",
    );
    assert!(
        skipped
            .iter()
            .any(|d| d.code == Some(code::E_QUERY_AS_NOT_A_ROW_CLASS)),
        "{skipped:?}"
    );

    // The class itself still compiles, which is the half `derive.rs`'s own
    // `#[Db\Field(skip: true)]` case pins — so this test is about the call and
    // not about a rule that grew at the declaration.
    let declaration_alone = check_src(
        "<?nvs
#[Core\\Db\\Derive]
class Row {
    #[Core\\Db\\Field(skip: true)]
    public Handle $cache;
    public int $n;
    public function constructor(Handle $cache, int $n)
    {
        $this->cache = $cache;
        $this->n = $n;
    }
}
class Handle {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
}
",
    );
    assert!(!declaration_alone.has_errors(), "{declaration_alone:?}");
}

/// `rule:core-api/required-optional-and-nullable`'s three columns, read off an
/// inline shape rather than off a declared class — the door a `shapeAs<{...}>`
/// type argument arrives through, where there is no property and no constructor
/// to carry any of them.
///
/// `{b?: ?int}` and `{a: string}` are written in that order deliberately: the
/// parameter positions are the **sorted** order `TypeInterner::shape` interns
/// fields in, which is the order `nvs_ir::lower::shape_class_label` keys a
/// shape class's slots on, so a codec that preserved the written order would
/// fill the wrong slot.
#[test]
fn an_inline_shape_reads_as_a_codec_in_the_interners_sorted_order() {
    let mut interner = TypeInterner::new();
    let int = interner.int();
    let null = interner.null();
    let nullable_int = interner.make_union([int, null]);
    let string = interner.string();
    let shape = interner.shape(vec![
        ShapeField {
            name: "b".to_owned(),
            ty: nullable_int,
            required: false,
        },
        ShapeField::required("a".to_owned(), string),
    ]);

    let codec = derive::shape_codec(shape, &mut interner, &EnumTable::default())
        .expect("an inline shape is a codec");

    // A shape class has no constructor, so what a decode fills is every slot.
    assert_eq!(codec.ctor_arity, 2);
    let keys: Vec<&str> = codec.fields.iter().map(|f| f.key.as_str()).collect();
    assert_eq!(keys, ["a", "b"]);

    let a = &codec.fields[0];
    assert_eq!(a.property, "a");
    assert_eq!(a.ty, CodecTy::Str);
    assert_eq!(a.param, Some(0));
    assert!(a.required, "no `?` was written on `a`");
    assert!(!a.nullable, "`string` admits no null");

    // The two spellings are independent: `b` may be absent, and if it is there
    // it may hold `null`. The decode target is the union's other arm.
    let b = &codec.fields[1];
    assert_eq!(b.property, "b");
    assert_eq!(b.ty, CodecTy::Int);
    assert_eq!(b.param, Some(1));
    assert!(!b.required, "`b?` may be absent");
    assert!(b.nullable, "`?int` admits null");
}

/// The same door, asked about a type that is not a shape at all: it answers
/// `None` rather than an empty codec, so a caller cannot mistake "nothing to
/// decode" for "a shape with no fields".
#[test]
fn a_type_that_is_not_a_shape_reads_as_no_codec_at_all() {
    let mut interner = TypeInterner::new();
    let int = interner.int();
    let enums = EnumTable::default();

    assert!(derive::shape_codec(int, &mut interner, &enums).is_none());

    let empty = interner.shape(vec![]);
    let codec =
        derive::shape_codec(empty, &mut interner, &enums).expect("an empty shape is still a shape");
    assert!(codec.fields.is_empty());
    assert_eq!(codec.ctor_arity, 0);
}

/// A call site that writes an inline shape where `rule:types/arrays`'s
/// type-argument door expects a class is recorded twice over: the label of the
/// class the shape's field *names* produce, and — under the type argument's own
/// span — the contract its field *types* produce.
///
/// Two calls writing one field set and two field types is the whole reason the
/// second record is not keyed by the first: both are `$shape{n}`, and a table
/// keyed by that label would answer one of them with the other's wire types.
#[test]
fn a_written_inline_shape_records_one_label_and_a_contract_per_call_site() {
    let (diags, declared) = check_src_declared(
        "<?nvs
string $doc = \"{}\";
var $one = Core\\Json::decodeAs<{n: int}>($doc);
var $two = Core\\Json::decodeAs<{n: string}>($doc);
",
    );
    assert!(!diags.has_errors(), "{diags:?}");

    let written = |call: &str| match declared.folded_at(call) {
        Some(ExprInfo::Call(resolved)) => resolved
            .written_shape
            .clone()
            .expect("an inline shape was written at this call site"),
        _ => panic!("no call was recorded for `{call}`"),
    };
    let one = written("Core\\Json::decodeAs<{n: int}>($doc)");
    let two = written("Core\\Json::decodeAs<{n: string}>($doc)");

    // One class, because a shape class carries slot names and not slot types.
    assert_eq!(one.label, "$shape{n}");
    assert_eq!(two.label, one.label);
    // The class-keyed table stays empty: what a shape leaves there is a label
    // two call sites share, so nothing about the wire is filed under it.
    assert!(declared.exprs().codec(&one.label).is_none());

    // Two contracts, each reached by the span the shape was written at.
    let field_ty = |shape: &nvs_types::expr_table::WrittenShape| {
        declared
            .exprs()
            .shape_codec(shape.codec)
            .expect("the call site recorded a wire contract")
            .fields[0]
            .ty
    };
    assert_eq!(field_ty(&one), CodecTy::Int);
    assert_eq!(field_ty(&two), CodecTy::Str);
}

/// The fourth way `check_row_sites` answers "no", and the one that used to be
/// said once per row: a field the erasure could give no wire type.
#[test]
fn a_query_as_over_an_inline_shape_field_is_refused_while_compiling() {
    let shaped = check_src(
        "<?nvs
#[Core\\Db\\Derive]
class Row {
    public int $n;
    public {city: string} $where;
    public function constructor(int $n, {city: string} $where)
    {
        $this->n = $n;
        $this->where = $where;
    }
}
class Rowsource {
    public static function all(Core\\Db\\Transaction $tx): Core\\Db\\Rows<Row>
    {
        return $tx->queryAs<Row>(\"select n, city from t\", []);
    }
}
",
    );
    assert!(
        shaped
            .iter()
            .any(|d| d.code == Some(code::E_QUERY_AS_NOT_A_ROW_CLASS)
                && d.message.contains("$where")),
        "{shaped:?}"
    );
}

/// `rule:core-classes/derive-generates-what-is-missing`'s hand-written door,
/// asked at the call site: `Core\Db\Codec` declares `fromRow` alone, so a class
/// that writes it has opted in as squarely as the attribute does and records no
/// mapping precisely because there is nothing left to generate.
///
/// The two halves are asserted together because they are one edit apart. Every
/// condition `check_row_sites` reports is about a mapping, so a pass that read
/// "no mapping" as "no codec" refuses the class that wrote its own — and one
/// that stopped asking would let the class with neither half through to the run
/// time refusal `nvs_stdlib::db::row`'s `hydrate` keeps as a backstop.
#[test]
fn a_query_as_over_a_hand_written_from_row_is_admitted_while_compiling() {
    let source = |body: &str| {
        format!(
            "<?nvs
class Account {{
    public int $id;
    public function constructor(int $id) {{ $this->id = $id; }}
{body}}}
class Rowsource {{
    public static function all(Core\\Db\\Transaction $tx): Core\\Db\\Rows<Account>
    {{
        return $tx->queryAs<Account>(\"select id from t\", []);
    }}
}}
"
        )
    };

    let written = check_src(&source(
        "    public static function fromRow(mixed $row): static { return new static(1); }\n",
    ));
    assert!(!written.has_errors(), "{written:?}");

    let neither = check_src(&source(""));
    assert!(
        neither
            .iter()
            .any(|d| d.code == Some(code::E_QUERY_AS_NOT_A_ROW_CLASS)
                && d.message.contains("fromRow")),
        "{neither:?}"
    );
}

/// The declared types a decoder used to have no wire type for: each erases to a
/// `CodecTy` of its own, so neither door has to tell them from the `Opaque`
/// that means a decoder is missing or from the `Class` that means a nested one.
///
/// Asked of every column-mapped `Core` value type at once rather than of the
/// `Instant` alone: the ones that are still a class label are only correct
/// while `nvs_stdlib::db::row` reads that label, so one more quietly erasing to
/// `Instant` — or the `Instant` quietly going back to `Class` — fails here
/// rather than one row at a time against a live server.
///
/// `bytes` is written both ways for `rule:security/derived-codec-qualifiers`:
/// the qualifier is a call-site question, so a `tainted bytes` column and a
/// `bytes` one are one wire type, exactly as the two spellings of `string` are.
///
/// The inline shape is asked of the JSON door, because it is not a column type
/// at all (`rule:core-classes/db-column-types`) and a `#[Db\Derive]` refuses one
/// at the declaration.
#[test]
fn a_decimal_an_instant_and_bytes_field_erase_to_their_own_codec_type() {
    let (diags, exprs) = check_src_table(
        "<?nvs
#[Core\\Db\\Derive]
class Row {
    public decimal $price;
    public Core\\Time\\Instant $at;
    public bytes $blob;
    public tainted bytes $payload;
    public Core\\Time\\Date $day;
    public Core\\Time\\TimeOfDay $clock;
    public Core\\Time\\DateTime $stamp;
    public Core\\Uuid $id;
    public function constructor(
        decimal $price,
        Core\\Time\\Instant $at,
        bytes $blob,
        tainted bytes $payload,
        Core\\Time\\Date $day,
        Core\\Time\\TimeOfDay $clock,
        Core\\Time\\DateTime $stamp,
        Core\\Uuid $id,
    )
    {
        $this->price = $price;
        $this->at = $at;
        $this->blob = $blob;
        $this->payload = $payload;
        $this->day = $day;
        $this->clock = $clock;
        $this->stamp = $stamp;
        $this->id = $id;
    }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    let codec = exprs
        .db_codec("Row")
        .expect("the class derives a row codec");
    let field = |property: &str| {
        codec
            .fields
            .iter()
            .find(|field| field.property == property)
            .unwrap_or_else(|| panic!("no `${property}` field"))
    };

    assert_eq!(field("price").ty, CodecTy::Decimal);
    assert_eq!(field("blob").ty, CodecTy::Bytes);
    assert_eq!(field("payload").ty, CodecTy::Bytes);

    // The label rides along with the wire type, because what the decode owes is
    // to say which class the column was expected to build.
    let at = field("at");
    assert_eq!(at.ty, CodecTy::Instant);
    assert_eq!(at.class.as_deref(), Some(r"Core\Time\Instant"));

    // The rest of the map is a class label and is read by it.
    for (property, class) in [
        ("day", r"Core\Time\Date"),
        ("clock", r"Core\Time\TimeOfDay"),
        ("stamp", r"Core\Time\DateTime"),
        ("id", r"Core\Uuid"),
    ] {
        let value = field(property);
        assert_eq!(value.ty, CodecTy::Class, "`${property}`");
        assert_eq!(value.class.as_deref(), Some(class), "`${property}`");
    }

    // None of them is the erasure that means "no decoder": for a row that is an
    // `array<array<T>>` and nothing else, every other reachable column type
    // having a wire type of its own.
    assert!(
        codec.fields.iter().all(|field| field.ty != CodecTy::Opaque),
        "{codec:?}"
    );

    // The JSON door's own third: an inline shape reached as a *field*. It
    // erases to a wire type carrying **both** resolved pointers a decode needs
    // — the class a literal of those same field names builds, and the per-field
    // list that class's label cannot carry, since a shape class is keyed on
    // field names alone (`rule:core-classes/derive-field-list`).
    let (diags, exprs) = check_src_table(
        "<?nvs
#[Core\\Json\\Derive]
class Order {
    public {total: decimal, note?: string} $meta;
    public function constructor({total: decimal, note?: string} $meta)
    {
        $this->meta = $meta;
    }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    let codec = exprs.codec("Order").expect("the class derives a codec");
    let meta = codec
        .fields
        .iter()
        .find(|field| field.property == "meta")
        .expect("no `$meta` field");
    assert_eq!(meta.ty, CodecTy::Shape);
    // Sorted, because that is the order the interner lays a shape's fields out
    // in and therefore the order its class lays its slots out in.
    assert_eq!(meta.class.as_deref(), Some("$shape{note,total}"));
    let nested = meta
        .shape
        .as_deref()
        .expect("a shape field carries its own contract");
    // `rule:types/shape-type`'s `?` is the *optional* column and says nothing
    // about `null`, so it rides down as `required` exactly as a constructor
    // parameter's default does on a class.
    let erased: Vec<(&str, CodecTy, bool)> = nested
        .fields
        .iter()
        .map(|field| (field.property.as_str(), field.ty, field.required))
        .collect();
    assert_eq!(
        erased,
        vec![
            ("note", CodecTy::Str, false),
            ("total", CodecTy::Decimal, true),
        ],
        "{nested:?}"
    );
    // A shape declares no constructor, so what a decode fills is every slot it
    // has — which is what makes the field's index its parameter and its slot.
    assert_eq!(nested.ctor_arity, nested.fields.len());
}
