//! The compiler's tables as a running program carries them.
//!
//! The checker's route table becomes `nvs_runtime::routes::Routes` here, and an
//! enum case's constant becomes `nvs_runtime::commands::CaseValue`. This is the
//! one home of that crossing, so `nvs` and the language server match a path
//! against the same table.

/// `rule:routing/matched-once-before-the-handler`'s table, as the runtime carries it.
///
/// A copy rather than a borrow, for `runtime_commands`' reason exactly: the
/// context outlives the front end's own tables in every caller, and a request
/// owns the table it was matched against. It is a handful of `String`s per
/// declared route, taken once per compiled unit and shared by every request
/// that unit answers.
///
/// **The OpenAPI half of a row does not cross** — summary, tags, security,
/// errors and example are `rule:routing/api-document-is-generated-from-the-route-table`'s document, generated from the compiler's
/// own table, and nothing a request asks reads them.
///
/// The one thing here that is a reading rather than a copy is the conversion:
/// `nvs_types::routes::RouteParam` carries the declared type as
/// `TypeInterner::describe` rendered it, and `nvs_runtime::routes::CaptureConv`
/// is the closed set a matcher needs instead. That enum's `Unconverted` doc
/// owns what is left under it.
pub fn runtime_routes(exprs: &crate::ExprTypeTable) -> nvs_runtime::routes::Routes {
    let table = exprs.routes();
    let crossed = nvs_runtime::routes::Routes::new(
        table
            .rows()
            .iter()
            .map(|row| {
                let route = nvs_runtime::routes::Route::new(
                    row.verb.clone(),
                    row.path.clone(),
                    row.name.as_ref().map(|(name, _)| name.clone()),
                    row.handler.clone(),
                    row.access.clone(),
                    row.params
                        .iter()
                        // `rule:routing/a-query-parameter-is-declared-like-a-capture`'s `#[Query]` parameters are declared on
                        // the same signature and are not part of the path, so
                        // they are not what a segment fills.
                        .filter(|param| param.source == crate::ParamIn::Path)
                        .map(|param| nvs_runtime::routes::Capture {
                            name: param.name.clone(),
                            conv: capture_conv(param),
                        })
                        .collect(),
                )
                .with_queries(
                    row.params
                        .iter()
                        .filter(|param| param.source == crate::ParamIn::Query)
                        .map(|param| {
                            let conv = capture_conv(param);
                            nvs_runtime::routes::QueryParam {
                                name: param.name.clone(),
                                default: param
                                    .default
                                    .as_ref()
                                    .and_then(|constant| query_default(constant, &conv)),
                                conv,
                            }
                        })
                        .collect(),
                );
                // `rule:attributes/access-payload`'s opt-out, carried across rather than derived a
                // second time: the verb's half of § 4 is `Route::new`'s and the
                // declaration's half is the compiler's, so this is the one
                // place the two meet.
                let route = if row.csrf {
                    route
                } else {
                    route.without_csrf()
                };
                let route = if row.static_handler {
                    route.with_static_handler()
                } else {
                    route
                };
                let route = if row.returns_value {
                    route.with_returned_value()
                } else {
                    route
                };
                if row.slotted {
                    route.with_slotted()
                } else {
                    route
                }
            })
            .collect(),
    );
    // The link half of the same crossing, off the call sites the checker
    // resolved rather than off the rows above: a mount's boot check asks
    // whether the unit *builds* an absolute link, which no declaration says.
    // Taken here so that every program this binary runs carries the same
    // answer, since a table marked in one builder and not in another is a
    // check that passes on which path compiled it.
    let crossed = if exprs.links_absolutely() {
        crossed.linking_absolutely()
    } else {
        crossed
    };
    // And the dispatch half, for the same reason: whether the unit calls
    // `Core\Router::dispatch` is a call the checker saw, and the door reads it
    // off this table to choose refusal mode.
    if table.dispatches() {
        crossed.dispatching()
    } else {
        crossed
    }
}

