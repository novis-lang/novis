//! The intl component as `nvs-ext` sees it (`rule:packaging/the-first-party-components-are-built-in`,
//! `rule:core-classes/intl-batch-shape`): `Novis\Intl\Icu` is in the binary and answers with no
//! `[[extension]]` entry, a sort of 10,000 strings is one crossing, its sort keys order as its
//! collator does, and the order is the locale's.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use nvs_ext::builtin::{INTL, INTL_CLASS, INTL_SHA256};
use nvs_ext::call::{Error, Failure, Host, Meter, Request};
use nvs_ext::convert::{Key, Value};
use nvs_ext::load::{Builtin, Extension, Loader};

/// The intl component among `builtins`.
fn intl(builtins: &[Builtin]) -> &Builtin {
    builtins
        .iter()
        .find(|builtin| builtin.manifest().class == INTL_CLASS)
        .expect("the intl component is built in")
}

/// `future` polled on this thread until it finishes. A guest call yields at every epoch tick, and
/// polling again resumes it.
fn block_on<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(out) = future.as_mut().poll(&mut cx) {
            return out;
        }
        std::thread::yield_now();
    }
}

fn list(items: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(
        items
            .into_iter()
            .enumerate()
            .map(|(index, item)| (Key::Int(i64::try_from(index).unwrap()), item))
            .collect(),
    )
}

fn strings(items: &[String]) -> Value {
    list(items.iter().cloned().map(Value::String))
}

fn items(value: Value) -> Vec<Value> {
    let Value::Array(entries) = value else {
        panic!("a list was expected, not {value:?}");
    };
    entries.into_iter().map(|(_, item)| item).collect()
}

/// `method` called with `strings`, `locale` and no options.
fn collate(
    request: &Request,
    extension: &Extension,
    method: &str,
    input: &[String],
    locale: &str,
) -> Result<Vec<Value>, Failure> {
    let args = vec![
        strings(input),
        Value::String(locale.to_owned()),
        Value::Array(Vec::new()),
    ];
    let result = block_on(request.call_values(extension, method, args))?;
    Ok(items(result.expect("the method returns a value")))
}

/// The order `collateOrder` returns, as indexes.
fn order(request: &Request, extension: &Extension, input: &[String], locale: &str) -> Vec<usize> {
    collate(request, extension, "collateOrder", input, locale)
        .expect("`collateOrder` answers")
        .into_iter()
        .map(|index| match index {
            Value::Uint(index) => usize::try_from(index).unwrap(),
            other => panic!("an index was expected, not {other:?}"),
        })
        .collect()
}

fn keys(request: &Request, extension: &Extension, input: &[String], locale: &str) -> Vec<Vec<u8>> {
    collate(request, extension, "sortKeys", input, locale)
        .expect("`sortKeys` answers")
        .into_iter()
        .map(|key| match key {
            Value::Bytes(key) => key,
            other => panic!("a key was expected, not {other:?}"),
        })
        .collect()
}

/// `count` names from a fixed seed, with accented letters and both cases, so ties and tailorings
/// are both in the batch.
fn names(count: usize) -> Vec<String> {
    const LETTERS: [&str; 16] = [
        "a", "b", "e", "k", "o", "s", "z", "A", "Z", "ä", "ö", "é", "Å", "ß", "-", " ",
    ];
    let nibble = |state: u64, i: usize| usize::try_from((state >> (4 * i)) & 0xF).unwrap();
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    (0..count)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let len = 1 + nibble(state, 15) % 8;
            (0..len).map(|i| LETTERS[nibble(state, i)]).collect()
        })
        .collect()
}

fn run<T>(test: impl FnOnce(&Request, &Extension) -> T) -> T {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let builtins = Loader::new(host.engine())
        .builtins()
        .expect("the built-in components load");
    let extension = intl(&builtins)
        .extension()
        .expect("the intl component compiles and checks");
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(60), None)));
    let out = test(&request, extension);
    block_on(request.end()).expect("the request ends");
    out
}

#[test]
fn the_intl_component_is_built_in_and_answers_with_no_configuration() {
    let host = Host::new(4, |_| Ok(())).expect("the host starts");
    let builtins = Loader::new(host.engine())
        .builtins()
        .expect("the built-in components load");
    let builtin = intl(&builtins);
    let digest: String = INTL_SHA256.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(builtin.sha256(), digest);
    assert_eq!(builtin.sha256(), nvs_ext::load::pin(INTL));
    assert!(!builtin.compiled(), "loading compiled the component");
    let extension = builtin
        .extension()
        .expect("the intl component compiles and checks");
    assert_eq!(extension.memory, None);
    assert_eq!(extension.grants, nvs_config::extension::Granted::default());
    let request = host.request(Arc::new(Meter::new(Duration::from_secs(30), None)));
    let input = ["b".to_owned(), "a".to_owned()];
    assert_eq!(order(&request, extension, &input, "en"), [1, 0]);
    match collate(&request, extension, "collateOrder", &input, "en_US!") {
        Err(Failure::Error(Error::Invalid(message))) => {
            assert!(message.contains("en_US!"), "{message}");
        }
        other => panic!("a malformed tag was not `invalid`: {other:?}"),
    }
    match collate(&request, extension, "formatNumbers", &input, "en") {
        Err(Failure::Error(Error::Runtime(_)) | Failure::Error(Error::Invalid(_))) => {}
        other => panic!("an export not implemented yet answered: {other:?}"),
    }
    block_on(request.end()).expect("the request ends");
}

#[test]
fn sorting_ten_thousand_strings_is_one_crossing() {
    let input = names(10_000);
    let (sorted, crossings) = run(|request, extension| {
        let before = request.crossings();
        let sorted = order(request, extension, &input, "de");
        (sorted, request.crossings() - before)
    });
    assert_eq!(crossings, 1);
    let mut seen = sorted.clone();
    seen.sort_unstable();
    assert_eq!(
        seen,
        (0..input.len()).collect::<Vec<_>>(),
        "not a permutation"
    );
}

#[test]
fn sort_keys_order_as_the_collator_orders() {
    let input = names(2_000);
    let (sorted, keys) = run(|request, extension| {
        (
            order(request, extension, &input, "de"),
            keys(request, extension, &input, "de"),
        )
    });
    assert_eq!(keys.len(), input.len());
    // Both sorts are stable, so equal strings are in input order on both sides.
    let mut by_key: Vec<usize> = (0..input.len()).collect();
    by_key.sort_by(|&a, &b| keys[a].cmp(&keys[b]));
    assert_eq!(by_key, sorted);
}

#[test]
fn a_swedish_sort_puts_a_umlaut_after_z_and_a_german_sort_does_not() {
    let input: Vec<String> = ["ä", "z", "a"].iter().map(|s| (*s).to_owned()).collect();
    let (swedish, german) = run(|request, extension| {
        (
            order(request, extension, &input, "sv"),
            order(request, extension, &input, "de"),
        )
    });
    let words = |order: Vec<usize>| -> Vec<&str> { order.iter().map(|&i| &*input[i]).collect() };
    assert_eq!(words(swedish), ["a", "z", "ä"]);
    assert_eq!(words(german), ["a", "ä", "z"]);
}
