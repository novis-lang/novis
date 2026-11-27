//! Each function's key covers everything its code reads, and nothing about
//! where it sits: a body edit moves its own key and no other, every
//! declaration a body reads moves the key of the function that reads it, and
//! a function moved in the program keeps its bytes.
//!
//! See `tests/common/mod.rs` for the shared fixtures, and
//! `nvs_codegen::function_keys` for the keys these tests read.

mod common;

use common::*;
use object::{Object, ObjectSection, ObjectSymbol};

/// Every function of `source`'s program, by Novis name, with its key.
fn keys(source: &str) -> Vec<(String, [u8; 32])> {
    nvs_codegen::function_keys(&lower(source)).expect("the fixture's functions emit")
}

fn key_of(keys: &[(String, [u8; 32])], name: &str) -> [u8; 32] {
    keys.iter()
        .find(|(function, _)| function == name)
        .map(|(_, key)| *key)
        .unwrap_or_else(|| panic!("the fixture has no `{name}`"))
}

/// Three classes whose methods do not call one another, and a script that
/// calls each. `body` is `B::middle`'s return expression.
fn three_classes(body: &str) -> String {
    format!(
        "<?nvs
class A {{
    public static function first(int $n): int {{
        echo \"a\";
        return $n + 1;
    }}
}}
class B {{
    public static function middle(int $n): int {{
        echo \"b\";
        return {body};
    }}
}}
class C {{
    public static function last(int $n): int {{
        if ($n > 2) {{
            echo \"c\";
        }}
        return $n * 3;
    }}
}}
echo A::first(1), B::middle(2), C::last(3);
"
    )
}

#[test]
fn a_body_edit_moves_only_its_own_key() {
    let before = keys(&three_classes("$n + 2"));
    let after = keys(&three_classes("$n + 2 + $n * 7"));
    assert_eq!(before.len(), after.len());
    for ((name, old), (same, new)) in before.iter().zip(&after) {
        assert_eq!(name, same, "the edit changed the function order");
        match name.as_str() {
            "B::middle" => assert_ne!(old, new, "`B::middle`'s edit did not move its key"),
            _ => assert_eq!(old, new, "`B::middle`'s edit moved `{name}`'s key"),
        }
    }
}

/// One declaration a body reads, written two ways. `reader` reads it and
/// `bystander` does not.
struct Declaration {
    /// The unit table the declaration lives in, named in a failure.
    table: &'static str,
    before: String,
    after: String,
    reader: &'static str,
    bystander: &'static str,
}

/// `Bystander::run` reads nothing the cases below change.
const BYSTANDER: &str = "class Bystander {
    public static function run(int $n): int {
        echo \"x\";
        return $n - 1;
    }
}
";

fn declarations() -> Vec<Declaration> {
    let program = |declarations: &str, reader: &str, script: &str| {
        format!("<?nvs\n{BYSTANDER}{declarations}{reader}echo Bystander::run(1);\n{script}")
    };
    let field_reader = "class Read {
    public static function field(Point $p): int {
        return $p->y;
    }
}
";
    let static_reader = "class Read {
    public static function slot(): int {
        return Counter::$hits;
    }
}
";
    let call_reader = |callee: &str| {
        format!(
            "class Read {{
    public static function call(int $n): int {{
        return Callee::{callee}($n);
    }}
}}
"
        )
    };
    vec![
        // A property added before the one the body reads moves its offset.
        Declaration {
            table: "classes (a property's offset)",
            before: program(
                "class Point {\n    public int $x = 0;\n    public int $y = 0;\n}\n",
                field_reader,
                "echo Read::field(new Point());\n",
            ),
            after: program(
                "class Point {\n    public int $w = 0;\n    public int $x = 0;\n    public int $y = 0;\n}\n",
                field_reader,
                "echo Read::field(new Point());\n",
            ),
            reader: "Read::field",
            bystander: "Bystander::run",
        },
        // A static property declared before the one the body reads moves its
        // slot.
        Declaration {
            table: "statics (a static property's slot)",
            before: program(
                "class Counter {\n    public static int $hits = 4;\n}\n",
                static_reader,
                "echo Read::slot();\n",
            ),
            after: program(
                "class Counter {\n    public static int $misses = 0;\n    public static int $hits = 4;\n}\n",
                static_reader,
                "echo Read::slot();\n",
            ),
            reader: "Read::slot",
            bystander: "Bystander::run",
        },
        // The function the body calls is another one.
        Declaration {
            table: "functions (a callee)",
            before: program(
                "class Callee {\n    public static function one(int $n): int { return $n; }\n    public static function two(int $n): int { return $n + 2; }\n}\n",
                &call_reader("one"),
                "echo Read::call(1), Callee::two(1);\n",
            ),
            after: program(
                "class Callee {\n    public static function one(int $n): int { return $n; }\n    public static function two(int $n): int { return $n + 2; }\n}\n",
                &call_reader("two"),
                "echo Read::call(1), Callee::one(1);\n",
            ),
            reader: "Read::call",
            bystander: "Bystander::run",
        },
        // The callee's body changes, and the caller's code does not: the
        // caller names its callee, and never the callee's code.
        Declaration {
            table: "functions (a callee's body is not a caller's input)",
            before: program(
                "class Callee {\n    public static function one(int $n): int { return $n; }\n}\n",
                &call_reader("one"),
                "echo Read::call(1);\n",
            ),
            after: program(
                "class Callee {\n    public static function one(int $n): int { return $n * 5; }\n}\n",
                &call_reader("one"),
                "echo Read::call(1);\n",
            ),
            reader: "Callee::one",
            bystander: "Read::call",
        },
    ]
}

