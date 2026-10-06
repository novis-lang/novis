//! The seam a compiled call into an extension reaches its host through: one
//! trait, one thread-local, and the helper [`nvs_extension_call`] every call
//! site names (`rule:packaging/extension-calls-are-statically-typed`).
//!
//! **The edge is inverted here**, for [`crate::host`]'s reason: the guest
//! runtime is wasmtime, which `nvs-codegen` and `nvs check` must not link, so
//! this crate declares [`Extensions`], `nvs-cli` implements it over
//! `nvs_ext::call::Request`, and compiled code names only this helper. A
//! program that calls no extension never reaches this module, and a process
//! that loaded none installs nothing.
//!
//! **The trampoline is one helper with the export named in slot 0.** A call
//! site passes the immortal string `Class::method` ahead of its arguments, in
//! `nvs_ir::Helper::CallCallable`'s variadic shape, and [`Extensions::call`]
//! finds the export by that name. It is a direct call: the compiled unit was
//! typed against the loaded set, which is in its key
//! (`rule:config/the-extension-set-is-in-every-unit-key`), so the name always
//! resolves. What it costs beyond a per-export trampoline is the host's lookup
//! of the name on every call.
//!
//! **Arguments are borrowed** and the result is a fresh reference, as for a
//! `Core` call (`nvs_ir::ir::InstKind::ExtensionCall`). A failure is a
//! [`Fault`], which the host builds from the guest's outcome
//! (`rule:packaging/a-guest-crash-throws`).
//!
//! **A thread-local, not a field of [`Ctx`]**, for [`crate::host`] § 2's
//! reason: a host is per process and per thread, and `Ctx` is `#[repr(C)]`
//! with offsets compiled code reads. What is per request — the instances a
//! request made — is the implementation's to keep.
//!
//! What it spends: one pointer-sized slot per thread, and nothing per call.

use std::cell::RefCell;
use std::rc::Rc;

use crate::ctx::Ctx;
use crate::value::Value;
use crate::{Fault, HelperResult, ThrownClass};

/// The host of the loaded extensions, as a compiled call reaches it.
pub trait Extensions {
    /// Calls `method` of the extension class `class` with `args`, borrowed,
    /// and returns the result as a fresh reference, or `null` for a `void`
    /// method.
    ///
    /// # Errors
    ///
    /// The [`Fault`] the program sees: a throw of the class the guest's
    /// failure maps to, or a `FATAL` for a resource limit.
    fn call(&self, ctx: &mut Ctx, class: &str, method: &str, args: &[Value]) -> HelperResult;
}

thread_local! {
    /// The extensions host for calls on this thread, or `None` when no
    /// extension is loaded.
    static CURRENT: RefCell<Option<Rc<dyn Extensions>>> = const { RefCell::new(None) };
}

/// Makes `host` the one compiled calls on this thread reach, until the
/// returned guard is dropped, which restores the one before it.
#[must_use = "the host is uninstalled when the guard is dropped"]
pub fn install(host: Rc<dyn Extensions>) -> Installed {
    let previous = CURRENT.with(|current| current.replace(Some(host)));
    Installed { previous }
}

/// The guard [`install`] returns.
pub struct Installed {
    previous: Option<Rc<dyn Extensions>>,
}

impl Drop for Installed {
    fn drop(&mut self) {
        let previous = self.previous.take();
        CURRENT.with(|current| *current.borrow_mut() = previous);
    }
}

impl std::fmt::Debug for Installed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Installed").finish_non_exhaustive()
    }
}

/// `nvs_ir::ir::InstKind::ExtensionCall`'s trampoline: slot 0 is the export,
/// `Class::method`, and the rest are its arguments.
///
/// # Safety
///
/// `ctx`, `args` and `out` must each be non-null, aligned and valid for the
/// duration of the call; `args` must point at `argc` initialized values, of
/// which there must be at least one; and `out` must be writable. Compiled Novis
/// code satisfies all of it by construction.
#[expect(
    unsafe_code,
    reason = "the helper ABI's pointer contract, discharged exactly where \
              `nvs_helper!` discharges it for every fixed-arity helper"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_extension_call(
    ctx: *mut Ctx,
    args: *const Value,
    argc: usize,
    out: *mut Value,
) -> i32 {
    #[expect(
        unsafe_code,
        reason = "the caller's contract is exactly `run_helper`'s"
    )]
    unsafe {
        crate::run_helper(ctx, args, argc, out, call_body)
    }
}

/// [`nvs_extension_call`]'s body, over safe slices.
fn call_body(ctx: &mut Ctx, args: &[Value]) -> HelperResult {
    let (export, args) = args
        .split_first()
        .ok_or_else(|| Fault::fatal("an extension call with no export named"))?;
    let (class, method) = export
        .as_text()
        .and_then(|export| export.rsplit_once("::"))
        .ok_or_else(|| Fault::fatal("an extension call whose export is not `Class::method`"))?;
    let host = CURRENT.with(|current| current.borrow().clone());
    let Some(host) = host else {
        return Err(Fault::thrown_as(
            ThrownClass::Extension,
            format!("the extension `{class}` is not loaded in this process"),
        ));
    };
    host.call(ctx, class, method, args)
}
