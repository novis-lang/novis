//! `rule:core-classes/db-error`: the text a database server chose reaches a
//! program only as a `tainted` string.
//!
//! `serverMessage` is the server's sentence and `constraint` is a name the
//! server sent, so both are `?tainted string`. A plain `string` target refuses
//! either, which is what keeps them out of a sink that takes trusted text.

mod common;

use common::*;
use nvs_diagnostics::code;

/// A method whose parameter is a `Core\Db\DbError`, with `body` inside it.
fn with_db_error(body: &str) -> String {
    format!(
        "<?nvs\n\
         class T {{\n\
         \x20 function m(Core\\Db\\DbError $e): void {{\n\
         \x20   {body}\n\
         \x20 }}\n\
         }}\n"
    )
}

#[test]
fn the_servers_text_on_a_db_error_is_a_tainted_string() {
    for plain in [
        "string $s = $e->serverMessage ?? \"\";",
        "string $c = $e->constraint ?? \"\";",
    ] {
        let diags = check_src(&with_db_error(plain));
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
            "`{plain}` compiled: {diags:?}"
        );
    }
    for tainted in [
        "?tainted string $s = $e->serverMessage;",
        "?tainted string $c = $e->constraint;",
    ] {
        let diags = check_src(&with_db_error(tainted));
        assert!(
            !diags.has_errors(),
            "`{tainted}` did not compile: {diags:?}"
        );
    }
}
