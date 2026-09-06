//! [ADR 0091] §§ 3-5: the two modes, the five defaults a mode selects, and the ceiling that bounds a
//! runtime flip.
//!
//! **§ 3's table is closed, and this is it.** A mode is a shorthand for the default of five
//! directives that are each settable on their own, so what a mode *is* can be printed as five rows
//! rather than described as a behaviour. That property only holds while the table has one home, and
//! this module is it: a second copy anywhere is a mode that changes something the first copy does not
//! list. § 3a's three startup rows are deliberately **not** here — they are fixed at boot, never
//! re-derived and never flippable, and mixing the two tables is exactly the confusion that section
//! was split out to prevent.
//!
//! **A value is spelled the way [`Request::get`](crate::Request::get) answers one**, because
//! `rule:config/ini-set-is-core-config-set`'s API is string-in/string-out and a flip whose answer could be told apart from the
//! same value written in `nvs.toml` would be a second spelling of every directive it touches. That
//! is the *text* of the value and not its TOML rendering — a string with no quotes around it, a bool
//! as `true`/`false`. `[log] level` is `Log\Level`'s own case name, `Info` or `Debug`
//! (`rule:errors/log-level`).
//!
//! **The ceiling is a comparison over a two-value order** (§ 5): `production` is the restrictive end
//! and `development` the permissive one, and a flip is allowed exactly where what is asked for is no
//! more permissive than the ceiling. An unset `[mode] ceiling` is the mode the host started in, which
//! is what makes a production host — one that wrote no configuration at all — refuse every flip
//! without an operator having to know this feature exists.
//!
//! **What is not here yet:** § 3's defaults are applied only by `rule:config/a-program-may-read-and-flip-its-mode`'s runtime flip, not at
//! boot. A tree that writes `mode = "development"` and nothing else still resolves `[log] format` to
//! nothing rather than to `"text"`, so a reader must treat an unset directive as its production
//! default. Closing that means the same table read where the snapshot is built, which is this
//! module's next caller and not a second copy of the rows.
//!
//! [ADR 0091]: ../../../docs/adr/0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md

/// The dotted key a mode is written and flipped at — § 4's own spelling, and the only one.
pub const KEY: &str = "mode.default";

/// § 5's restrictive end, and the mode a host with no configuration starts in.
pub const PRODUCTION: &str = "production";

/// § 5's permissive end.
pub const DEVELOPMENT: &str = "development";

/// One row of § 3's table: a directive whose default the mode selects, and the value it takes in
/// each of the two modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Derived {
    /// The dotted key, as `nvs.toml` writes it and as `Core\Config` reads it.
    pub key: &'static str,
    /// The value in `production`.
    pub production: &'static str,
    /// The value in `development`.
    pub development: &'static str,
}

impl Derived {
    /// This row's value in `mode`, and `None` for a mode that is not one of the two.
    #[must_use]
    pub fn value(&self, mode: &str) -> Option<&'static str> {
        match mode {
            PRODUCTION => Some(self.production),
            DEVELOPMENT => Some(self.development),
            _ => None,
        }
    }
}

/// § 3's table, complete: the five directives a mode selects the default of, and nothing else.
///
/// Each row's owning ADR is named in the section's own table; they are not repeated here, because a
/// row's value belongs to the mode and its *meaning* belongs to the ADR that spells the directive.
pub const DERIVED: &[Derived] = &[
    Derived {
        key: "debug.inline",
        production: "false",
        development: "true",
    },
    Derived {
        key: "log.format",
        production: "json",
        development: "text",
    },
    Derived {
        key: "log.level",
        production: "Info",
        development: "Debug",
    },
    Derived {
        key: "log.access",
        production: "false",
        development: "true",
    },
    Derived {
        key: "http.errors.detail",
        production: "generic",
        development: "full",
    },
];

/// How permissive `mode` is — § 5's order, as the only comparison this module makes.
///
/// `None` for anything that is not one of the two modes, which is what refuses a flip to a name
/// nobody defined rather than deriving a table of nulls for it.
#[must_use]
pub fn rank(mode: &str) -> Option<u8> {
    match mode {
        PRODUCTION => Some(0),
        DEVELOPMENT => Some(1),
        _ => None,
    }
}

/// § 5: whether `asked` is a mode within `ceiling`, both named.
///
/// An unknown mode is outside every ceiling, on either side of the comparison: a ceiling nobody can
/// rank is one nothing may be checked against, and answering `true` there would make a typo in a
/// root-owned file into a permission.
#[must_use]
pub fn within(asked: &str, ceiling: &str) -> bool {
    match (rank(asked), rank(ceiling)) {
        (Some(asked), Some(ceiling)) => asked <= ceiling,
        _ => false,
    }
}
