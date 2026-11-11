use nvs_runtime::{ClassDesc, ClassTable, Ctx, OutputSink, SlotCheck, Tag, decode, encode};

/// `Dog`, `Cat`, a typed `Owner`, and one class of every kind `encode`
/// refuses, each with the one property `name`.
fn classes() -> ClassTable {
    let mut table = ClassTable::new();
    table.define("Dog", &["name"], &[]);
    table.define("Cat", &["name"], &[]);
    let callable = table.define("{callable}", &["name"], &[]);
    table.set_callable(callable);
    table.define("Task$gen", &["name"], &[]);
    let handle = table.define(r"Core\IO\File", &["name"], &[]);
    table.set_host_handle(handle);
    table.define("main$fn0", &["name"], &[]);
    let wallet = table.define("Wallet", &["name"], &[]);
    table.set_secret_fields(wallet, vec![true]);
    let owner = table.define("Owner", &["count", "pet", "pets"], &[]);
    table.set_field_tags(
        owner,
        vec![Some(Tag::Int), Some(Tag::Object), Some(Tag::Array)],
    );
    table.set_field_types(owner, vec!["int".into(), "Dog".into(), "array<Dog>".into()]);
    table.set_field_classes(owner, vec![None, Some(vec!["Dog".into()]), None]);
    table.set_field_checks(
        owner,
        vec![
            None,
            None,
            Some(SlotCheck::Array(Box::new(SlotCheck::Class("Dog".into())))),
        ],
    );
    table
}

/// The descriptor for `name`: the table's own class, or `Core\Uri`, a `Core`
/// class that states the type of each of its eight slots.
fn class_desc(table: &ClassTable, name: &str) -> Option<*const ClassDesc> {
    table.id_of(name).map(|id| table.desc(id)).or_else(|| {
        (name == nvs_stdlib::uri::NAME)
            .then_some(name)
            .and_then(nvs_stdlib::core_class_desc)
    })
}

/// Re-encodes `value`, consuming the caller's reference to it.
///
/// `encode` refuses every class the table marks, so this fails when `decode`
/// returned an object of one.
fn encoded(value: nvs_runtime::Value) -> Vec<u8> {
    encode(value).expect("a value `decode` built holds no class `encode` refuses")
}

/// `Core\Serialize::decode` must not panic on any payload, and whatever it
/// accepts must encode again to a payload that decodes and encodes to the same
/// bytes.
///
/// The class table is the one a forged payload meets: `Owner` declares an
/// `int`, a `Dog` and an `array<Dog>` field, and `Cat` has the same shape as
/// `Dog`. So the fuzzer reaches the per-field type checks that keep a decoded
/// object from holding a value its slot does not admit
/// (`rule:classes/serialize-is-a-closed-format`), which is where a missed check
/// is a memory-safety hole rather than a wrong answer. The table also holds a
/// callable's class, a generator's state class, a host-handle class, a class
/// the compiler made and a class with a `secret` property, which `decode` must
/// refuse by name, as `encode` does. `Core\Uri` resolves too, and a decoded
/// object of it must hold a value of its stated type in every slot.
///
/// It runs inside a context, as a request decodes: a payload whose objects form
/// a cycle keeps them alive past the last release, and the context's teardown
/// frees them, as the end of a request does. With no context they would
/// outlive the run and be reported as a leak that no request has.
pub fn serialize(data: &[u8]) {
    let table = classes();
    let resolve = |name: &str| class_desc(&table, name);
    let mut ctx = Ctx::new(OutputSink::Sink);
    ctx.enter(|| {
        let Ok(once) = decode(data, &resolve) else {
            return;
        };
        if once.class_name().as_deref() == Some(nvs_stdlib::uri::NAME)
            && let Err(why) = nvs_stdlib::fuzz::check_fields(once)
        {
            panic!("`decode` built a `Core\\Uri` whose slot does not fit its stated type: {why}");
        }
        let first = encoded(once);
        let twice = decode(&first, &resolve).expect("a payload `encode` wrote decodes again");
        let second = encoded(twice);
        assert_eq!(first, second, "one round trip settles the payload");
    });
}
