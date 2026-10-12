//! The properties the engine parses: each longhand with whether it inherits and the grammar its
//! value is read with, and each shorthand with the longhands it sets.
//!
//! This is the engine's half of the subset. `extensions/pdf/subset.json` is the list a reader and
//! the editor see (`rule:core-classes/pdf-css-subset-table`), and `tests/css.rs` holds the two
//! equal in both directions, so a property is added here and there in one change.

/// How a longhand's value is read. Every grammar also takes `inherit`, `initial`, `unset` and
/// `revert`, which the cascade handles before the grammar sees the value.
#[derive(Debug, Clone, Copy)]
pub enum Grammar {
    /// One length or percentage, or one of the keywords. `negative` admits a value below zero.
    Length {
        negative: bool,
        keywords: &'static [&'static str],
    },
    /// One colour, or `currentcolor`.
    Color,
    /// Exactly one of the keywords.
    Keyword(&'static [&'static str]),
    /// One or more of the keywords, each at most once; or `none`.
    Keywords(&'static [&'static str]),
    /// One number of zero or more, or one of the keywords.
    Number(&'static [&'static str]),
    /// One whole number, or one of the keywords. `positive` admits only 1 and up.
    Integer {
        positive: bool,
        keywords: &'static [&'static str],
    },
    /// `normal`, a number, or a length or percentage.
    LineHeight,
    /// `normal`, `bold`, `bolder`, `lighter` or a number from 1 to 1000.
    FontWeight,
    /// A comma-separated list of family names, each an identifier sequence or a string.
    Families,
    /// `none`, or a list of the 2D transform functions.
    Transform,
    /// `static`, `relative`, `absolute`, `fixed` or `running(<name>)`.
    Position,
    /// Any value. The stage that lays the property out reads it, and until then the cascade
    /// carries its text after `var()` is substituted.
    Free,
}

/// A longhand: its name, whether a child inherits it, and its grammar.
#[derive(Debug)]
pub struct Property {
    pub name: &'static str,
    pub inherited: bool,
    pub grammar: Grammar,
}

const fn prop(name: &'static str, inherited: bool, grammar: Grammar) -> Property {
    Property {
        name,
        inherited,
        grammar,
    }
}

const fn len(negative: bool, keywords: &'static [&'static str]) -> Grammar {
    Grammar::Length { negative, keywords }
}

const BORDER_STYLE: &[&str] = &["none", "hidden", "solid", "dashed", "dotted", "double"];
const BORDER_WIDTH: &[&str] = &["thin", "medium", "thick"];
const SIZE: &[&str] = &["auto", "min-content", "max-content", "fit-content"];
const OVERFLOW: &[&str] = &["visible", "hidden", "clip", "auto", "scroll"];
const ALIGN: &[&str] = &[
    "normal",
    "stretch",
    "start",
    "end",
    "center",
    "flex-start",
    "flex-end",
    "baseline",
    "left",
    "right",
];
const DISTRIBUTE: &[&str] = &[
    "normal",
    "stretch",
    "start",
    "end",
    "center",
    "flex-start",
    "flex-end",
    "left",
    "right",
    "space-between",
    "space-around",
    "space-evenly",
];
const BREAK: &[&str] = &[
    "auto",
    "avoid",
    "always",
    "all",
    "page",
    "left",
    "right",
    "recto",
    "verso",
    "avoid-page",
];
const LIST_STYLE_TYPE: &[&str] = &[
    "none",
    "disc",
    "circle",
    "square",
    "decimal",
    "decimal-leading-zero",
    "lower-roman",
    "upper-roman",
    "lower-alpha",
    "upper-alpha",
    "lower-latin",
    "upper-latin",
    "lower-greek",
];

