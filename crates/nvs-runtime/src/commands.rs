//! [ADR 0086](../../../docs/adr/0086-core-cli-terminal-is-a-sink.md) § 6's
//! command table, as a *running* program sees it: the rows the compiler built,
//! carried on the request's own context.
//!
//! # Why the table is a runtime value at all
//!
//! § 6 says `Core\Command::help` and `::run` are **generated from the table**,
//! and the table is a compile product — `nvs_types::commands::CommandTable`,
//! built by the same ADR 0061 § 3 scan ADR 0077's routes are. The alternative
//! this rejects is expanding those members while checking, which is what
//! [`crate::script`]'s sibling `Core\Program::implementing` does: `help` takes a
//! `?string $name` a program is free to compute, so an expansion would have to
//! fold every page the program could ask for and index them, and `run` reads
//! the process's own argument vector, which no compile-time answer can hold.
//!
//! So the rows cross instead, and they cross as **strings and one closed enum**
//! — the same decision `nvs_types::commands::Command`'s own docs make about
//! crossing into `nvs-ir`, with [`ArgConv`] the one thing a matcher needs that
//! no string spells: which conversion the checker picked for a parameter.
//! Nothing below this line learns that a compiler exists: this crate has no
//! `nvs-types` dependency and could not name one of its types if it wanted to.
//!
//! # Where it is installed, and what a context without one means
//!
//! Written onto [`crate::Ctx`] before the program runs, by whoever compiled it
//! — `nvs run` today — exactly as the configuration snapshot and the origin are,
//! and never rewritten during the run. A context with no table is a program
//! that declared no `#[Command]`, which is § 6's "a program with no `#[Command]`
//! builds no table, runs no scan, and pays nothing" as a member sees it, and it
//! is also every context nobody installed one on. Both answer the same way, so
//! there is one case to write rather than two.
//!
//! **What it spends:** one `Arc` clone per request, over one `String` per name,
//! `about:`, spelling and declared default the program wrote — tens of them for
//! a CLI, and nothing at all for a program with no command. O(in-flight
//! requests), per
//! [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md).
//!
//! # Known gaps
//!
//! 1. **Four of § 6's conversions are [`ArgConv::Unconverted`].** `decimal`, an
//!    enum, a union of literal types and `Core\Uuid` are types
//!    `nvs_types::commands::converts_from_string` admits and no matcher turns
//!    text into yet, so a command declaring one compiles and refuses at the
//!    moment it is *run* rather than at the moment it is written. Each is a
//!    conversion that already exists as a `Core` member; what is missing is the
//!    matcher's arm, not the algorithm.

/// What an argument's text becomes before the handler is called.
///
/// § 6's "a matched value's type comes from the parameter" as the *one* fact
/// about that type which crosses: not the type, but the conversion the checker
/// already picked for it. `nvs_types::commands::ArgConv` is where the choice is
/// made and is the home of the rule; this is the same closed set with the
/// compiler's types taken off it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArgConv {
    /// `string` — the argument's own text, unconverted and `tainted`.
    Text,
    /// `bool` — § 6's flag, which an option is given by being *written* rather
    /// than by carrying text. A positional declared `bool` has no such spelling
    /// to be written, so it reads the words `true` and `false`.
    Flag,
    /// `int` — a signed decimal, and a usage error where the text is not one.
    Int,
    /// `uint` — as [`Self::Int`], and a usage error where the number is
    /// negative.
    Uint,
    /// A type § 6 admits and [`crate::commands`]'s gap 1 does not convert yet.
    Unconverted,
}

/// One argument of a command, in the order it was declared.
///
/// `nvs_types::commands::CommandArg`'s fields that survive the crossing. The
/// declared *type* is not among them — [`ArgConv`] is what a matcher needs of
/// it, and it is one closed set rather than the type lattice.
#[derive(Clone, Debug)]
pub struct CommandArg {
    /// The parameter's own name, sigil-less — what a positional argument is
    /// called in usage text.
    pub param: String,
    /// Every spelling this argument answers to, with the `-`/`--` a command
    /// line writes. **Empty exactly for a positional argument**, which is how
    /// the two are told apart here for the reason the compiler's own row states:
    /// two fields that must agree are two fields that can disagree.
    pub spellings: Vec<String>,
    /// The `#[Option]`'s `about:`, or `None` on a positional argument.
    pub about: Option<String>,
    /// The parameter's declared default, as the text a command line would have
    /// written to supply it — `Some("3")` for § 6's `#[Option] uint $retries =
    /// 3`, and `None` for an argument a command line must fill itself.
    ///
    /// Text rather than a constant, because [`ArgConv`] already turns a word
    /// into the parameter's value and a defaulted argument is then the same
    /// value a written one would have been; `nvs_types::commands::CommandArg`'s
    /// own field is the home of that reasoning. **A flag ignores it** — § 6
    /// gives an option that is `bool` by being *written*, so the one nobody
    /// wrote is `false` whatever its declaration says.
    pub default: Option<String>,
    /// What this argument's text becomes before the handler is called.
    pub conv: ArgConv,
}

impl CommandArg {
    /// Whether this argument is an option rather than a positional — see
    /// [`Self::spellings`].
    #[must_use]
    pub fn is_option(&self) -> bool {
        !self.spellings.is_empty()
    }
}

/// One row of § 6's table: a declared command and the arguments a command line
/// fills.
#[derive(Clone, Debug)]
pub struct Command {
    /// § 6's `name:` — the word a command line selects this command by.
    pub name: String,
    /// § 6's `about:`, the line a usage page renders beside the name, or `None`
    /// where the author wrote none. Nothing is derived from the method's name.
    pub about: Option<String>,
    /// `Class::method` the `#[Command]` was attached to, which is what § 6's
    /// dispatch calls.
    pub handler: String,
    /// Every parameter in declaration order, positionals and options in one
    /// list, because that order *is* the positional order.
    pub args: Vec<CommandArg>,
}

/// Every command the program declares, in the compiler's own load order.
///
/// A `Vec` rather than a map, which is the compiler-side table's shape and its
/// reason: order is what makes a rendered page deterministic for an unchanged
/// program. A duplicate name cannot reach here at all — § 6 makes it a compile
/// error.
#[derive(Clone, Debug, Default)]
pub struct CommandTable {
    rows: Vec<Command>,
}

impl CommandTable {
    /// The table holding `rows`, in the order they are to be rendered.
    #[must_use]
    pub fn new(rows: Vec<Command>) -> Self {
        Self { rows }
    }

    /// Every row, in load order.
    #[must_use]
    pub fn rows(&self) -> &[Command] {
        &self.rows
    }

    /// The row a command line's first word names, or `None` where the program
    /// declares no such command.
    #[must_use]
    pub fn named(&self, name: &str) -> Option<&Command> {
        self.rows.iter().find(|row| row.name == name)
    }
}