#[test]
fn every_declaration_a_body_reads_is_in_its_key() {
    for case in declarations() {
        let before = keys(&case.before);
        let after = keys(&case.after);
        assert_ne!(
            key_of(&before, case.reader),
            key_of(&after, case.reader),
            "{}: a change `{}` reads did not move its key",
            case.table,
            case.reader
        );
        assert_eq!(
            key_of(&before, case.bystander),
            key_of(&after, case.bystander),
            "{}: a change `{}` does not read moved its key",
            case.table,
            case.bystander
        );
    }
}

/// A program with `Gate::less` after a class whose one method has
/// `statements` statements, so `Gate::less`'s first statement, edge and
/// query numbers all move with `statements`. The statements share one line,
/// so `Gate::less` keeps its lines: a backtrace prints them, so they are in
/// its key.
fn moved(statements: usize) -> String {
    let mut body = String::new();
    for i in 0..statements {
        body.push_str(&format!("if ($n > {i}) {{ echo \"{i}\"; }} "));
    }
    format!(
        "<?nvs
class Before {{
    public static function run(int $n): int {{
        {body}
        return $n;
    }}
}}
class Gate {{
    public static function less(int $a, int $b): bool {{
        if ($a < $b) {{
            echo \"less\";
        }}
        return $a < $b;
    }}
}}
echo Before::run(3), Gate::less(1, 2);
"
    )
}

/// The bytes the object `program` compiles to holds for the function whose
/// symbol `named` accepts, from its symbol to the next symbol in its section.
fn function_bytes(object: &[u8], named: impl Fn(&str) -> bool) -> Vec<u8> {
    let file = object::File::parse(object).expect("the compiler writes an object it can read");
    let symbol = file
        .symbols()
        .find(|symbol| symbol.name().is_ok_and(&named))
        .expect("the object has the function");
    let section_index = symbol.section_index().expect("the function is defined");
    let section = file
        .section_by_index(section_index)
        .expect("the function's section");
    let start = symbol.address();
    let end = file
        .symbols()
        .filter(|other| other.section_index() == Some(section_index))
        .map(|other| other.address())
        .filter(|address| *address > start)
        .min()
        .unwrap_or_else(|| section.address() + section.size());
    let data = section.data().expect("the section has bytes");
    let from = usize::try_from(start - section.address()).expect("an offset in the section");
    let to = usize::try_from(end - section.address()).expect("an offset in the section");
    data[from..to].to_vec()
}

/// Whether `symbol` is the one the compiler names `Gate::less` by: its index
/// in the program, then the sanitised name.
fn is_gate_less(symbol: &str) -> bool {
    let symbol = symbol.trim_start_matches('_');
    symbol
        .strip_prefix("nvs")
        .and_then(|rest| rest.split_once('_'))
        .is_some_and(|(index, name)| {
            !index.is_empty() && index.bytes().all(|b| b.is_ascii_digit()) && name == "Gate__less"
        })
}

#[test]
fn a_function_moved_in_the_program_keeps_its_bytes() {
    let near = lower(&moved(1));
    let far = lower(&moved(9));
    let near_object = nvs_codegen::compile_object(&near).expect("the near fixture compiles");
    let far_object = nvs_codegen::compile_object(&far).expect("the far fixture compiles");

    // The function reads its first statement's number through a relocation,
    // so the case reaches what it is about.
    let file = object::File::parse(&near_object[..]).expect("an object");
    assert!(
        file.symbols().any(|symbol| symbol
            .name()
            .is_ok_and(|name| name.trim_start_matches('_') == "nvs_base_stmt_Gate_3a_3aless")),
        "`Gate::less` does not relocate against its statement base"
    );

    assert_eq!(
        function_bytes(&near_object, is_gate_less),
        function_bytes(&far_object, is_gate_less),
        "`Gate::less` compiled to other bytes when the function before it grew"
    );
    let near_keys = nvs_codegen::function_keys(&near).expect("the near fixture emits");
    let far_keys = nvs_codegen::function_keys(&far).expect("the far fixture emits");
    assert_eq!(
        key_of(&near_keys, "Gate::less"),
        key_of(&far_keys, "Gate::less")
    );
    assert_ne!(
        key_of(&near_keys, "Before::run"),
        key_of(&far_keys, "Before::run")
    );

    // Linked and run, both programs print what their source says.
    assert_eq!(output_of_program(&near), "03less1");
    assert_eq!(output_of_program(&far), "0123less1");
}
