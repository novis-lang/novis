//! `rule:security/secret-sinks-refuse`'s erased read and erased call, and
//! `rule:types/conversion`'s message rule, run end to end: each test compiles
//! a program, runs it, and reads what it printed and the message it caught.
//!
//! The qualifiers are erased before a value exists, so these are the checks
//! the run time owns. The checker's half is `nvs-types`' `qualifiers_hold`.
//!
//! See `tests/common/mod.rs` for the shared fixtures and for why these go
//! through the real pipeline.

mod common;

use common::*;

/// A class with one `secret` property, one `?secret string` property, one
/// plain property, and a method of each kind.
const ACCOUNT: &str = r#"<?nvs
class Account {
    public secret string $token = "s3cr3t";
    public ?secret string $backup = "b4ckup";
    public string $owner = "Shop";

    public function key(): secret string {
        return $this->token;
    }

    public function label(): string {
        return $this->owner;
    }
}

var $account = new Account();
mixed $loose = $account;
"#;

/// What every refused read of `Account::$token` prints.
const TOKEN_REFUSED: &str = "`Account::$token` is secret. A secret property cannot be read \
                             through `mixed`, `object`, a shape type or `Core\\Reflect`. Read it \
                             through a variable of its class.";

#[test]
fn an_erased_read_of_a_secret_property_throws() {
    let source = format!(
        r#"{ACCOUNT}
object $erased = $account;
echo $erased->owner, "\n";
try {{
    var $first = $erased->token;
    echo "read through object\n";
}} catch (LogicError $e) {{
    echo $e->message, "\n";
}}
try {{
    var $second = $loose->backup;
    echo "read through mixed\n";
}} catch (LogicError $e) {{
    echo $e->message, "\n";
}}
try {{
    if ($loose is tainted {{token: string}}) {{
        echo "shape test\n";
    }}
}} catch (LogicError $e) {{
    echo $e->message, "\n";
}}
echo Core\Str::length(Core\Secret::reveal($account->token, "the test reads it")), "\n";
"#
    );
    let output = output_of(&source);
    assert!(!output.contains("s3cr3t"), "{output}");
    assert!(!output.contains("b4ckup"), "{output}");
    assert_eq!(
        output,
        format!(
            "Shop\n{TOKEN_REFUSED}\n`Account::$backup` is secret. A secret property cannot be \
             read through `mixed`, `object`, a shape type or `Core\\Reflect`. Read it through a \
             variable of its class.\n{TOKEN_REFUSED}\n6\n"
        )
    );
}

#[test]
fn a_reflective_read_or_call_never_returns_a_secret_as_plain_text() {
    let source = format!(
        r#"{ACCOUNT}
var $described = Core\Reflect::forObject($account);
echo $described->get($account, "owner"), "\n";
try {{
    var $read = $described->get($account, "token");
    echo "reflective read\n";
}} catch (LogicError $e) {{
    echo $e->message, "\n";
}}
echo $described->call($account, "label", []), "\n";
try {{
    var $called = $described->call($account, "key", []);
    echo "reflective call\n";
}} catch (LogicError $e) {{
    echo $e->message, "\n";
}}
try {{
    var $erased = $loose->key();
    echo "erased call\n";
}} catch (LogicError $e) {{
    echo $e->message, "\n";
}}
echo $loose->label(), "\n";
"#
    );
    let output = output_of(&source);
    assert!(!output.contains("s3cr3t"), "{output}");
    let call_refused = "`Account::key` returns a secret value. It cannot be called through \
                        `mixed` or `Core\\Reflect`. Call it through a variable of its class.";
    assert_eq!(
        output,
        format!("Shop\n{TOKEN_REFUSED}\nShop\n{call_refused}\n{call_refused}\nShop\n")
    );
}

#[test]
fn a_failed_conversion_never_quotes_a_text_operand() {
    let source = r#"<?nvs
class Animal {}

string $pin = "hunter2";
try {
    echo $pin as int, "\n";
} catch (RuntimeError $e) {
    echo $e->message, "\n";
}
try {
    echo $pin as uint, "\n";
} catch (RuntimeError $e) {
    echo $e->message, "\n";
}
try {
    echo $pin as float, "\n";
} catch (RuntimeError $e) {
    echo $e->message, "\n";
}
try {
    "a"|"b" $choice = $pin as "a"|"b";
    echo $choice, "\n";
} catch (RuntimeError $e) {
    echo $e->message, "\n";
}
try {
    class<Animal> $kind = $pin as class<Animal>;
    echo "converted\n";
} catch (RuntimeError $e) {
    echo $e->message, "\n";
}
try {
    echo "x" as int, "\n";
} catch (RuntimeError $e) {
    echo $e->message, "\n";
}
"#;
    let output = output_of(source);
    assert!(!output.contains("hunter2"), "{output}");
    assert_eq!(
        output,
        "cannot convert a `string` of 7 bytes to `int`\n\
         cannot convert a `string` of 7 bytes to `uint`\n\
         cannot convert a `string` of 7 bytes to `float`\n\
         a `string` of 7 bytes is not one of `\"a\"`, `\"b\"`\n\
         cannot convert a `string` to `class<Animal>`: it does not name a class that is a \
         `Animal`\n\
         cannot convert a `string` of 1 byte to `int`\n"
    );
}
