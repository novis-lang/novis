//! `rule:tooling/commands-are-compiled`'s `#[Command]` and `#[Option]`: the nominal match that keeps a
//! userland spelling out of the command table, and the payload check that is
//! the pass behind both roster entries.
//!
//! What is asserted here is what the two names *are*: recognized after
//! `nvs_hir::resolve_ref`, checked against a roster of options rather than
//! against a shape, held to the two of § 6's three compile errors one parameter
//! list answers on its own — and collected into the table the third is reported
//! over, which is `ExprTypeTable::commands` and is the channel § 6's dispatch,
//! usage text and completions all read.
//!
//! The marker's own placement is asserted here too: an `#[Option]` is refused
//! where no `#[Command]` on the same method reads it, which is a question about
//! the declaration and needs no table at all.

mod common;

use common::{check_src, check_src_table};
use nvs_diagnostics::code;
use nvs_types::commands::ArgConv;
use nvs_types::enums::EnumValue;

/// A user class implementing `Parses`, in the shape
/// `nvs_hir::interfaces::PARSES` spells the contract: one required
/// `parse(tainted string $s): static`, and the text kept in a field declared
/// `tainted` because a class carries no qualifier of its own
/// (`rule:security/tainted-qualifier`).
const SLUG: &str = "class Slug implements Parses {\n  \
                    public tainted string $text = \"\";\n  \
                    public function constructor(tainted string $text) { $this->text = $text; }\n  \
                    public static function parse(tainted string $s): static \
                    { return new static($s); }\n}\n";

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
        "<?nvs\nclass Deploy {\n  #[Core\\Command(name: \"deploy\", about: \"Push it\")]\n  \
         public static function deploy(\n    string $target,\n    \
         #[Core\\Option(short: \"n\", about: \"Print what would happen\")] bool $dryRun,\n  \
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

