//! `rule:security/derived-codec-qualifiers`: a class a typed decoder reaches
//! through a shape is held to the same decode checks as a class written at the
//! root of the type argument.
//!
//! The table below crosses every typed decoder with every place a class can
//! sit in the type argument: at the root, under a shape, under a shape in a
//! shape, under a nullable shape field, in an array of shapes, and in a shape
//! property of another deriving class. Each cell is checked three times: a
//! plain `string` property is `E0810`, a `tainted string` one compiles, and a
//! class whose constructor the codec cannot fill is `E0820`.

mod common;

use common::check_src;
use nvs_diagnostics::{Code, Diagnostics, code};

/// Whether `diags` reported `want`, by code: a fixture written to trip one
/// check can trip a second, and "some error" is satisfied by the wrong one.
fn reported(diags: &Diagnostics, want: Code) -> bool {
    diags.iter().any(|d| d.code == Some(want))
}

/// Every typed decoder that reads text from outside the program, written as a
/// call with `T` where the type argument goes. The function around it gives a
/// `tainted` document, a byte source and an array of `mixed`.
const DECODERS: &[&str] = &[
    "Core\\Request::jsonAs<T>()",
    "Core\\Request::queryAs<T>()",
    "Core\\Request::postAs<T>()",
    "Core\\Json::decodeAs<T>($doc)",
    "Core\\Json::decodeEach<T>($src)",
    "Core\\Arr::shapeAs<T>($raw)",
];

/// Every place a class can sit in the type argument. `Post` is the class the
/// check is about; `Blog` is a deriving class with a shape property.
const DEPTHS: &[&str] = &[
    "Post",
    "{ post: Post }",
    "{ blog: { post: Post } }",
    "{ post: ?Post }",
    "array<{ post: Post }>",
    "{ posts: array<{ post: Post }> }",
    "Blog",
    "{ blog: Blog }",
];

/// `Blog` holds `Post` in a shape property, so a decode of `Blog` reaches
/// `Post` only through that shape.
const BLOG: &str = "#[Core\\Json\\Derive]
class Blog {
    public tainted string $name;
    public { main: { post: Post } } $main;
    public function constructor(tainted string $name, { main: { post: Post } } $main)
    {
        $this->name = $name;
        $this->main = $main;
    }
}
";

/// A `Post` whose one text property is a plain `string`.
const PLAIN: &str = "#[Core\\Json\\Derive]
class Post {
    public string $title;
    public function constructor(string $title) { $this->title = $title; }
}
";

/// A `Post` whose one text property is `tainted`.
const TAINTED: &str = "#[Core\\Json\\Derive]
class Post {
    public tainted string $title;
    public function constructor(tainted string $title) { $this->title = $title; }
}
";

/// A `Post` whose codec skips a constructor parameter, so no decode can
/// build one.
const UNBUILDABLE: &str = "#[Core\\Json\\Derive]
class Post {
    public tainted string $title;
    #[Core\\Json\\Field(skip: true)]
    public tainted string $draft;
    public function constructor(tainted string $title, tainted string $draft)
    {
        $this->title = $title;
        $this->draft = $draft;
    }
}
";

/// The program for one decoder, one depth and one `Post`.
fn program(decoder: &str, depth: &str, post: &str) -> String {
    let call = decoder.replace("<T>", &format!("<{depth}>"));
    format!(
        "<?nvs\n{post}{BLOG}\
         class Shop {{\n    public static function run(tainted string $doc, \
         Core\\Bytes::Source $src, array<mixed> $raw): void\n    \
         {{\n        var $decoded = {call};\n    }}\n}}\n"
    )
}

#[test]
fn a_plain_string_reached_through_a_shape_is_e0810_at_every_decoder() {
    for decoder in DECODERS {
        for depth in DEPTHS {
            let src = program(decoder, depth, PLAIN);
            let diags = check_src(&src);
            assert!(
                reported(&diags, code::E_DECODED_FIELD_NOT_TAINTED),
                "{decoder} with {depth}: no E0810\n{src}\n{diags:?}"
            );
            assert!(
                diags.iter().any(|d| d.message.contains("`Post::$title`")
                    && d.notes.iter().any(|n| n.starts_with("help: "))),
                "{decoder} with {depth}: E0810 names no `Post::$title` with help\n{diags:?}"
            );
        }
    }
}

#[test]
fn a_tainted_string_reached_through_a_shape_compiles_at_every_decoder() {
    for decoder in DECODERS {
        for depth in DEPTHS {
            let src = program(decoder, depth, TAINTED);
            let diags = check_src(&src);
            assert!(
                !diags.has_errors(),
                "{decoder} with {depth}: an error\n{src}\n{diags:?}"
            );
        }
    }
}

#[test]
fn a_class_that_cannot_be_built_reached_through_a_shape_is_e0820_at_every_decoder() {
    for decoder in DECODERS {
        for depth in DEPTHS {
            let src = program(decoder, depth, UNBUILDABLE);
            let diags = check_src(&src);
            assert!(
                reported(&diags, code::E_DECODED_CLASS_NOT_CONSTRUCTIBLE),
                "{decoder} with {depth}: no E0820\n{src}\n{diags:?}"
            );
        }
    }
}

#[test]
fn a_class_with_no_codec_reached_through_a_shape_is_e0821() {
    let src = "<?nvs
class Note {
    public tainted string $text;
    public function constructor(tainted string $text) { $this->text = $text; }
}
class Shop {
    public static function run(): void
    {
        var $decoded = Core\\Request::jsonAs<{ note: Note }>();
    }
}
";
    let diags = check_src(src);
    assert!(
        reported(&diags, code::E_DECODED_CLASS_HAS_NO_CODEC),
        "{diags:?}"
    );
}
