//! `rule:errors/a-use-of-deprecated-code-may-log-or-throw`'s compiled check: where it is emitted,
//! and what it does under each value of the context's deprecation word.
//!
//! See `tests/common/mod.rs` for the shared fixtures and for why these go
//! through the real pipeline.

mod common;

use common::*;
use nvs_ir::ir::InstKind;
use nvs_runtime::OnDeprecated;

/// Every deprecation check in `program`, as `(function, message)`, in the
/// order the functions and their blocks are laid out.
fn checks(program: &nvs_ir::Program) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for f in &program.functions {
        for block in &f.blocks {
            for inst in &block.insts {
                if let InstKind::DeprecationCheck { message, .. } = &inst.kind {
                    out.push((f.name.clone(), message.clone()));
                }
            }
        }
    }
    out
}

/// Runs `source` with the deprecation word set to `on`. Returns what it
/// printed, or the pending message of what it threw.
fn run_under(on: OnDeprecated, source: &str) -> Result<String, String> {
    let mut ctx = Ctx::buffered();
    ctx.set_on_deprecated(on);
    match run_with(&mut ctx, source) {
        Ok(_) => Ok(
            String::from_utf8(ctx.take_buffered_output().expect("a buffered context"))
                .expect("the script echoed UTF-8"),
        ),
        Err(status) => {
            assert_eq!(status, THROWN, "the script stopped without a throw");
            Err(ctx.pending().expect("a pending exception").into_owned())
        }
    }
}

/// A deprecated method, called through an interface that does not deprecate
/// it.
const A_METHOD: &str = "<?nvs\n\
interface Sized { function size(): int; }\n\
class Store implements Sized {\n  \
  public int $count = 3;\n  \
  #[Core\\Deprecated(since: '2.0', replace: '$this->count')]\n  \
  function size(): int { return $this->count; }\n\
}\n\
Sized $s = new Store();\n\
echo $s->size(), \"\\n\";\n";

#[test]
fn a_deprecated_method_checks_the_setting_on_entry() {
    // The check sits in the method itself and nowhere else: a call through
    // `Sized`, which is not deprecated, still reaches it.
    let found = checks(&lower(A_METHOD));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "Store::size");
    assert!(
        found[0].1.contains("is deprecated since 2.0.") && found[0].1.contains("$this->count"),
        "{found:?}"
    );

    assert_eq!(
        run_under(OnDeprecated::Ignore, A_METHOD).as_deref(),
        Ok("3\n")
    );
    assert_eq!(run_under(OnDeprecated::Log, A_METHOD).as_deref(), Ok("3\n"));
    let thrown = run_under(OnDeprecated::Throw, A_METHOD).expect_err("the entry check throws");
    assert_eq!(thrown, found[0].1);
}

/// One declaration of each kind a use names: a property, a class constant, an
/// enum case, a class, and a parameter.
const DECLARATIONS: &str = "<?nvs\n\
class Store {\n  \
  #[Core\\Deprecated(replace: '$this->count')]\n  \
  public int $total = 1;\n  \
  public int $count = 1;\n  \
  #[Core\\Deprecated(since: '2.0')]\n  \
  const int LIMIT = 2;\n  \
  function size(#[Core\\Deprecated] int $limit = 0): int { return $limit; }\n\
}\n\
enum Status { #[Core\\Deprecated(since: '2.0')] Active, Banned }\n\
#[Core\\Deprecated(since: '2.0')]\n\
class Shop { function name(): string { return \"shop\"; } }\n\
Store $s = new Store();\n";

/// Each use, what it prints when nothing stops it, and the declaration its
/// check names.
const USES: &[(&str, &str, &str)] = &[
    ("echo $s->total, \"\\n\";", "1\n", "`Store::$total`"),
    ("echo Store::LIMIT, \"\\n\";", "2\n", "`Store::LIMIT`"),
    (
        "echo Status::Active == Status::Banned ? \"y\" : \"n\", \"\\n\";",
        "n\n",
        "`Status::Active`",
    ),
    ("echo (new Shop())->name(), \"\\n\";", "shop\n", "`Shop`"),
    ("echo $s->size(4), \"\\n\";", "4\n", "`$limit`"),
];

#[test]
fn a_deprecated_property_constant_case_class_and_parameter_check_at_the_use() {
    for (code, printed, what) in USES {
        let source = format!("{DECLARATIONS}{code}\n");
        // One check, in the script that holds the use and not in any method.
        let found = checks(&lower(&source));
        assert_eq!(found.len(), 1, "{code}: {found:?}");
        assert_eq!(found[0].0, nvs_ir::lower::ENTRY_SCRIPT_LABEL, "{code}");
        assert!(found[0].1.contains(what), "{code}: {found:?}");

        assert_eq!(
            run_under(OnDeprecated::Ignore, &source).as_deref(),
            Ok(*printed),
            "{code}"
        );
        let thrown =
            run_under(OnDeprecated::Throw, &source).expect_err("the use throws before it prints");
        assert_eq!(thrown, found[0].1, "{code}");
    }
}

#[test]
fn a_program_with_no_deprecated_use_emits_no_deprecation_check() {
    // Deprecated declarations that nothing uses, and a method of a deprecated
    // class that is not itself deprecated, cost nothing at all.
    let source = "<?nvs\n\
        #[Core\\Deprecated(since: '2.0')]\n\
        class Old { function run(): int { return 1; } }\n\
        class Store {\n  \
          #[Core\\Deprecated(since: '2.0')]\n  \
          const int LIMIT = 2;\n  \
          public int $count = 1;\n  \
          function size(): int { return $this->count; }\n\
        }\n\
        echo (new Store())->size(), \"\\n\";\n";
    assert_eq!(checks(&lower(source)), Vec::new());
    assert_eq!(run_under(OnDeprecated::Throw, source).as_deref(), Ok("1\n"));
    assert_eq!(checks(&lower(A_LOOP_WITH_NOTHING_DEPRECATED)), Vec::new());
}

const A_LOOP_WITH_NOTHING_DEPRECATED: &str =
    "<?nvs\nint $i = 0;\nwhile ($i < 3) {\n    $i = $i + 1;\n}\necho $i;\n";
