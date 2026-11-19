//! Which `Core` members own their subject, and what an owning entry does with
//! the reference it is passed (`rule:types/a-replaced-value-is-changed-in-place`).
//!
//! A member owns its first parameter when that parameter is an `array<…>`, a
//! `string` or a `bytes`, and its result is the same kind, nullable or not.
//! The signature decides it, with no list and no opt-out, so the first test
//! reads every registry row and the helper table and names each member whose
//! entry disagrees. `nvs_runtime`'s `owned` module states the protocol the
//! second test holds: an owning entry that throws leaves the subject with its
//! caller, and one that succeeds consumes it. The next two hold
//! `Core\Arr::reduce`, which gives its carry to the callback. The next three
//! hold every owning `Core\Arr` member to its row in [`ARR_ROWS`]: a member
//! that keeps its storage returns a sole subject's own array, and a shared
//! subject copies only the entries the result keeps. The last two hold every
//! owning `Core\Str` and `Core\Bytes` member to its row in [`TEXT_ROWS`] the
//! same way: on a sole subject, the result is the subject's own allocation.

use std::cell::RefCell;

use nvs_runtime::{Ctx, NvsArray, NvsStr, THROWN, Value, call};
use nvs_stdlib::registry::{CLASSES, CoreMethod, CoreTy};

/// The three kinds a member can own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Array,
    Str,
    Bytes,
}

/// The kind a written type has, through a nullable, an alias and a parameter
/// mark, or `None` for a type that is not one of the three.
fn kind(ty: &CoreTy) -> Option<Kind> {
    match ty {
        CoreTy::Nullable(inner)
        | CoreTy::Alias(_, inner)
        | CoreTy::Keys(_, inner)
        | CoreTy::Dispatched(inner)
        | CoreTy::Bounded(_, inner) => kind(inner),
        CoreTy::Array(_) => Some(Kind::Array),
        CoreTy::Str
        | CoreTy::Text(_)
        | CoreTy::Path(_)
        | CoreTy::Program(_)
        | CoreTy::ClassName(_)
        | CoreTy::StringHoles(..)
        | CoreTy::SecretStr
        | CoreTy::SecretText(_)
        | CoreTy::TaintedStr
        | CoreTy::SecretTaintedStr => Some(Kind::Str),
        CoreTy::Bytes
        | CoreTy::Blob(_)
        | CoreTy::SecretBytes
        | CoreTy::SecretBlob(_)
        | CoreTy::TaintedBytes
        | CoreTy::SecretTaintedBytes => Some(Kind::Bytes),
        _ => None,
    }
}

/// Whether the signature rule makes `method` own its first parameter. The
/// result may be nullable and the parameter may not, because an owning entry
/// takes a subject of its declared kind and a `null` is none of the three.
fn signature_owns(method: &CoreMethod) -> bool {
    method
        .params
        .first()
        .filter(|param| !matches!(param, CoreTy::Nullable(_)))
        .and_then(kind)
        .is_some_and(|subject| kind(&method.return_ty) == Some(subject))
}