#[test]
fn two_options_sharing_a_spelling_are_a_diagnostic() {
    // § 6's second compile error. A short spelling is claimed explicitly, so
    // two `short: "n"` on one method leave `-n` with two answers and the
    // parser with none.
    let diags = check_src(&command_src(
        "  #[Command(name: \"deploy\")]\n  public static function deploy(\n    \
         #[Option(short: \"n\")] bool $dryRun,\n    \
         #[Option(short: \"n\")] bool $noColor,\n  ): void {}\n",
    ));
    assert!(diags.has_errors());

    // The long spelling is the parameter's own name unless `long:` gives
    // another, so this collides even though neither declaration repeats a
    // written word — which is the case reading one attribute at a time cannot
    // see, and the reason the walk is per method rather than per payload.
    let diags = check_src(&command_src(
        "  #[Command(name: \"deploy\")]\n  public static function deploy(\n    \
         #[Option] bool $dryRun,\n    \
         #[Option(long: \"dryRun\")] bool $pretend,\n  ): void {}\n",
    ));
    assert!(diags.has_errors());

    // A short and a long that read the same are two spellings, not one: `-n`
    // and `--n` are different arguments, so nothing is claimed twice here.
    let diags = check_src(&command_src(
        "  #[Command(name: \"deploy\")]\n  public static function deploy(\n    \
         #[Option(short: \"n\", long: \"dry-run\")] bool $dryRun,\n    \
         #[Option(long: \"n\")] bool $noColor,\n  ): void {}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    // Two commands may each claim `-n`: the spellings are per method, because
    // a command line names one command before it names any option.
    let diags = check_src(&command_src(
        "  #[Command(name: \"deploy\")]\n  public static function deploy(\n    \
         #[Option(short: \"n\")] bool $dryRun,\n  ): void {}\n  \
         #[Command(name: \"ship\")]\n  public static function ship(\n    \
         #[Option(short: \"n\")] bool $dryRun,\n  ): void {}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_option_on_a_parameter_with_no_conversion_from_string_is_a_diagnostic() {
    // § 6's third compile error, which is `rule:security/route-capture-is-laundered-by-its-type`'s conversion roster
    // applied unchanged: an argument arrives as text, so a parameter no text
    // can become is an option that could never be given.
    let diags = check_src(&command_src(
        "  #[Command(name: \"deploy\")]\n  public static function deploy(\n    \
         #[Option] array<string> $targets,\n  ): void {}\n",
    ));
    assert!(diags.has_errors());

    // `float` is refused where `decimal` is admitted — § 3 names one and not
    // the other, and this is the pair that says the roster is the ADR's rather
    // than "anything numeric".
    let diags = check_src(&command_src(
        "  #[Command(name: \"deploy\")]\n  public static function deploy(\n    \
         #[Option] float $ratio,\n  ): void {}\n",
    ));
    assert!(diags.has_errors());

    let diags = check_src(&command_src(
        "  #[Command(name: \"deploy\")]\n  public static function deploy(\n    \
         #[Option] decimal $ratio,\n    #[Option] uint $retries,\n    \
         #[Option] string $target,\n    #[Option] bool $dryRun,\n  ): void {}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    // A parameter carrying no `#[Option]` is a positional argument and is
    // converted the same way — but it is not this refusal's business, because
    // § 6 gives the marker the meaning and the roster follows the marker.
    let diags = check_src(&command_src(
        "  #[Command(name: \"deploy\")]\n  public static function deploy(\n    \
         array<string> $targets,\n  ): void {}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_option_marker_outside_a_command_method_is_refused() {
    // § 6 gives `#[Option]` its meaning on a `#[Command]` method's parameter,
    // so the same declaration with the `#[Command]` taken away supplies the
    // argument from nowhere. The refusal is the walk over *every* method's
    // doing: the command pass visits only the methods a `#[Command]` marks, so
    // this is the one mistake it cannot see.
    let diags = check_src(&command_src(
        "  public static function deploy(\n    #[Option] string $target,\n  ): void {}\n",
    ));
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_OPTION_WITHOUT_COMMAND)),
        "{diags:?}"
    );
    assert_eq!(diags.error_count(), 1, "{diags:?}");

    // One report per marker, each with its own span, so an author who wrote
    // two is told about both rather than one per rebuild.
    let diags = check_src(&command_src(
        "  public static function deploy(\n    #[Option] string $target,\n    \
         #[Option] bool $dryRun,\n  ): void {}\n",
    ));
    assert_eq!(diags.error_count(), 2, "{diags:?}");

    // A sibling `#[Command]` is what the marker is stray of, and it needs no
    // payload beyond the one the roster already checks — this is the ordinary
    // declaration every other test in this file writes.
    let diags = check_src(&command_src(
        "  #[Command(name: \"deploy\")]\n  public static function deploy(\n    \
         #[Option] string $target,\n  ): void {}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    // Nominal, like every other name on the closed roster: a userland `Option`
    // alias is a different attribute and is nothing this refusal has an
    // opinion about.
    let diags = check_src(
        "<?nvs\ntype Option = {};\nclass Deploy {\n  \
         public static function deploy(#[Option] string $target): void {}\n}\n",
    );
    assert!(
        !diags
            .iter()
            .any(|d| d.code == Some(code::E_OPTION_WITHOUT_COMMAND)),
        "{diags:?}"
    );
}

/// § 6's own example, whole, plus a second command that shares nothing with it
/// — enough for the table to be a table rather than a row.
const PROGRAM: &str = "<?nvs\nuse Core\\Command;\nuse Core\\Option;\nclass Deploy {\n  \
                       #[Command(name: \"deploy\", about: \"Push the current build\")]\n  \
                       public static function deploy(\n    string $target,\n    \
                       #[Option(short: \"n\", about: \"Print what would happen\")] bool $dryRun,\n    \
                       #[Option] uint $retries = 3,\n  ): uint { return 0; }\n\n  \
                       #[Command(name: \"status\")]\n  \
                       public static function status(): void {}\n}\n";

#[test]
fn a_command_table_is_built_from_the_program_enumeration() {
    // § 6 is `rule:routing/routes-are-compiled-not-registered`'s table with the route swapped for a command, over the
    // same `rule:programs/implementing` enumeration, so what is asserted is what the route
    // table's own case asserts: the rows exist, they are in load order, and
    // they are reachable through `ExprTypeTable` — the channel every
    // whole-program fact crosses to `nvs-ir` by. Nothing reads them back yet;
    // `Core\Command::run` is a later goal's.
    let (diags, exprs) = check_src_table(PROGRAM);
    assert!(!diags.has_errors(), "{diags:?}");

    let table = exprs.commands();
    assert_eq!(table.rows().len(), 2);
    // Declaration order within a file, which is the order a duplicate is
    // reported in, so a consumer walking the rows sees the program the
    // diagnostics described.
    let names: Vec<&str> = table.rows().iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, ["deploy", "status"]);

    // The name is the one lookup a command is found by — § 6's dispatch reads
    // exactly this — and `about` is carried because `::help` is generated from
    // the table rather than written by hand.
    let deploy = table.named("deploy").expect("the named command");
    assert_eq!(deploy.handler, "Deploy::deploy");
    assert_eq!(deploy.about.as_deref(), Some("Push the current build"));
    assert!(
        table
            .named("status")
            .expect("the named command")
            .about
            .is_none()
    );
    assert!(table.named("migrate").is_none());

    // "A parameter is a positional argument unless it carries `#[Option]`",
    // and an option that writes no `long:` claims its parameter's own name —
    // so a positional is exactly the argument claiming no spelling, and the
    // declaration order is the positional order.
    let args: Vec<(&str, Vec<&str>)> = deploy
        .args
        .iter()
        .map(|arg| {
            (
                arg.param.as_str(),
                arg.spellings.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    assert_eq!(
        args,
        [
            ("target", vec![]),
            ("dryRun", vec!["-n", "--dryRun"]),
            ("retries", vec!["--retries"]),
        ]
    );
    assert_eq!(
        deploy.args[1].about.as_deref(),
        Some("Print what would happen")
    );

    // `rule:attributes/repeatable`'s repetition: two `#[Command]`s on one method are two names
    // for one implementation, which is what an alias is — the reading
    // `#[Route]` already gets, and the only one that does not silently ignore
    // an attribute the compiler recognizes.
    let (diags, exprs) = check_src_table(&command_src(
        "  #[Command(name: \"deploy\")]\n  #[Command(name: \"ship\")]\n  \
         public static function deploy(): void {}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    let table = exprs.commands();
    assert_eq!(table.rows().len(), 2);
    assert_eq!(
        table.named("ship").expect("the alias").handler,
        "Deploy::deploy"
    );

    // Nominal, like every other name on the closed roster: a userland alias
    // spelled `Command` contributes nothing, which is the whole reason § 6's
    // two names sit on `nvs_types::derive::ATTRIBUTES`.
    let (diags, exprs) = check_src_table(
        "<?nvs\ntype Command = {name: string};\nclass Deploy {\n  \
         #[Command(name: \"deploy\")]\n  public static function deploy(): void {}\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    assert!(exprs.commands().rows().is_empty());

    // "A program with no `#[Command]` builds no table" — read back as empty
    // rather than as absent.
    let (_, exprs) = check_src_table("<?nvs\necho \"\";\n");
    assert!(exprs.commands().rows().is_empty());
}

#[test]
fn a_declared_default_crosses_as_the_text_a_command_line_would_have_written() {
    // § 6 infers nothing from a default — a parameter is positional unless it
    // carries `#[Option]` — so what the row says about one is only whether the
    // command line owes it. `uint $retries = 3` is the section's own example,
    // and it is the argument `Core\Command::run` fills without being told to.
    let (diags, exprs) = check_src_table(PROGRAM);
    assert!(!diags.has_errors(), "{diags:?}");
    let deploy = exprs.commands().named("deploy").expect("the named command");
    let defaults: Vec<Option<&str>> = deploy
        .args
        .iter()
        .map(|arg| arg.default.as_deref())
        .collect();
    assert_eq!(
        defaults,
        [None, None, Some("3")],
        "only the parameter that wrote a default carries one"
    );

    // Every constant a command line could have spelled, in the one form it
    // spells them: the folded value as text, so the matcher's conversion stays
    // the only place a type is decided. The two refusals that bound this list
    // are elsewhere and each is already a diagnostic — a type with no
    // conversion from `string` cannot be an `#[Option]` at all (§ 6's third
    // compile error), and a default that is not a literal of its own declared
    // type is `E_PARAM_DEFAULT_NOT_LITERAL` wherever it is written.
    let (diags, exprs) = check_src_table(
        "<?nvs\nuse Core\\Command;\nuse Core\\Option;\nclass Run {\n  \
         #[Command(name: \"run\")]\n  public static function run(\n    \
         string $tag = \"wip\",\n    #[Option] int $offset = -2,\n    \
         #[Option] bool $loud = true,\n  ): void {}\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    let run = exprs.commands().named("run").expect("the named command");
    let defaults: Vec<Option<&str>> = run.args.iter().map(|arg| arg.default.as_deref()).collect();
    assert_eq!(
        defaults,
        [Some("wip"), Some("-2"), Some("true")],
        "a string arrives cooked and a negative int keeps its sign"
    );
}

/// An enum default crosses as the case's **name**, which is the one form
/// `ArgConv::Enum` converts: the same decision that refuses `--level 10` for
/// `Level::Warn` makes the folded backing value useless as a default's text,
/// since nothing on the other side would accept it back.
#[test]
fn an_enum_default_crosses_as_the_case_name_a_command_line_types() {
    let (diags, exprs) = check_src_table(
        "<?nvs\nuse Core\\Command;\nuse Core\\Option;\nenum Mode { Off = 0, Fast = 3 }\n\
         class Run {\n  #[Command(name: \"run\")]\n  public static function run(\n    \
         #[Option] Mode $mode = Mode::Fast,\n  ): void {}\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    let run = exprs.commands().named("run").expect("the named command");
    let defaults: Vec<Option<&str>> = run.args.iter().map(|arg| arg.default.as_deref()).collect();
    assert_eq!(
        defaults,
        [Some("Fast")],
        "a defaulted enum option carries the word, not the integer it folded to"
    );
}

#[test]
fn a_duplicate_command_name_is_a_diagnostic() {
    // The first of § 6's three compile errors, and the one no declaration can
    // answer on its own: a command line names one command and expects one
    // answer, so which of two methods ran would otherwise depend on the order
    // the files were walked in.
    let diags = check_src(
        "<?nvs\nuse Core\\Command;\nclass Deploy {\n  #[Command(name: \"deploy\")]\n  \
         public static function deploy(): void {}\n}\nclass Ship {\n  \
         #[Command(name: \"deploy\")]\n  public static function ship(): void {}\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_DUPLICATE_COMMAND)),
        "{diags:?}"
    );
    assert_eq!(diags.error_count(), 1, "{diags:?}");

    // Two `#[Command]`s on one method are two rows, so the same collision is
    // reachable without a second method at all.
    let diags = check_src(&command_src(
        "  #[Command(name: \"deploy\")]\n  #[Command(name: \"deploy\")]\n  \
         public static function deploy(): void {}\n",
    ));
    assert_eq!(diags.error_count(), 1, "{diags:?}");

    // Two commands are only a collision when they claim one name; sharing a
    // class, a parameter list or an option spelling is not it.
    let diags = check_src(&command_src(
        "  #[Command(name: \"deploy\")]\n  \
         public static function deploy(#[Option] string $target): void {}\n  \
         #[Command(name: \"ship\")]\n  \
         public static function ship(#[Option] string $target): void {}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_user_class_implementing_parses_may_be_a_command_argument() {
    // § 6's conversion roster is `converts_from_string` unchanged, so a class
    // reaches a command line the way it reaches a route capture — by
    // implementing `Parses` rather than by being named
    // (`rule:security/route-capture-is-laundered-by-its-type`). Both kinds of
    // argument take it: a positional argument and an `#[Option]` are one type
    // list read at two spellings, and § 6's third compile error is the same
    // predicate answered the other way.
    let (diags, exprs) = check_src_table(&format!(
        "<?nvs\n{SLUG}class Deploy {{\n  \
         #[Core\\Command(name: \"deploy\")]\n  \
         public static function deploy(Slug $target, #[Core\\Option] Slug $from): void {{}}\n}}\n"
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    // The class travels *on the row*, because the conversion is the answer
    // itself rather than a lookup key: what the matcher needs is the name of
    // the `parse` it will call, and a rendered type cannot tell a class built
    // from text from an enum whose case spellings are undecided.
    let deploy = exprs.commands().named("deploy").expect("the named command");
    assert_eq!(deploy.args[0].conv, ArgConv::Parses("Slug".to_owned()));
    assert_eq!(deploy.args[1].conv, ArgConv::Parses("Slug".to_owned()));

    // The refusal is the same predicate read the other way: a class that does
    // not implement the interface has no conversion from text, so § 6's third
    // compile error is where an argument declared at one is reported.
    let diags = check_src(&format!(
        "<?nvs\n{SLUG}class Plain {{}}\nclass Deploy {{\n  \
         #[Core\\Command(name: \"deploy\")]\n  \
         public static function deploy(#[Core\\Option] Plain $from): void {{}}\n}}\n"
    ));
    assert!(diags.has_errors(), "{diags:?}");
}

#[test]
fn a_subset_of_an_enums_cases_converts_by_the_same_words_the_whole_enum_does() {
    // § 3's union of enum cases, which § 6 admits wherever it admits the enum
    // itself. The row is `ArgConv::Enum` with the cases the union did not name
    // dropped — a filter over one roster rather than a second reading of it —
    // so the word a command line writes and the value it becomes are decided in
    // one place for `Level` and for a subset of `Level` alike.
    let src = |declared: &str| {
        format!(
            "<?nvs\nenum Level: uint {{\n  Quiet = 1,\n  Warn = 10,\n  Error = 20,\n}}\n\
             class Logs {{\n  #[Core\\Command(name: \"emit\")]\n  \
             public static function emit({declared} $level): void {{}}\n}}\n"
        )
    };
    let (diags, exprs) = check_src_table(&src("Level::Error|Level::Warn"));
    assert!(!diags.has_errors(), "{diags:?}");
    let emit = exprs.commands().named("emit").expect("the named command");
    assert_eq!(
        emit.args[0].conv,
        ArgConv::Enum {
            class: "Level".to_owned(),
            // Ascending by value and not in the order the union wrote them:
            // a set the compiler renders into a usage line or a refusal has to
            // read the same on two builds, which is the whole enum's rule read
            // through the filter rather than a second one.
            cases: vec![
                ("Warn".to_owned(), EnumValue::Uint(10)),
                ("Error".to_owned(), EnumValue::Uint(20)),
            ],
        }
    );

    // One case on its own is that subset written with one member, which is the
    // arrangement a lone single-value type already has beside a union of them.
    let (diags, exprs) = check_src_table(&src("Level::Quiet"));
    assert!(!diags.has_errors(), "{diags:?}");
    let emit = exprs.commands().named("emit").expect("the named command");
    assert_eq!(
        emit.args[0].conv,
        ArgConv::Enum {
            class: "Level".to_owned(),
            cases: vec![("Quiet".to_owned(), EnumValue::Uint(1))],
        }
    );

    // The whole enum is the same row with nothing filtered out, asserted here
    // beside the subset because "they cannot disagree" is the claim the filter
    // is shaped to make true.
    let (diags, exprs) = check_src_table(&src("Level"));
    assert!(!diags.has_errors(), "{diags:?}");
    let emit = exprs.commands().named("emit").expect("the named command");
    assert_eq!(
        emit.args[0].conv,
        ArgConv::Enum {
            class: "Level".to_owned(),
            cases: vec![
                ("Quiet".to_owned(), EnumValue::Uint(1)),
                ("Warn".to_owned(), EnumValue::Uint(10)),
                ("Error".to_owned(), EnumValue::Uint(20)),
            ],
        }
    );
}

#[test]
fn an_option_declared_at_cases_of_two_enums_is_a_diagnostic() {
    // § 6 admits a subset of *one* enum's cases: a union spanning two leaves no
    // enum for an admitted word to be a case of, so there is no conversion to
    // choose and it is refused where it is written rather than at the moment
    // the command is run. `nvs_types::routes::admitted_cases` is the one home
    // of that test, and a route capture declared at such a union is refused by
    // the same answer read through `enum_capture`.
    let two = "enum Level {\n  Quiet,\n  Loud,\n}\nenum Mode {\n  Fast,\n  Slow,\n}\n";
    let diags = check_src(&format!(
        "<?nvs\n{two}class Radio {{\n  #[Core\\Command(name: \"tune\")]\n  \
         public static function tune(#[Core\\Option] Level::Loud|Mode::Fast $level): void {{}}\n}}\n"
    ));
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_OPTION_TYPE_HAS_NO_CONVERSION)),
        "{diags:?}"
    );

    // Cases of one enum in the same position are the conversion above, so what
    // the refusal answers is the span of the union and not the spelling.
    let diags = check_src(&format!(
        "<?nvs\n{two}class Radio {{\n  #[Core\\Command(name: \"tune\")]\n  \
         public static function tune(#[Core\\Option] Level::Loud|Level::Quiet $level): void {{}}\n}}\n"
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_command_that_names_nothing_is_a_diagnostic() {
    // § 6 leaves it unwritten and the table pass is what decides it: `name` is
    // the word a command line selects a command by, so a row without one is
    // reachable by nothing — `nvs_types::commands`' module docs own the
    // reasoning, and `rule:routing/route-attribute`'s optional `name` is a different question
    // because a route is reached by its path.
    for attributes in [
        "  #[Command]\n  public static function deploy(): void {}\n",
        "  #[Command(about: \"Push it\")]\n  public static function deploy(): void {}\n",
    ] {
        let diags = check_src(&command_src(attributes));
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_COMMAND_WITHOUT_NAME)),
            "{diags:?}"
        );
        assert_eq!(diags.error_count(), 1, "{diags:?}");
    }

    // A `name` written at the wrong type is the roster's report and only the
    // roster's: naming it a second time as a missing field would put the
    // author's second problem before their first.
    let diags = check_src(&command_src(
        "  #[Command(name: 1)]\n  public static function deploy(): void {}\n",
    ));
    assert!(
        !diags
            .iter()
            .any(|d| d.code == Some(code::E_COMMAND_WITHOUT_NAME)),
        "{diags:?}"
    );
    assert_eq!(diags.error_count(), 1, "{diags:?}");
}
