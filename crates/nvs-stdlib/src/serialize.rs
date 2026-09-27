//! `Core\Serialize` — `rule:classes/graph-copy`'s externalizing carrier, and nothing else.
//!
//! There is no walk in this file. The graph copy is
//! [`nvs_runtime::graph`]'s, written once and reached by both of § 2's
//! carriers; what lives here is the pair of members that reach it, their
//! qualifiers, and the two questions the signature answers that the walk does
//! not.
//!
//! # `decode` is a `tainted` sink, and `encode` is not a launderer
//!
//! `rule:classes/serialize-is-a-closed-format`'s last bullet: the closed format and the no-hook rule shut the
//! code-execution class, but not type confusion — a payload that reconstructs a
//! `User` with `isAdmin: true` satisfies every check the format makes. So the
//! parameter is [`Qual::Sink`] and bytes that arrived from outside the process
//! are refused *at compile time*, with no launderer anywhere. `encode`'s own
//! result is [`Qual::Contagious`] by omission: bytes made from a tainted graph
//! are still tainted, which is the ordinary rule and not a special case.
//!
//! # Why the failure is a `ParseError` on the way in and a `LogicError` on the
//! way out
//!
//! Every refusal [`nvs_runtime::GraphError`] carries is one message, and the
//! two members classify it differently on purpose. A payload that is not this
//! format, names an unresolvable class, or records a property set the class no
//! longer has is *input that did not match a format this code declared*, which
//! is what spec § 10's `ParseError` is. A value that cannot be encoded — a
//! closure, a `secret` property — is a value the program itself built, so it is
//! a bug in the program and a `LogicError`, exactly as `Core\Json::encode`
//! already answers.
//!
//! # There is no `Core\Serialize::clone`
//!
//! § 2's live carrier is reached at the `spawn` boundary rather than by a
//! member, and PHP's `clone` is the language's own operator. This class is the
//! externalizing half only.

use nvs_runtime::{Fault, NvsStr, ThrownClass, Value};

use crate::registry::{
    ClassDoc, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// `Core\Serialize`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Turns a value into `bytes` and back. `encode()` writes the bytes, and `decode()` \
            builds the same value again from them. Use it to save a value in a file or a cache \
            and read it later.",
};

