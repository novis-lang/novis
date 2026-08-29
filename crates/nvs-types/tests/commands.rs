//! ADR 0086 § 6's `#[Command]` and `#[Option]`: the nominal match that keeps a
//! userland spelling out of the command table, and the payload check that is
//! the pass behind both roster entries.
//!
//! The table itself has no rows yet — `nvs_types::commands`' gap 1 owns what §
//! 6 still cannot report — so what is asserted here is exactly what the two
//! names *are* today: recognized after `nvs_hir::resolve_ref`, and checked
//! against a roster of options rather than against a shape.

mod common;

use common::check_src;

/// § 6's own example, reduced to the two attributes and the one class member
/// they attach to. The placing import is per *name* — `use Core\Command;`
/// aliases `Command`, exactly as `use Core\Test;` aliases `Test` for
/// `#[Test]`; a `use Core;` aliases only `Core` itself and leaves a bare
/// `#[Command]` resolving to `\Command`.
fn command_src(attributes: &str) -> String {
    format!("<?nvs\nuse Core\\Command;\nuse Core\\Option;\nclass Deploy {{\n{attributes}\n}}\n")
}

#[test]
fn a_command_and_an_option_are_matched_nominally_rather_than_as_shapes() {
    // Fully qualified needs no import at all. Before both names joined
    // `nvs_types::derive::ATTRIBUTES` this was `E0726` four times over —
    // `Core\Command` is not a `type` alias and was never going to be one,
    // because § 6 builds a table from it and a userland alias must not
    // contribute a command.
    let diags = check_src(
        "<?nvs\nclass Deploy {\n  #[\\Core\\Command(name: \"deploy\", about: \"Push it\")]\n  \
         public static function deploy(\n    string $target,\n    \
         #[\\Core\\Option(short: \"n\", about: \"Print what would happen\")] bool $dryRun,\n  \
         ): void {}\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");

    // The `use`d bare spelling is the same attribute, and the bare one with
    // nothing importing it resolves to `\Command` — no declaration at all, so
    // it is the ordinary undeclared-name refusal rather than a silently
    // ignored attribute.
    let diags = check_src(&command_src(
        "  #[Command(name: \"deploy\")]\n  public static function deploy(): void {}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    let diags = check_src(
        "<?nvs\nclass Deploy {\n  #[Command(name: \"deploy\")]\n  \
         public static function deploy(): void {}\n}\n",
    );
    assert!(diags.has_errors());
}

#[test]
fn a_bare_option_is_the_whole_attribute_and_carries_no_payload() {
    // § 6: a parameter is a positional argument unless it carries `#[Option]`.
    // The marker on its own is what most parameters will write, so it has to
    // pass the payload check that an empty field list walks zero times.
    let diags = check_src(&command_src(
        "  #[Command(name: \"deploy\")]\n  public static function deploy(\n    \
         string $target,\n    #[Option] bool $dryRun,\n  ): void {}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn each_attribute_admits_only_its_own_options() {
    // `short` is `#[Option]`'s and is no field of `#[Command]`; the roster is
    // per attribute rather than one pooled set, so a plausible field borrowed
    // from the neighbouring attribute is still a typo.
    let diags = check_src(&command_src(
        "  #[Command(name: \"deploy\", short: \"d\")]\n  \
         public static function deploy(): void {}\n",
    ));
    assert!(diags.has_errors());

    let diags = check_src(&command_src(
        "  #[Command(name: \"deploy\")]\n  \
         public static function deploy(#[Option(name: \"dry\")] bool $dryRun): void {}\n",
    ));
    assert!(diags.has_errors());
}

#[test]
fn an_option_value_is_a_string_and_is_written_once() {
    // Every option of both attributes is a `string`, so a payload that reads
    // plausibly — `about: 1` — is refused at the value rather than accepted
    // and folded to something the help text cannot render.
    let diags = check_src(&command_src(
        "  #[Command(name: \"deploy\", about: 1)]\n  \
         public static function deploy(): void {}\n",
    ));
    assert!(diags.has_errors());

    let diags = check_src(&command_src(
        "  #[Command(name: \"deploy\", name: \"ship\")]\n  \
         public static function deploy(): void {}\n",
    ));
    assert!(diags.has_errors());
}