/// Every longhand the engine parses, in no particular order.
pub static PROPERTIES: &[Property] = &[
    // Text.
    prop("color", true, Grammar::Color),
    prop("font-family", true, Grammar::Families),
    prop(
        "font-size",
        true,
        len(
            false,
            &[
                "xx-small",
                "x-small",
                "small",
                "medium",
                "large",
                "x-large",
                "xx-large",
                "xxx-large",
                "smaller",
                "larger",
            ],
        ),
    ),
    prop(
        "font-style",
        true,
        Grammar::Keyword(&["normal", "italic", "oblique"]),
    ),
    prop("font-weight", true, Grammar::FontWeight),
    prop(
        "font-variant",
        true,
        Grammar::Keyword(&["normal", "small-caps"]),
    ),
    prop("font-feature-settings", true, Grammar::Free),
    prop("line-height", true, Grammar::LineHeight),
    prop(
        "text-align",
        true,
        Grammar::Keyword(&["left", "right", "center", "justify", "start", "end"]),
    ),
    prop(
        "text-align-last",
        true,
        Grammar::Keyword(&["auto", "left", "right", "center", "justify", "start", "end"]),
    ),
    prop("text-indent", true, len(true, &[])),
    prop(
        "text-decoration-line",
        false,
        Grammar::Keywords(&["underline", "overline", "line-through"]),
    ),
    prop(
        "text-decoration-style",
        false,
        Grammar::Keyword(&["solid", "double", "dotted", "dashed", "wavy"]),
    ),
    prop("text-decoration-color", false, Grammar::Color),
    prop(
        "text-transform",
        true,
        Grammar::Keyword(&["none", "uppercase", "lowercase", "capitalize"]),
    ),
    prop("letter-spacing", true, len(true, &["normal"])),
    prop("word-spacing", true, len(true, &["normal"])),
    prop(
        "white-space",
        true,
        Grammar::Keyword(&[
            "normal",
            "nowrap",
            "pre",
            "pre-wrap",
            "pre-line",
            "break-spaces",
        ]),
    ),
    prop(
        "word-break",
        true,
        Grammar::Keyword(&["normal", "break-all", "keep-all", "break-word"]),
    ),
    prop(
        "overflow-wrap",
        true,
        Grammar::Keyword(&["normal", "break-word", "anywhere"]),
    ),
    prop(
        "text-overflow",
        false,
        Grammar::Keyword(&["clip", "ellipsis"]),
    ),
    prop(
        "vertical-align",
        false,
        len(
            true,
            &[
                "baseline",
                "sub",
                "super",
                "top",
                "text-top",
                "middle",
                "bottom",
                "text-bottom",
            ],
        ),
    ),
    prop("direction", true, Grammar::Keyword(&["ltr", "rtl"])),
    prop(
        "unicode-bidi",
        false,
        Grammar::Keyword(&[
            "normal",
            "embed",
            "isolate",
            "bidi-override",
            "isolate-override",
            "plaintext",
        ]),
    ),
    prop(
        "hyphens",
        true,
        Grammar::Keyword(&["none", "manual", "auto"]),
    ),
    // Boxes.
    prop("margin-top", false, len(true, &["auto"])),
    prop("margin-right", false, len(true, &["auto"])),
    prop("margin-bottom", false, len(true, &["auto"])),
    prop("margin-left", false, len(true, &["auto"])),
    prop("padding-top", false, len(false, &[])),
    prop("padding-right", false, len(false, &[])),
    prop("padding-bottom", false, len(false, &[])),
    prop("padding-left", false, len(false, &[])),
    prop("border-top-width", false, len(false, BORDER_WIDTH)),
    prop("border-right-width", false, len(false, BORDER_WIDTH)),
    prop("border-bottom-width", false, len(false, BORDER_WIDTH)),
    prop("border-left-width", false, len(false, BORDER_WIDTH)),
    prop("border-top-style", false, Grammar::Keyword(BORDER_STYLE)),
    prop("border-right-style", false, Grammar::Keyword(BORDER_STYLE)),
    prop("border-bottom-style", false, Grammar::Keyword(BORDER_STYLE)),
    prop("border-left-style", false, Grammar::Keyword(BORDER_STYLE)),
    prop("border-top-color", false, Grammar::Color),
    prop("border-right-color", false, Grammar::Color),
    prop("border-bottom-color", false, Grammar::Color),
    prop("border-left-color", false, Grammar::Color),
    prop("border-top-left-radius", false, len(false, &[])),
    prop("border-top-right-radius", false, len(false, &[])),
    prop("border-bottom-right-radius", false, len(false, &[])),
    prop("border-bottom-left-radius", false, len(false, &[])),
    prop("width", false, len(false, SIZE)),
    prop("height", false, len(false, SIZE)),
    prop("min-width", false, len(false, SIZE)),
    prop("min-height", false, len(false, SIZE)),
    prop("max-width", false, len(false, &["none"])),
    prop("max-height", false, len(false, &["none"])),
    prop(
        "box-sizing",
        false,
        Grammar::Keyword(&["content-box", "border-box"]),
    ),
    prop("background-color", false, Grammar::Color),
    prop("background-image", false, Grammar::Free),
    prop("background-repeat", false, Grammar::Free),
    prop("background-position", false, Grammar::Free),
    prop("background-size", false, Grammar::Free),
    prop("outline-width", false, len(false, BORDER_WIDTH)),
    prop("outline-style", false, Grammar::Keyword(BORDER_STYLE)),
    prop("outline-color", false, Grammar::Color),
    prop("outline-offset", false, len(true, &[])),
    prop("opacity", false, Grammar::Number(&[])),
    prop(
        "display",
        false,
        Grammar::Keyword(&[
            "none",
            "block",
            "inline",
            "inline-block",
            "flow-root",
            "list-item",
            "flex",
            "inline-flex",
            "grid",
            "inline-grid",
            "table",
            "inline-table",
            "table-caption",
            "table-header-group",
            "table-row-group",
            "table-footer-group",
            "table-row",
            "table-cell",
            "table-column-group",
            "table-column",
            "contents",
        ]),
    ),
    prop(
        "visibility",
        true,
        Grammar::Keyword(&["visible", "hidden", "collapse"]),
    ),
    prop("overflow-x", false, Grammar::Keyword(OVERFLOW)),
    prop("overflow-y", false, Grammar::Keyword(OVERFLOW)),
    // Lists and generated content.
    prop("list-style-type", true, Grammar::Keyword(LIST_STYLE_TYPE)),
    prop(
        "list-style-position",
        true,
        Grammar::Keyword(&["inside", "outside"]),
    ),
    prop("list-style-image", true, Grammar::Free),
    prop("counter-reset", false, Grammar::Free),
    prop("counter-increment", false, Grammar::Free),
    prop("content", false, Grammar::Free),
    // Images.
    prop(
        "object-fit",
        false,
        Grammar::Keyword(&["fill", "contain", "cover", "none", "scale-down"]),
    ),
    prop("object-position", false, Grammar::Free),
    // Layout.
    prop(
        "flex-direction",
        false,
        Grammar::Keyword(&["row", "row-reverse", "column", "column-reverse"]),
    ),
    prop(
        "flex-wrap",
        false,
        Grammar::Keyword(&["nowrap", "wrap", "wrap-reverse"]),
    ),
    prop("flex-grow", false, Grammar::Number(&[])),
    prop("flex-shrink", false, Grammar::Number(&[])),
    prop("flex-basis", false, len(false, &["auto", "content"])),
    prop(
        "order",
        false,
        Grammar::Integer {
            positive: false,
            keywords: &[],
        },
    ),
    prop("justify-content", false, Grammar::Keyword(DISTRIBUTE)),
    prop("align-content", false, Grammar::Keyword(DISTRIBUTE)),
    prop("align-items", false, Grammar::Keyword(ALIGN)),
    prop("justify-items", false, Grammar::Keyword(ALIGN)),
    prop("align-self", false, Grammar::Keyword(ALIGN)),
    prop("justify-self", false, Grammar::Keyword(ALIGN)),
    prop("row-gap", false, len(false, &["normal"])),
    prop("column-gap", false, len(false, &["normal"])),
    prop("grid-template-columns", false, Grammar::Free),
    prop("grid-template-rows", false, Grammar::Free),
    prop("grid-template-areas", false, Grammar::Free),
    prop("grid-auto-columns", false, Grammar::Free),
    prop("grid-auto-rows", false, Grammar::Free),
    prop("grid-auto-flow", false, Grammar::Free),
    prop("grid-row", false, Grammar::Free),
    prop("grid-column", false, Grammar::Free),
    prop("grid-area", false, Grammar::Free),
    prop(
        "float",
        false,
        Grammar::Keyword(&["none", "left", "right", "inline-start", "inline-end"]),
    ),
    prop(
        "clear",
        false,
        Grammar::Keyword(&[
            "none",
            "left",
            "right",
            "both",
            "inline-start",
            "inline-end",
        ]),
    ),
    prop("position", false, Grammar::Position),
    prop("top", false, len(true, &["auto"])),
    prop("right", false, len(true, &["auto"])),
    prop("bottom", false, len(true, &["auto"])),
    prop("left", false, len(true, &["auto"])),
    prop(
        "z-index",
        false,
        Grammar::Integer {
            positive: false,
            keywords: &["auto"],
        },
    ),
    prop("transform", false, Grammar::Transform),
    prop("transform-origin", false, Grammar::Free),
    // Pages.
    prop("size", false, Grammar::Free),
    prop("page", false, Grammar::Free),
    prop("string-set", false, Grammar::Free),
    prop("break-before", false, Grammar::Keyword(BREAK)),
    prop("break-after", false, Grammar::Keyword(BREAK)),
    prop(
        "break-inside",
        false,
        Grammar::Keyword(&["auto", "avoid", "avoid-page"]),
    ),
    prop(
        "orphans",
        true,
        Grammar::Integer {
            positive: true,
            keywords: &[],
        },
    ),
    prop(
        "widows",
        true,
        Grammar::Integer {
            positive: true,
            keywords: &[],
        },
    ),
    // Tables.
    prop(
        "border-collapse",
        true,
        Grammar::Keyword(&["collapse", "separate"]),
    ),
    prop("border-spacing", true, Grammar::Free),
    prop("caption-side", true, Grammar::Keyword(&["top", "bottom"])),
    prop("empty-cells", true, Grammar::Keyword(&["show", "hide"])),
    prop("table-layout", false, Grammar::Keyword(&["auto", "fixed"])),
    prop("box-decoration-break", false, Grammar::Keyword(&["slice"])),
];