#[test]
fn every_member_owns_exactly_what_its_signature_says() {
    let mut wrong = Vec::new();
    for class in CLASSES {
        // An instance member's receiver is argument slot 0, so its first
        // parameter is slot 1 or later. A member the lowering hands a prepared
        // argument first, such as `Core\IO::read`'s own-file word, has its
        // first parameter one slot later again.
        for (members, slot) in [(class.methods, 0), (class.instance, 1)] {
            for method in members {
                let owns = signature_owns(method);
                match (owns, nvs_stdlib::owning_entry(method.symbol)) {
                    (false, None) => {}
                    (true, Some(entry)) if entry.slot >= slot => {}
                    (true, Some(entry)) => wrong.push(format!(
                        "{}::{} takes its subject from slot {}, before its first parameter's slot {slot}",
                        class.name, method.name, entry.slot
                    )),
                    (true, None) => wrong.push(format!(
                        "{}::{} owns its subject by its signature, and `{}` has no owning entry",
                        class.name, method.name, method.symbol
                    )),
                    (false, Some(_)) => wrong.push(format!(
                        "{}::{} has an owning entry, and its signature does not own its subject",
                        class.name, method.name
                    )),
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} member(s) disagree with the signature rule:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// The owning entry of `symbol`, as the JIT is handed it.
fn owned_entry(symbol: &str) -> nvs_runtime::NvsFn {
    let owned = nvs_stdlib::owning_entry(symbol)
        .unwrap_or_else(|| panic!("`{symbol}` has an owning entry"));
    assert!(
        nvs_stdlib::symbols()
            .iter()
            .any(|(name, address)| *name == owned.symbol && *address == owned.address),
        "the JIT's symbol table lists `{}`",
        owned.symbol
    );
    #[expect(
        unsafe_code,
        reason = "an owning entry is a helper compiled with exactly this signature"
    )]
    unsafe {
        std::mem::transmute::<*const u8, nvs_runtime::NvsFn>(owned.address)
    }
}

#[test]
#[expect(
    clippy::disallowed_methods,
    reason = "the test watches one string handle's count, and its bytes are an ASCII literal"
)]
fn an_owning_helper_leaves_a_refused_subject_with_its_caller() {
    let repeat = owned_entry("nvs_core_str_repeat");
    let mut ctx = Ctx::buffered();
    let kept = NvsStr::new(b"ab");

    // A count no string can hold throws in `check`. The reference passed in
    // is still the caller's, so the count is unchanged and the text is intact.
    let passed = Value::str(kept.clone());
    let refused = call(repeat, &mut ctx, &[passed, Value::uint(u64::MAX)]);
    assert_eq!(refused.err(), Some(THROWN));
    drop(ctx.take_pending());
    assert_eq!(
        kept.refcount(),
        2,
        "a refused call leaves the subject with its caller"
    );
    assert_eq!(passed.as_text(), Some("ab"));
    #[expect(
        unsafe_code,
        reason = "the refused call left this reference with the caller, which releases it once"
    )]
    unsafe {
        passed.release();
    }
    assert_eq!(kept.refcount(), 1);

    // A call that succeeds consumes the reference it was passed.
    let passed = Value::str(kept.clone());
    let result = call(repeat, &mut ctx, &[passed, Value::uint(2)]);
    let result = result.unwrap_or_else(|status| panic!("repeat returned status {status}"));
    assert_eq!(result.as_text(), Some("abab"));
    assert_eq!(
        kept.refcount(),
        1,
        "a call that succeeds consumes the subject it was passed"
    );
    #[expect(
        unsafe_code,
        reason = "the result carries one fresh reference the caller owns"
    )]
    unsafe {
        result.release();
    }
}

/// The helper the JIT calls for `symbol`.
fn helper(symbol: &str) -> nvs_runtime::NvsFn {
    let (_, address) = nvs_stdlib::symbols()
        .into_iter()
        .find(|(name, _)| *name == symbol)
        .unwrap_or_else(|| panic!("the JIT's symbol table lists `{symbol}`"));
    #[expect(
        unsafe_code,
        reason = "every address in the symbol table is a helper compiled with this signature"
    )]
    unsafe {
        std::mem::transmute::<*const u8, nvs_runtime::NvsFn>(address)
    }
}

/// A callable that declares two parameters and runs `invoke`.
fn callable_of_two(invoke: nvs_runtime::NvsFn) -> Value {
    callable(invoke, 2)
}