/// Which conversion § 5's declared type is, as the matcher spells it.
///
/// A closed set of *values* takes precedence over the type that describes it —
/// a union of string literals renders as a type nothing would convert, and its
/// admitted set is the whole of what § 5 narrows with. Everything the runtime
/// has no arm for is `Unconverted` rather than silently `Text`, so the gap is
/// one an arm closes rather than a behaviour somebody has to notice. A class
/// implementing `Parses` is its own arm and carries its own name, so the
/// crossing can reach the `parse` this walk may not —
/// `nvs_runtime::routes::CaptureConv::Parses` is the home of that split.
fn capture_conv(param: &crate::RouteParam) -> nvs_runtime::routes::CaptureConv {
    use nvs_runtime::routes::CaptureConv;

    // Above the closed set because an enum subset is both: its spellings are a
    // set a link is checked against, and only this arm carries what each one
    // converts to, which is the difference between handing a program its case
    // and handing it the segment.
    if let Some(narrowed) = &param.cases {
        return CaptureConv::Enum {
            class: narrowed.class.clone(),
            cases: narrowed
                .cases
                .iter()
                .map(|(spelling, value)| (spelling.clone(), case_value(*value)))
                .collect(),
        };
    }
    if let Some(allowed) = &param.allowed {
        return CaptureConv::OneOf(allowed.clone());
    }
    if let Some(conv) = param
        .ty
        .as_deref()
        .and_then(nvs_stdlib::router::native_conv)
    {
        return conv;
    }
    match param.ty.as_deref() {
        // A capture is always `tainted`, and both spellings render for one
        // declared `string` depending on where the qualifier was written.
        Some("string" | "tainted string") | None => CaptureConv::Text,
        // Below the `Core\Uuid` arm of `native_conv` on purpose: that class
        // implements `Parses` too, so a rule reading the interface alone would
        // take the one type the engine ships off the match and defer it to the
        // crossing.
        Some(class) if param.parses => CaptureConv::Parses(class.to_owned()),
        Some(_) => CaptureConv::Unconverted,
    }
}

/// A `#[Query]` parameter's folded default as the value an absent key gives it.
///
/// Converted by the parameter's own `conv`, so a default and a written value
/// are the same value: `uint $page = 1` gives what `?page=1` gives. An enum case
/// folds to its backing value, which is already what [`CaptureConv::Enum`]
/// converts a spelling to, so it is checked against the admitted cases rather
/// than spelled again.
///
/// `None` where no conversion has an answer, which leaves the key required.
/// No program that compiles reaches it: a default is a literal of the declared
/// type, and every type a `#[Query]` parameter may declare and a literal may
/// write converts here.
///
/// [`CaptureConv::Enum`]: nvs_runtime::routes::CaptureConv::Enum
fn query_default(
    constant: &crate::ConstArg,
    conv: &nvs_runtime::routes::CaptureConv,
) -> Option<nvs_runtime::routes::Param> {
    use crate::ConstArg;
    use nvs_runtime::commands::CaseValue;
    use nvs_runtime::routes::{CaptureConv, Param};

    if let CaptureConv::Enum { cases, .. } = conv {
        let value = match constant {
            ConstArg::EnumInt(number) | ConstArg::Int(number) => CaseValue::Int(*number),
            ConstArg::EnumUint(number) | ConstArg::Uint(number) => CaseValue::Uint(*number),
            _ => return None,
        };
        return cases
            .iter()
            .any(|(_, admitted)| *admitted == value)
            .then_some(match value {
                CaseValue::Int(number) => Param::Int(number),
                CaseValue::Uint(number) => Param::Uint(number),
            });
    }
    let text = match constant {
        ConstArg::Int(number) => number.to_string(),
        ConstArg::Uint(number) => number.to_string(),
        ConstArg::Str(text) => text.clone(),
        _ => return None,
    };
    conv.convert(&text)
}

/// One enum case's constant, narrowed to the two integer types a running
/// program holds it in.
///
/// Both tables that carry an enum's cases cross here — a command's argument and
/// a route's capture — because the narrowing is one decision:
/// `nvs_runtime::commands::CaseValue` owns why the value does not travel widened
/// to the `i128` a membership test wants.
pub fn case_value(value: crate::enums::EnumValue) -> nvs_runtime::commands::CaseValue {
    match value {
        crate::enums::EnumValue::Int(number) => nvs_runtime::commands::CaseValue::Int(number),
        crate::enums::EnumValue::Uint(number) => nvs_runtime::commands::CaseValue::Uint(number),
    }
}
