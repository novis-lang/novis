//! Seeding the checker's signature table with the loaded extension set.
//!
//! `rule:packaging/extension-calls-are-statically-typed`: each manifest's class becomes the same
//! [`ClassSignature`](crate::signatures::ClassSignature) a `Core` class becomes, its methods
//! `static` and public, and its parameter qualifiers in the [`MethodSig`] fields a `Core` row
//! fills. From there a call is checked by the machinery every other call goes through — arity,
//! each argument against its parameter, `crate::expr::quals`' admission — with no extension branch
//! anywhere (`rule:security/extension-manifest-only-tightens`).
//!
//! **A layer of its own, never part of the native base.** [`crate::core_lib::base`] is built once
//! per process from the binary's own registry. The extension set is the configuration's, so
//! [`seed`] runs in [`build_signatures`](crate::signatures::build_signatures) for every check,
//! after the natives and before any user declaration is collected. It spends one signature per
//! loaded class per check, and nothing for a program checked with no set.
//!
//! **What a manifest declares, read the way it only tightens:**
//!
//! - A parameter with `"sink": true` is [`Qual::Sink`], which refuses a `tainted` argument. Every
//!   other parameter is [`Qual::Contagious`] (`rule:security/extension-contagion`): a `tainted`
//!   argument is accepted, and the result is `tainted`.
//! - A method with `"source": true` returns its type made `tainted`, so its result is `tainted`
//!   whatever its arguments are.
//! - A type outside `rule:packaging/a-value-crosses-as-its-wit-type`'s table, or a constant whose
//!   value is not of its type, leaves the method or the constant out. The loader has already
//!   refused such a manifest; a reader that meets one anyway makes the member unknown, which only
//!   rejects calls. [`hir_classes`] reads the same filter, so name resolution and the signature
//!   table agree on which members exist.
//! - `array<K, V>` is interned as `array<V>`: a Novis `array` is already keyed, and `K` is only
//!   what the WIT list of pairs carries.

use nvs_ext::manifest::{Const, Manifest, Method};
use nvs_ext::types::NovisType;
use nvs_hir::{ExtensionClass, QName};
use nvs_stdlib::registry::{ParamText, Qual};
use rustc_hash::FxHashMap;

use crate::defaults::ConstArg;
use crate::signatures::{ConstSig, MethodSig, SignatureTable};
use crate::ty::{ShapeField, TypeId, TypeInterner};

/// The classes of `manifests` as name resolution reads them: each class's name, and the methods
/// and constants [`seed`] gives it a signature for.
#[must_use]
pub fn hir_classes(manifests: &[Manifest]) -> Vec<ExtensionClass> {
    manifests
        .iter()
        .map(|manifest| ExtensionClass {
            name: QName::parse(&manifest.class),
            methods: methods(manifest)
                .map(|(method, _)| method.name.clone())
                .collect(),
            consts: consts(manifest)
                .map(|(constant, ..)| constant.name.clone())
                .collect(),
        })
        .collect()
}

/// A table holding the classes of `manifests` and nothing else, for a reader that asks about the
/// set alone: what a method's signature is, with no program around it.
#[must_use]
pub fn signatures(manifests: &[Manifest], interner: &mut TypeInterner) -> SignatureTable {
    let mut table = SignatureTable::new();
    seed(&mut table, interner, manifests);
    table
}

/// Adds the class of every manifest in `manifests` to `table`.
pub(crate) fn seed(
    table: &mut SignatureTable,
    interner: &mut TypeInterner,
    manifests: &[Manifest],
) {
    for manifest in manifests {
        let qname = QName::parse(&manifest.class);
        let mut sigs = FxHashMap::default();
        for (method, types) in methods(manifest) {
            sigs.insert(method.name.clone(), method_sig(method, &types, interner));
        }
        table.seed_class(qname.clone(), FxHashMap::default(), sigs);
        let constants = consts(manifest)
            .map(|(constant, ty, value)| {
                let ty = lower(&ty, interner);
                (
                    constant.name.clone(),
                    ConstSig {
                        ty,
                        value: Some(value),
                    },
                )
            })
            .collect();
        table.seed_constants(qname, constants);
    }
}

/// One method's parameter types and its return type, `None` for `void`.
struct Types {
    params: Vec<NovisType>,
    returns: Option<NovisType>,
}