/// A callable that declares `arity` parameters of any type and runs `invoke`.
fn callable(invoke: nvs_runtime::NvsFn, arity: i64) -> Value {
    let mut table = nvs_runtime::ClassTable::new();
    let id = table.define("{callable}", &["arity", "params"], &[]);
    table.set_methods(
        id,
        vec![nvs_runtime::MethodRow {
            name: nvs_runtime::CALLABLE_INVOKE.to_owned(),
            code: invoke as *const u8,
            arity: 0,
            param_tags: 0,
            param_names: Vec::new(),
            param_types: Vec::new(),
            public: true,
            protected: false,
            native: false,
            param_classes: Vec::new(),
            param_checks: Vec::new(),
            is_static: false,
            by_reference: false,
        }],
    );
    table.set_callable(id);
    let table: &'static nvs_runtime::ClassTable = Box::leak(Box::new(table));
    #[expect(
        unsafe_code,
        reason = "the table is leaked, so the descriptor outlives the object"
    )]
    let object = unsafe { nvs_runtime::NvsObj::new(table.desc(id)) };
    object.set_field(nvs_runtime::CALLABLE_ARITY_SLOT, Value::int(arity));
    let any = u64::from(nvs_runtime::CALLABLE_PARAM_TAG_ANY);
    object.set_field(
        nvs_runtime::CALLABLE_PARAM_TAGS_SLOT,
        Value::int(i64::from_ne_bytes((any | any << 4).to_ne_bytes())),
    );
    Value::object(object)
}

thread_local! {
    /// The count of each carry the callback below was handed, in call order.
    static SEEN: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}

/// A fold that records its carry's count. A carry it shares with another
/// holder is copied, as a change copies it; a carry it alone holds is
/// returned as it is, as a change in place returns it. The second call
/// throws when `THROW_SECOND` is set, after it frees its arguments, as a
/// compiled callee's error exit frees them.
#[expect(
    unsafe_code,
    reason = "`call_callable` passes the receiver, the carry and the value, each \
              this callee's to release, and the address of a live result"
)]
unsafe extern "C" fn records_its_carry(ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
    let (receiver, carry, value) = unsafe { (*args, *args.add(1), *args.add(2)) };
    let count = carry.refcount().expect("the carry is an array");
    let calls = SEEN.with(|seen| {
        seen.borrow_mut().push(count);
        seen.borrow().len()
    });
    unsafe {
        receiver.release();
        value.release();
    }
    if calls == 2 && THROW_SECOND.with(std::cell::Cell::get) {
        unsafe {
            carry.release();
            (*ctx).set_pending("the second entry is refused");
        }
        return THROWN;
    }
    let next = if count == 1 {
        carry
    } else {
        unsafe {
            carry.release();
        }
        Value::array(NvsArray::new())
    };
    unsafe {
        *out = next;
    }
    nvs_runtime::OK
}

