//! `rule:security/taint-propagation`'s one classifier: text is clean only
//! where the checker proves it, and a type the classifier does not recognise
//! counts as tainted.
//!
//! The first tests ask `text_class` directly, over interned types. The last
//! ones check whole programs: a class reached through nested shapes is held
//! to the decode checks, and two derive type maps refuse the text forms they
//! do not list.

mod common;

use common::check_src;
use nvs_diagnostics::{Code, Diagnostics, code};
use nvs_hir::QName;
use nvs_types::expr::quals::{TextClass, may_carry_outside_text, text_class};
use nvs_types::ty::{ShapeField, TypeInterner};

/// Whether `diags` reported `want`, by code.
fn reported(diags: &Diagnostics, want: Code) -> bool {
    diags.iter().any(|d| d.code == Some(want))
}

#[test]
fn a_core_class_no_row_declares_counts_as_tainted() {
    let mut interner = TypeInterner::new();
    let unknown = interner.class(QName::parse(r"Core\NoSuchClass"));
    assert_eq!(text_class(unknown, &interner), TextClass::Tainted);
    assert!(may_carry_outside_text(unknown, &interner));

    // A declared class carries no text of its own, and a source of request
    // bytes is tainted.
    let file = interner.class(QName::parse(r"Core\IO\File"));
    assert_eq!(text_class(file, &interner), TextClass::NoText);
    let part = interner.class(QName::parse(r"Core\Request\Part"));
    assert_eq!(text_class(part, &interner), TextClass::Tainted);
}

#[test]
fn a_generic_wrapper_counts_as_tainted_when_its_argument_does() {
    let mut interner = TypeInterner::new();
    let tainted = interner.tainted_string();
    let clean = interner.string();
    let iterator = QName::parse(nvs_hir::interfaces::ITERATOR);
    let outside = interner.generic_class(iterator.clone(), vec![tainted]);
    assert_eq!(
        text_class(outside, &interner),
        TextClass::Holds(vec![tainted])
    );
    assert!(may_carry_outside_text(outside, &interner));

    let own = interner.generic_class(iterator, vec![clean]);
    assert!(!may_carry_outside_text(own, &interner));

    // A wrapper of an unknown `Core` name is tainted whatever it wraps.
    let unknown = interner.generic_class(QName::parse(r"Core\NoSuchBox"), vec![clean]);
    assert!(may_carry_outside_text(unknown, &interner));
}

#[test]
fn a_shape_in_an_array_in_a_nullable_counts_as_tainted() {
    let mut interner = TypeInterner::new();
    let tainted = interner.tainted_string();
    let shape = interner.shape(vec![ShapeField::required("a".to_owned(), tainted)]);
    let array = interner.array(shape);
    let null = interner.null();
    let nullable = interner.make_union([array, null]);
    assert!(may_carry_outside_text(nullable, &interner));

    // The same nesting over `mixed` is tainted too.
    let mixed = interner.mixed();
    let shape = interner.shape(vec![ShapeField::required("a".to_owned(), mixed)]);
    let array = interner.array(shape);
    let nullable = interner.make_union([array, null]);
    assert!(may_carry_outside_text(nullable, &interner));

    // Text the program made is clean at any depth.
    let clean = interner.string();
    let shape = interner.shape(vec![ShapeField::required("a".to_owned(), clean)]);
    let array = interner.array(shape);
    let nullable = interner.make_union([array, null]);
    assert!(!may_carry_outside_text(nullable, &interner));
}

#[test]
fn a_class_under_two_shapes_is_held_to_the_decode_check() {
    let src = "<?nvs
#[Core\\Json\\Derive]
class User {
    public string $name;
    public function constructor(string $name) { $this->name = $name; }
}
class Shop {
    public static function run(): void
    {
        var $decoded = Core\\Request::jsonAs<{ a: { b: User } }>();
    }
}
";
    let diags = check_src(src);
    assert!(
        reported(&diags, code::E_DECODED_FIELD_NOT_TAINTED),
        "no E0810\n{diags:?}"
    );
}

/// A deriving class with one property of type `ty`, under `attribute`.
fn deriving(attribute: &str, ty: &str) -> String {
    format!(
        "<?nvs
#[{attribute}]
class Item {{
    public {ty} $value;
    public function constructor({ty} $value) {{ $this->value = $value; }}
}}
"
    )
}

#[test]
fn a_row_codec_refuses_every_secret_text_form() {
    for attribute in [r"Core\Db\Derive", r"Core\Json\Derive"] {
        let control = deriving(attribute, "tainted string");
        let diags = check_src(&control);
        assert!(
            !diags.has_errors(),
            "the control did not compile\n{control}\n{diags:?}"
        );
    }
    for ty in [
        "secret string",
        "secret bytes",
        "secret tainted string",
        "secret tainted bytes",
    ] {
        let src = deriving(r"Core\Db\Derive", ty);
        let diags = check_src(&src);
        assert!(diags.has_errors(), "`{ty}` compiled in a row\n{src}");
    }
}

#[test]
fn a_json_codec_refuses_every_bytes_form() {
    for ty in [
        "bytes",
        "tainted bytes",
        "secret bytes",
        "secret tainted bytes",
    ] {
        let src = deriving(r"Core\Json\Derive", ty);
        let diags = check_src(&src);
        assert!(diags.has_errors(), "`{ty}` compiled in JSON\n{src}");
    }
}