/// Every shorthand the engine parses, with the longhands it sets. `css::value::expand` reads a
/// shorthand's value into them.
pub static SHORTHANDS: &[(&str, &[&str])] = &[
    (
        "margin",
        &["margin-top", "margin-right", "margin-bottom", "margin-left"],
    ),
    (
        "padding",
        &[
            "padding-top",
            "padding-right",
            "padding-bottom",
            "padding-left",
        ],
    ),
    ("inset", &["top", "right", "bottom", "left"]),
    (
        "border-width",
        &[
            "border-top-width",
            "border-right-width",
            "border-bottom-width",
            "border-left-width",
        ],
    ),
    (
        "border-style",
        &[
            "border-top-style",
            "border-right-style",
            "border-bottom-style",
            "border-left-style",
        ],
    ),
    (
        "border-color",
        &[
            "border-top-color",
            "border-right-color",
            "border-bottom-color",
            "border-left-color",
        ],
    ),
    (
        "border",
        &[
            "border-top-width",
            "border-right-width",
            "border-bottom-width",
            "border-left-width",
            "border-top-style",
            "border-right-style",
            "border-bottom-style",
            "border-left-style",
            "border-top-color",
            "border-right-color",
            "border-bottom-color",
            "border-left-color",
        ],
    ),
    (
        "border-top",
        &["border-top-width", "border-top-style", "border-top-color"],
    ),
    (
        "border-right",
        &[
            "border-right-width",
            "border-right-style",
            "border-right-color",
        ],
    ),
    (
        "border-bottom",
        &[
            "border-bottom-width",
            "border-bottom-style",
            "border-bottom-color",
        ],
    ),
    (
        "border-left",
        &[
            "border-left-width",
            "border-left-style",
            "border-left-color",
        ],
    ),
    (
        "border-radius",
        &[
            "border-top-left-radius",
            "border-top-right-radius",
            "border-bottom-right-radius",
            "border-bottom-left-radius",
        ],
    ),
    (
        "outline",
        &["outline-width", "outline-style", "outline-color"],
    ),
    (
        "font",
        &[
            "font-style",
            "font-variant",
            "font-weight",
            "font-size",
            "line-height",
            "font-family",
        ],
    ),
    (
        "text-decoration",
        &[
            "text-decoration-line",
            "text-decoration-style",
            "text-decoration-color",
        ],
    ),
    (
        "background",
        &[
            "background-color",
            "background-image",
            "background-repeat",
            "background-position",
            "background-size",
        ],
    ),
    (
        "list-style",
        &["list-style-type", "list-style-position", "list-style-image"],
    ),
    ("overflow", &["overflow-x", "overflow-y"]),
    ("gap", &["row-gap", "column-gap"]),
    ("flex", &["flex-grow", "flex-shrink", "flex-basis"]),
    ("flex-flow", &["flex-direction", "flex-wrap"]),
    ("page-break-before", &["break-before"]),
    ("page-break-after", &["break-after"]),
    ("page-break-inside", &["break-inside"]),
];

/// The longhand named `name`.
pub fn property(name: &str) -> Option<&'static Property> {
    PROPERTIES.iter().find(|p| p.name == name)
}

/// The longhands the shorthand `name` sets.
pub fn shorthand(name: &str) -> Option<&'static [&'static str]> {
    SHORTHANDS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, longhands)| *longhands)
}

/// The longhands a declaration of `name` sets: the shorthand's, or the longhand itself, or none
/// for a property outside the subset.
pub fn longhands(name: &str) -> Option<Vec<&'static str>> {
    if let Some(list) = shorthand(name) {
        return Some(list.to_vec());
    }
    property(name).map(|p| vec![p.name])
}

/// Every property name the engine parses, longhands first and then shorthands.
pub fn names() -> impl Iterator<Item = &'static str> {
    PROPERTIES
        .iter()
        .map(|p| p.name)
        .chain(SHORTHANDS.iter().map(|(n, _)| *n))
}