thread_local! {
    /// Whether [`records_its_carry`] throws on its second call.
    static THROW_SECOND: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Three entries to fold over, and a seed its caller keeps.
fn three_entries_and_a_seed() -> (Value, Value) {
    let mut subject = NvsArray::new();
    for n in 1..=3 {
        subject.append(Value::int(n));
    }
    let mut seed = NvsArray::new();
    seed.append(Value::int(0));
    (Value::array(subject), Value::array(seed))
}

/// Frees one reference the test holds.
fn dropped(value: Value) {
    #[expect(unsafe_code, reason = "the test owns this reference")]
    unsafe {
        value.release();
    }
}

#[test]
fn a_reduce_callback_changes_its_carry_in_place() {
    SEEN.with(|seen| seen.borrow_mut().clear());
    THROW_SECOND.with(|flag| flag.set(false));
    let reduce = helper("nvs_core_arr_reduce");
    let mut ctx = Ctx::buffered();
    let (subject, seed) = three_entries_and_a_seed();
    let folder = callable_of_two(records_its_carry);
    let answer = call(reduce, &mut ctx, &[subject, folder, seed])
        .unwrap_or_else(|status| panic!("reduce returned status {status}"));
    // The caller still holds the seed, so the first call shares it and copies
    // once. Every later call holds the only reference to its carry.
    assert_eq!(SEEN.with(|seen| seen.borrow().clone()), vec![2, 1, 1]);
    assert_eq!(
        seed.refcount(),
        Some(1),
        "the seed is the caller's alone again"
    );
    assert_eq!(answer.refcount(), Some(1));
    for value in [answer, seed, subject, folder] {
        dropped(value);
    }
}

#[test]
fn a_reduce_callback_that_throws_frees_the_carry_once() {
    SEEN.with(|seen| seen.borrow_mut().clear());
    THROW_SECOND.with(|flag| flag.set(true));
    let reduce = helper("nvs_core_arr_reduce");
    let mut ctx = Ctx::buffered();
    let (subject, seed) = three_entries_and_a_seed();
    let folder = callable_of_two(records_its_carry);
    let refused = call(reduce, &mut ctx, &[subject, folder, seed]);
    assert_eq!(refused.err(), Some(THROWN));
    drop(ctx.take_pending());
    // The second call's carry is the first call's fresh copy, which the
    // callback alone held and freed as it threw. The seed is untouched.
    assert_eq!(SEEN.with(|seen| seen.borrow().clone()), vec![2, 1]);
    assert_eq!(
        seed.refcount(),
        Some(1),
        "the seed is the caller's alone again"
    );
    for value in [seed, subject, folder] {
        dropped(value);
    }
}

/// What an owning `Core\Arr` member does with a sole subject.
enum Row {
    /// It changes the subject's own array and returns it. `args` gives the
    /// arguments after the subject, and `count` is the result's entry count
    /// for the subject `[1, 2, 3, 4]`.
    Keeps {
        args: fn() -> Vec<Value>,
        count: usize,
    },
    /// It builds a new array, because the result keeps no entry of the
    /// subject in its place.
    Builds,
}

/// Every owning `Core\Arr` member, by its name in the registry.
const ARR_ROWS: &[(&str, Row)] = &[
    (
        "slice",
        Row::Keeps {
            args: || vec![Value::int(1), Value::null(), Value::bool(false)],
            count: 3,
        },
    ),
    (
        "replaceRange",
        Row::Keeps {
            args: || vec![Value::int(1), Value::int(1), list(&[9])],
            count: 4,
        },
    ),
    (
        "withoutFirst",
        Row::Keeps {
            args: Vec::new,
            count: 3,
        },
    ),
    (
        "withoutLast",
        Row::Keeps {
            args: Vec::new,
            count: 3,
        },
    ),
    (
        "withoutNull",
        Row::Keeps {
            args: Vec::new,
            count: 4,
        },
    ),
    (
        "filter",
        Row::Keeps {
            args: || vec![callable(keeps_even, 1)],
            count: 2,
        },
    ),
    (
        "unique",
        Row::Keeps {
            args: || vec![Value::null()],
            count: 4,
        },
    ),
    (
        "diff",
        Row::Keeps {
            args: set_args,
            count: 3,
        },
    ),
    (
        "intersect",
        Row::Keeps {
            args: set_args,
            count: 1,
        },
    ),
    (
        "append",
        Row::Keeps {
            args: || vec![list(&[9])],
            count: 5,
        },
    ),
    (
        "prepend",
        Row::Keeps {
            args: || vec![list(&[9])],
            count: 5,
        },
    ),
    (
        "appendAll",
        Row::Keeps {
            args: layer_of_nine,
            count: 5,
        },
    ),
    (
        "padStart",
        Row::Keeps {
            args: || vec![Value::uint(6), Value::int(0)],
            count: 6,
        },
    ),
    (
        "padEnd",
        Row::Keeps {
            args: || vec![Value::uint(6), Value::int(0)],
            count: 6,
        },
    ),
    (
        "overlay",
        Row::Keeps {
            args: layer_of_nine,
            count: 4,
        },
    ),
    (
        "overlayDeep",
        Row::Keeps {
            args: layer_of_nine,
            count: 4,
        },
    ),
    (
        "underlay",
        Row::Keeps {
            args: layer_of_nine,
            count: 4,
        },
    ),
    (
        "reverse",
        Row::Keeps {
            args: || vec![Value::bool(false)],
            count: 4,
        },
    ),
    (
        "sort",
        Row::Keeps {
            args: || {
                vec![
                    Value::null(),
                    Value::int(1),
                    Value::null(),
                    Value::bool(false),
                ]
            },
            count: 4,
        },
    ),
    (
        "sortByKey",
        Row::Keeps {
            args: || vec![Value::int(1), Value::null(), Value::null()],
            count: 4,
        },
    ),
    (
        "values",
        Row::Keeps {
            args: Vec::new,
            count: 4,
        },
    ),
    (
        "map",
        Row::Keeps {
            args: || vec![callable(doubles, 1)],
            count: 4,
        },
    ),
    (
        "mapKeys",
        Row::Keeps {
            args: || vec![callable(shifts_the_key, 1)],
            count: 4,
        },
    ),
    (
        "flip",
        Row::Keeps {
            args: Vec::new,
            count: 4,
        },
    ),
    ("chunk", Row::Builds),
    ("column", Row::Builds),
    ("groupBy", Row::Builds),
    ("countBy", Row::Builds),
    ("keys", Row::Builds),
    ("flatten", Row::Builds),
    ("flattenDeep", Row::Builds),
    ("fillKeys", Row::Builds),
    ("fromKeysAndValues", Row::Builds),
];

/// A list of the given integers.
fn list(values: &[i64]) -> Value {
    let mut array = NvsArray::new();
    for value in values {
        array.append(Value::int(*value));
    }
    Value::array(array)
}

/// `diff` and `intersect` against `[2]`, on values, with no callback.
fn set_args() -> Vec<Value> {
    vec![list(&[2]), Value::int(0), Value::null(), Value::null()]
}

/// A variadic tail of one layer, `[9]`.
fn layer_of_nine() -> Vec<Value> {
    let mut tail = NvsArray::new();
    tail.append(list(&[9]));
    vec![Value::array(tail)]
}

/// The argument a one-parameter callback is passed, after it frees the
/// receiver.
#[expect(
    unsafe_code,
    reason = "`call_callable` passes the receiver and the value, each this callee's to release"
)]
unsafe fn the_value(args: *const Value) -> i64 {
    let (receiver, value) = unsafe { (*args, *args.add(1)) };
    let number = value.as_int().expect("every entry here is an int");
    unsafe {
        receiver.release();
        value.release();
    }
    number
}

