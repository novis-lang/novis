//! Which `Core` members own their subject, and what an owning entry does with
//! the reference it is passed (`rule:types/a-replaced-value-is-changed-in-place`).
//!
//! A member owns its first parameter when that parameter is an `array<…>`, a
//! `string` or a `bytes`, and its result is the same kind, nullable or not.
//! The signature decides it, with no list and no opt-out, so the first test
//! reads every registry row and the helper table and names each member whose
//! entry disagrees. `nvs_runtime`'s `owned` module states the protocol the
//! second test holds: an owning entry that throws leaves the subject with its
//! caller, and one that succeeds consumes it. The last two hold
//! `Core\Arr::reduce`, which gives its carry to the callback.

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
    object.set_field(nvs_runtime::CALLABLE_ARITY_SLOT, Value::int(2));
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