/// `rule:classes/graph-copy`'s `encode`/`decode` pair, taking
/// `rule:core-api/shape-rules` R6's naming.
/// PHP's bare `serialize`/`unserialize` spellings do not exist.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Serialize",
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "encode",
            names: &["value"],
            params: &[CoreTy::Mixed],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_serialize_encode",
            doc: Some(&ENCODE_DOC),
        },
        CoreMethod {
            name: "decode",
            names: &["payload"],
            params: &[CoreTy::Blob(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Mixed,
            symbol: "nvs_core_serialize_decode",
            doc: Some(&DECODE_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Serialize::encode`'s reference card — `rule:core-api/reference-card`.
const ENCODE_DOC: MethodDoc = MethodDoc {
    short: "Turns a value into `bytes` that `decode()` can read back. The value can be a \
            number, a string, an array or an object, and it can contain other arrays and \
            objects.",
    params: &[ParamDoc {
        name: "value",
        desc: "The value to turn into bytes. Two places that point to the same object still \
               point to one object after `decode()`.",
        shape: &[],
    }],
    ret: "The bytes. If `$value` is `tainted`, the bytes are also `tainted`.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$value` contains a closure, an open file or connection, or an object with a \
               `secret` property. It is also thrown when arrays and objects are nested more \
               than 256 levels deep.",
    }],
};

/// `Core\Serialize::decode`'s reference card — `rule:core-api/reference-card`.
const DECODE_DOC: MethodDoc = MethodDoc {
    short: "Builds the value again from the `bytes` that `encode()` returned. It reads only \
            this format. Bytes that came from outside the program, such as a request body, are \
            `tainted`, and a call with them does not compile.",
    params: &[ParamDoc {
        name: "payload",
        desc: "The bytes that `encode()` returned.",
        shape: &[],
    }],
    ret: "The value. An object in it is a new object of the class with the same name in this \
          program, and its constructor does not run.",
    errors: &[ErrorDoc {
        error: "ParseError",
        desc: "`$payload` was not written by `encode()`, is cut short, or has extra bytes at \
               the end. It is also thrown when it names a class this program does not have, or \
               a class whose properties are different now.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_serialize_encode" => (nvs_core_serialize_encode as *const ()).cast(),
        "nvs_core_serialize_decode" => (nvs_core_serialize_decode as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Serialize::encode(mixed $value): bytes` — `rule:classes/graph-copy`'s graph
    /// copy, externalized.
    fn nvs_core_serialize_encode(_ctx, args: [1]) {
        // The walk consumes one reference and the argument slot keeps its own,
        // so this is the reference the walk gives up.
        #[expect(
            unsafe_code,
            reason = "the argument slot holds a live reference for the length of \
                      the call, which is `Value::retain`'s whole obligation"
        )]
        unsafe {
            args[0].retain();
        }
        let written = nvs_runtime::encode(args[0]).map_err(|why| {
            Fault::thrown_as(
                ThrownClass::Logic,
                format!("Core\\Serialize::encode(): {why}"),
            )
        })?;
        Ok(Value::bytes(NvsStr::new(&written)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Serialize::decode(bytes $payload): mixed` — the same operation in
    /// reverse, over Novis's own closed format and no other.
    fn nvs_core_serialize_decode(ctx, args: [1]) {
        let payload = args[0].as_bytes().ok_or_else(|| {
            // Unreachable from source: the parameter is `CoreTy::Blob`, so a
            // non-`bytes` argument is `E0401` at the checker.
            Fault::fatal(format!(
                "Core\\Serialize::decode expected bytes, got tag {}",
                args[0].tag_byte()
            ))
        })?.to_vec();
        // The resolver is the program's own class table — `rule:classes/serialize-is-a-closed-format`'s "a
        // payload naming a class the receiving side cannot resolve is refused,
        // naming the class". `nvs_runtime::graph`'s known gap 2 owns what that
        // leaves out.
        let resolve = |name: &str| ctx.class_desc(name);
        nvs_runtime::decode(&payload, &resolve).map_err(|why| {
            Fault::thrown_as(
                ThrownClass::Parse,
                format!("Core\\Serialize::decode(): {why}"),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, NvsArray, NvsStr, Value};

    /// A graph with every scalar kind, a string key, a list and a nested
    /// array, so the payload has a value boundary of each kind to cut at.
    fn fixture() -> Value {
        let mut inner = NvsArray::new();
        inner.append(Value::int(-7));
        inner.append(Value::float(2.5));
        inner.append(Value::null());
        let mut outer = NvsArray::new();
        outer.set(
            NvsStr::new(b"name"),
            Value::str(NvsStr::new("Café 🔑".as_bytes())),
        );
        outer.set(NvsStr::new(b"paid"), Value::bool(true));
        outer.set(
            NvsStr::new(b"raw"),
            Value::bytes(NvsStr::new(&[0, 255, 128])),
        );
        outer.set(NvsStr::new(b"lines"), Value::array(inner));
        Value::array(outer)
    }

    /// Calls `encode` over `value` and answers the payload's octets. Releases
    /// the payload; the caller keeps `value`.
    fn encoded(ctx: &mut Ctx, value: Value) -> Vec<u8> {
        let payload = nvs_runtime::call(super::nvs_core_serialize_encode, ctx, &[value])
            .expect("the fixture holds nothing `encode` refuses");
        let octets = payload
            .as_bytes()
            .expect("`encode` returns `bytes`")
            .to_vec();
        #[expect(
            unsafe_code,
            reason = "the caller owns the one reference `encode` returned"
        )]
        unsafe {
            payload.release();
        }
        octets
    }

    /// Calls `decode` over `octets`, answering the value or the refusal's
    /// sentence. Releases the argument it built.
    fn decoded(ctx: &mut Ctx, octets: &[u8]) -> Result<Value, String> {
        let payload = Value::bytes(NvsStr::new(octets));
        let answer = nvs_runtime::call(super::nvs_core_serialize_decode, ctx, &[payload])
            .map_err(|_| ctx.take_pending().unwrap_or_default().into_owned());
        #[expect(
            unsafe_code,
            reason = "the test built the payload and releases it once"
        )]
        unsafe {
            payload.release();
        }
        answer
    }

    /// `encode` is a function of the value alone: encoding the same graph twice
    /// gives the same octets, and encoding what `decode` rebuilt from them gives
    /// them a third time, so nothing in the graph is lost on the way round.
    // covers: Core\Serialize::encode
    #[test]
    fn encode_is_stable_and_survives_a_round_trip_unchanged() {
        let mut ctx = Ctx::buffered();
        let value = fixture();
        let first = encoded(&mut ctx, value);
        assert_eq!(
            encoded(&mut ctx, value),
            first,
            "the same graph encodes the same octets"
        );
        let back = decoded(&mut ctx, &first).expect("`decode` reads what `encode` wrote");
        assert_eq!(
            encoded(&mut ctx, back),
            first,
            "the rebuilt graph encodes to the octets it was rebuilt from"
        );
        #[expect(
            unsafe_code,
            reason = "the test owns both graphs, and releases each once"
        )]
        unsafe {
            back.release();
            value.release();
        }
    }

    /// `decode` accepts a payload whole and nothing else: every shorter prefix
    /// of it, the empty one included, is refused, and so is the payload with
    /// one more octet, each with a sentence naming the member.
    // covers: Core\Serialize::decode
    #[test]
    fn decode_accepts_the_whole_payload_and_refuses_every_cut_and_one_octet_more() {
        let mut ctx = Ctx::buffered();
        let value = fixture();
        let whole = encoded(&mut ctx, value);
        #[expect(
            unsafe_code,
            reason = "the test built the fixture and releases it once"
        )]
        unsafe {
            value.release();
        }
        let back = decoded(&mut ctx, &whole).expect("the whole payload is accepted");
        #[expect(
            unsafe_code,
            reason = "the caller owns the one reference `decode` returned"
        )]
        unsafe {
            back.release();
        }
        let mut longer = whole.clone();
        longer.push(0);
        let refused = (0..whole.len())
            .map(|cut| &whole[..cut])
            .chain([longer.as_slice()])
            .filter(|octets| match decoded(&mut ctx, octets) {
                Ok(value) => {
                    #[expect(unsafe_code, reason = "an accepted cut is released once")]
                    unsafe {
                        value.release();
                    }
                    false
                }
                Err(why) => why.contains(r"Core\Serialize::decode()"),
            })
            .count();
        assert_eq!(
            refused,
            whole.len() + 1,
            "each of the {} cuts and the one longer payload is refused by name",
            whole.len()
        );
    }
}