/// A `filter` predicate that keeps the even values.
#[expect(unsafe_code, reason = "`out` is the address of a live result")]
unsafe extern "C" fn keeps_even(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
    unsafe {
        *out = Value::bool(the_value(args) % 2 == 0);
    }
    nvs_runtime::OK
}

/// A `map` callback that doubles each value.
#[expect(unsafe_code, reason = "`out` is the address of a live result")]
unsafe extern "C" fn doubles(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
    unsafe {
        *out = Value::int(the_value(args) * 2);
    }
    nvs_runtime::OK
}

/// A `mapKeys` callback that gives each entry the key `100 + value`.
#[expect(unsafe_code, reason = "`out` is the address of a live result")]
unsafe extern "C" fn shifts_the_key(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
    unsafe {
        *out = Value::int(100 + the_value(args));
    }
    nvs_runtime::OK
}

/// The registry symbol of each owning `Core\Arr` member, by name.
fn owning_arr_members() -> Vec<(&'static str, &'static str)> {
    let class = CLASSES
        .iter()
        .find(|class| class.name == r"Core\Arr")
        .expect("the registry has Core\\Arr");
    class
        .methods
        .iter()
        .filter(|method| nvs_stdlib::owning_entry(method.symbol).is_some())
        .map(|method| (method.name, method.symbol))
        .collect()
}