/// Every method of `manifest` whose types are all in the table, with those types.
fn methods(manifest: &Manifest) -> impl Iterator<Item = (&Method, Types)> {
    manifest.methods.iter().filter_map(|method| {
        let params = method
            .params
            .iter()
            .map(|param| NovisType::parse(&param.ty).ok())
            .collect::<Option<Vec<_>>>()?;
        let returns = match method.returns.as_str() {
            "void" => None,
            text => Some(NovisType::parse(text).ok()?),
        };
        Some((method, Types { params, returns }))
    })
}

/// Every constant of `manifest` whose type is in the table and whose value is of that type, with
/// the type and the value.
fn consts(manifest: &Manifest) -> impl Iterator<Item = (&Const, NovisType, ConstArg)> {
    manifest.consts.iter().filter_map(|constant| {
        let ty = NovisType::parse(&constant.ty).ok()?;
        let value = const_value(&ty, &constant.value)?;
        Some((constant, ty, value))
    })
}

/// `value` as a constant of the type `ty`, or `None` when it is not one or has no constant form.
fn const_value(ty: &NovisType, value: &serde_json::Value) -> Option<ConstArg> {
    match ty {
        NovisType::Optional(_) if value.is_null() => Some(ConstArg::Null),
        NovisType::Optional(inner) => const_value(inner, value),
        NovisType::Bool => value.as_bool().map(ConstArg::Bool),
        NovisType::Int => value.as_i64().map(ConstArg::Int),
        NovisType::Uint => value.as_u64().map(ConstArg::Uint),
        NovisType::Float => value.as_f64().map(ConstArg::Float),
        NovisType::String => value.as_str().map(|text| ConstArg::Str(text.to_owned())),
        _ => None,
    }
}

/// One manifest method as the checker's own signature.
fn method_sig(method: &Method, types: &Types, interner: &mut TypeInterner) -> MethodSig {
    let count = method.params.len();
    let returns = match &types.returns {
        Some(ty) => lower(ty, interner),
        None => interner.void(),
    };
    MethodSig {
        params: types.params.iter().map(|ty| lower(ty, interner)).collect(),
        returns_static: false,
        param_names: method
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect(),
        param_quals: method
            .params
            .iter()
            .map(|param| {
                Some(if param.sink {
                    Qual::Sink
                } else {
                    Qual::Contagious
                })
            })
            .collect(),
        inout: vec![false; count],
        variadic: false,
        defaults: method
            .params
            .iter()
            .zip(&types.params)
            .map(|(param, ty)| {
                param
                    .default
                    .as_ref()
                    .and_then(|value| const_value(ty, value))
            })
            .collect(),
        type_params: Vec::new(),
        type_bounds: Vec::new(),
        return_ty: if method.source {
            crate::expr::quals::tainted_result(returns, interner)
        } else {
            returns
        },
        // An extension declares one class of `static` methods and nothing else
        // (`rule:classes/no-free-functions-or-constants`).
        is_static: true,
        interface_private: false,
        visibility: nvs_syntax::ast::Visibility::Public,
        // The export behind a trampoline is code, so a call never looks for an override.
        has_body: true,
        param_text: vec![ParamText::Plain; count],
    }
}

/// One manifest type into an interned one.
fn lower(ty: &NovisType, interner: &mut TypeInterner) -> TypeId {
    match ty {
        NovisType::Bool => interner.bool_ty(),
        NovisType::Int => interner.int(),
        NovisType::Uint => interner.uint(),
        NovisType::Float => interner.float(),
        NovisType::String => interner.string(),
        NovisType::Bytes => interner.bytes(),
        NovisType::Mixed => interner.mixed(),
        NovisType::List(elem) | NovisType::Keyed(_, elem) => {
            let elem = lower(elem, interner);
            interner.array(elem)
        }
        NovisType::Optional(inner) => {
            let inner = lower(inner, interner);
            let null = interner.null();
            interner.make_union([inner, null])
        }
        NovisType::Shape(fields) => {
            let fields = fields
                .iter()
                .map(|(name, optional, ty)| ShapeField {
                    name: name.clone(),
                    ty: lower(ty, interner),
                    required: !optional,
                })
                .collect();
            interner.shape(fields)
        }
    }
}
