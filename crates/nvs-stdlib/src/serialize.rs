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

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

/// `rule:classes/graph-copy`'s `encode`/`decode` pair, taking
/// `rule:core-api/shape-rules` R6's naming.
/// PHP's bare `serialize`/`unserialize` spellings do not exist.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Serialize",
    doc: None,
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
    short: "Copies the whole value graph under `$value` into Novis's own closed byte format, as \
            `serialize` does — the same graph copy the `spawn` boundary runs, externalized so it \
            can be stored or sent.",
    params: &[ParamDoc {
        name: "value",
        desc: "The value to encode: scalars, arrays and class instances, however deeply \
               nested.",
        shape: &[],
    }],
    ret: "The payload; tainted whenever `$value` was, since encoding launders nothing.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$value` holds something that has no meaning on the other side of a copy \
               boundary — a closure, a resource, or an instance with a `secret` property that \
               was not revealed — or the graph nests deeper than the copy's limit.",
    }],
};

/// `Core\Serialize::decode`'s reference card — `rule:core-api/reference-card`.
const DECODE_DOC: MethodDoc = MethodDoc {
    short: "Rebuilds the value `encode` wrote into `$payload`, as `unserialize` does, over \
            Novis's own format and no other. The parameter is a `tainted` sink, so bytes that \
            arrived from outside the process are refused at compile time.",
    params: &[ParamDoc {
        name: "payload",
        desc: "The bytes `encode` answered.",
        shape: &[],
    }],
    ret: "The decoded value, every instance in it an object of the class this program \
          declares under the name the payload records.",
    errors: &[ErrorDoc {
        error: "ParseError",
        desc: "`$payload` does not carry Novis's serialization marker, is another format \
               version, ends in the middle of a value or carries bytes past its end, or names \
               a class this program does not declare or whose declared properties are not \
               the ones the payload records.",
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