#[test]
fn every_arr_member_has_a_row_here() {
    let owning = owning_arr_members();
    let mut wrong = Vec::new();
    for (name, _) in &owning {
        if !ARR_ROWS.iter().any(|(row, _)| row == name) {
            wrong.push(format!(
                "Core\\Arr::{name} owns its subject and has no row in ARR_ROWS"
            ));
        }
    }
    for (row, _) in ARR_ROWS {
        if !owning.iter().any(|(name, _)| name == row) {
            wrong.push(format!(
                "ARR_ROWS names `{row}`, which is no owning Core\\Arr member"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_arr_member_keeps_what_it_does_not_change() {
    let mut wrong = Vec::new();
    for (name, symbol) in owning_arr_members() {
        let Some((_, Row::Keeps { args, count })) = ARR_ROWS.iter().find(|(row, _)| *row == name)
        else {
            continue;
        };
        let entry = owned_entry(symbol);
        let mut ctx = Ctx::buffered();
        let subject = list(&[1, 2, 3, 4]);
        let storage = subject.array_ptr();
        let rest = args();
        let mut passed = vec![subject];
        passed.extend(rest.iter().copied());
        let result = call(entry, &mut ctx, &passed)
            .unwrap_or_else(|status| panic!("Core\\Arr::{name} returned status {status}"));
        if result.array_ptr() != storage {
            wrong.push(format!(
                "Core\\Arr::{name} built a new array from a sole subject"
            ));
        }
        let got = result.as_array().map_or(0, |array| array.count());
        if got != *count {
            wrong.push(format!("Core\\Arr::{name} gave {got} entries, not {count}"));
        }
        for value in rest.into_iter().chain([result]) {
            dropped(value);
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_shared_subject_builds_only_the_window_it_keeps() {
    let slice = owned_entry("nvs_core_arr_slice");
    let mut ctx = Ctx::buffered();
    // A thousand rows, each an array of its own, so each row's count shows
    // whether the slice copied it.
    let mut big = NvsArray::new();
    for n in 0..1000 {
        big.append(list(&[n]));
    }
    let subject = Value::array(big);
    let kept = NvsArray::clone(&subject.as_array().expect("an array"));
    let result = call(
        slice,
        &mut ctx,
        &[subject, Value::int(500), Value::int(10), Value::bool(false)],
    )
    .unwrap_or_else(|status| panic!("slice returned status {status}"));
    assert_ne!(
        result.array_ptr(),
        subject.array_ptr(),
        "a shared subject is not changed"
    );
    let window = result.as_array().expect("slice returns an array");
    assert_eq!(window.count(), 10);
    let mut copied = 0;
    let mut from = 0;
    while let Some(at) = kept.next_slot(from) {
        let row = kept
            .value_at(at)
            .expect("next_slot only names live entries");
        if row.refcount() != Some(1) {
            copied += 1;
        }
        from = at + 1;
    }
    assert_eq!(copied, 10, "the slice holds only the ten rows it keeps");
    dropped(result);
    drop(kept);
}

/// What an owning `Core\Str` or `Core\Bytes` member does with a sole subject.
enum TextRow {
    /// It changes the subject's own allocation and returns it. The subject is
    /// `subject` with `room` spare bytes behind it, `args` gives the arguments
    /// after it, and `result` is what the member returns.
    Keeps {
        subject: &'static str,
        room: usize,
        args: fn() -> Vec<Value>,
        result: &'static str,
    },
    /// It builds a new string, because the result keeps no byte of the subject
    /// in its place.
    Builds,
}

/// Every owning `Core\Str` and `Core\Bytes` member, by its class and its name
/// in the registry.
const TEXT_ROWS: &[(&str, &str, TextRow)] = &[
    (
        r"Core\Str",
        "slice",
        TextRow::Keeps {
            subject: "abcdef",
            room: 0,
            args: || vec![Value::int(1), Value::null()],
            result: "bcdef",
        },
    ),
    (
        r"Core\Str",
        "at",
        TextRow::Keeps {
            subject: "abcdef",
            room: 0,
            args: || vec![Value::int(2)],
            result: "c",
        },
    ),
    (
        r"Core\Str",
        "before",
        TextRow::Keeps {
            subject: "ab,cd",
            room: 0,
            args: || vec![text(","), Value::bool(false)],
            result: "ab",
        },
    ),
    (
        r"Core\Str",
        "after",
        TextRow::Keeps {
            subject: "ab,cd",
            room: 0,
            args: || vec![text(","), Value::bool(false)],
            result: "cd",
        },
    ),
    (
        r"Core\Str",
        "trim",
        TextRow::Keeps {
            subject: "  ab  ",
            room: 0,
            args: || vec![text(" ")],
            result: "ab",
        },
    ),
    (
        r"Core\Str",
        "trimStart",
        TextRow::Keeps {
            subject: "  ab  ",
            room: 0,
            args: || vec![text(" ")],
            result: "ab  ",
        },
    ),
    (
        r"Core\Str",
        "trimEnd",
        TextRow::Keeps {
            subject: "  ab  ",
            room: 0,
            args: || vec![text(" ")],
            result: "  ab",
        },
    ),
    (
        r"Core\Bytes",
        "slice",
        TextRow::Keeps {
            subject: "abcdef",
            room: 0,
            args: || vec![Value::int(1), Value::null()],
            result: "bcdef",
        },
    ),
    (
        r"Core\Str",
        "padEnd",
        TextRow::Keeps {
            subject: "ab",
            room: 2,
            args: || vec![Value::uint(4), text("-")],
            result: "ab--",
        },
    ),
    (
        r"Core\Str",
        "padStart",
        TextRow::Keeps {
            subject: "ab",
            room: 2,
            args: || vec![Value::uint(4), text("-")],
            result: "--ab",
        },
    ),
    (
        r"Core\Str",
        "repeat",
        TextRow::Keeps {
            subject: "ab",
            room: 2,
            args: || vec![Value::uint(2)],
            result: "abab",
        },
    ),
    (
        r"Core\Bytes",
        "repeat",
        TextRow::Keeps {
            subject: "ab",
            room: 2,
            args: || vec![Value::uint(2)],
            result: "abab",
        },
    ),
    (
        r"Core\Str",
        "replaceRange",
        TextRow::Keeps {
            subject: "abcdef",
            room: 0,
            args: || vec![Value::int(1), Value::int(2), text("X")],
            result: "aXdef",
        },
    ),
    (
        r"Core\Str",
        "replace",
        TextRow::Keeps {
            subject: "a-b-c",
            room: 0,
            args: || {
                vec![
                    text("-"),
                    text("+"),
                    Value::bool(false),
                    Value::uint(u64::MAX),
                ]
            },
            result: "a+b+c",
        },
    ),
    (
        r"Core\Str",
        "replaceAll",
        TextRow::Keeps {
            subject: "a-b-c",
            room: 0,
            args: || {
                let mut table = NvsArray::new();
                table.set(NvsStr::new(b"-"), text("+"));
                vec![Value::array(table), Value::bool(false)]
            },
            result: "a+b+c",
        },
    ),
    (
        r"Core\Str",
        "lower",
        TextRow::Keeps {
            subject: "AbC",
            room: 0,
            args: Vec::new,
            result: "abc",
        },
    ),
    (
        r"Core\Str",
        "upper",
        TextRow::Keeps {
            subject: "AbC",
            room: 0,
            args: Vec::new,
            result: "ABC",
        },
    ),
    (
        r"Core\Str",
        "upperFirst",
        TextRow::Keeps {
            subject: "abc",
            room: 0,
            args: Vec::new,
            result: "Abc",
        },
    ),
    (
        r"Core\Str",
        "lowerFirst",
        TextRow::Keeps {
            subject: "ABC",
            room: 0,
            args: Vec::new,
            result: "aBC",
        },
    ),
    (
        r"Core\Str",
        "fold",
        TextRow::Keeps {
            subject: "AbC",
            room: 0,
            args: Vec::new,
            result: "abc",
        },
    ),
    (
        r"Core\Str",
        "normalize",
        TextRow::Keeps {
            subject: "e\u{301}",
            room: 0,
            args: || vec![Value::int(0)],
            result: "\u{e9}",
        },
    ),
    (
        r"Core\Str",
        "reverse",
        TextRow::Keeps {
            subject: "abc",
            room: 0,
            args: Vec::new,
            result: "cba",
        },
    ),
    (
        r"Core\Str",
        "wrap",
        TextRow::Keeps {
            subject: "ab cd",
            room: 0,
            args: || vec![Value::uint(2), text("\n"), Value::bool(false)],
            result: "ab\ncd",
        },
    ),
    (r"Core\Str", "format", TextRow::Builds),
];

/// A `string` value of `content`, with no spare room.
fn text(content: &str) -> Value {
    Value::text(content)
}

/// A sole subject of `content` with `room` spare bytes behind it, tagged as
/// `class` says.
#[expect(
    clippy::disallowed_methods,
    reason = "every subject in TEXT_ROWS is a UTF-8 literal"
)]
fn text_subject(class: &str, content: &str, room: usize) -> Value {
    let built = NvsStr::build(content.len() + room, |out| out.push(content.as_bytes()));
    if class == r"Core\Bytes" {
        Value::bytes(built)
    } else {
        Value::str(built)
    }
}

/// The registry symbol of each owning `Core\Str` and `Core\Bytes` member, with
/// its class and name.
fn owning_text_members() -> Vec<(&'static str, &'static str, &'static str)> {
    CLASSES
        .iter()
        .filter(|class| class.name == r"Core\Str" || class.name == r"Core\Bytes")
        .flat_map(|class| {
            class
                .methods
                .iter()
                .filter(|method| nvs_stdlib::owning_entry(method.symbol).is_some())
                .map(|method| (class.name, method.name, method.symbol))
        })
        .collect()
}

#[test]
fn every_string_member_has_a_row_here() {
    let owning = owning_text_members();
    let mut wrong = Vec::new();
    for (class, name, _) in &owning {
        if !TEXT_ROWS.iter().any(|(c, n, _)| c == class && n == name) {
            wrong.push(format!(
                "{class}::{name} owns its subject and has no row in TEXT_ROWS"
            ));
        }
    }
    for (class, name, _) in TEXT_ROWS {
        if !owning.iter().any(|(c, n, _)| c == class && n == name) {
            wrong.push(format!(
                "TEXT_ROWS names `{class}::{name}`, which is no owning member"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_string_member_keeps_what_it_does_not_change() {
    let mut wrong = Vec::new();
    for (class, name, symbol) in owning_text_members() {
        let Some((
            _,
            _,
            TextRow::Keeps {
                subject,
                room,
                args,
                result: want,
            },
        )) = TEXT_ROWS.iter().find(|(c, n, _)| *c == class && *n == name)
        else {
            continue;
        };
        let entry = owned_entry(symbol);
        let mut ctx = Ctx::buffered();
        let passed = text_subject(class, subject, *room);
        let storage = passed.buffer_ptr();
        let rest = args();
        let mut all = vec![passed];
        all.extend(rest.iter().copied());
        let result = call(entry, &mut ctx, &all)
            .unwrap_or_else(|status| panic!("{class}::{name} returned status {status}"));
        if result.buffer_ptr() != storage {
            wrong.push(format!(
                "{class}::{name} built a new string from a sole subject"
            ));
        }
        let got = result
            .as_text()
            .map(str::as_bytes)
            .or_else(|| result.as_bytes())
            .map(<[u8]>::to_vec)
            .unwrap_or_default();
        if got != want.as_bytes() {
            wrong.push(format!(
                "{class}::{name} gave {:?}, not {want:?}",
                String::from_utf8_lossy(&got)
            ));
        }
        for value in rest.into_iter().chain([result]) {
            dropped(value);
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
