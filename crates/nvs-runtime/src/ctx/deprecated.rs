//! `rule:errors/a-use-of-deprecated-code-may-log-or-throw`'s runtime half: the
//! word compiled code loads at a use of deprecated code, and
//! [`nvs_deprecated_use`], the slow path it branches to when the word is not
//! zero.
//!
//! The word is [`Ctx`]'s `deprecated` field, at [`DEPRECATED_OFFSET`] in the
//! hot line. It is written by [`Ctx::refresh_limits`] from
//! `[errors] deprecated`, so it follows the request's configuration and every
//! `Core\Config::set` or `restore` of it, and a request's setting dies with
//! the request (`rule:config/a-runtime-set-is-request-local`).
//!
//! **What it spends:** one word per context, and under `"log"` one entry per
//! distinct deprecated site the request reaches, freed with the context.

use super::*;

/// What a use of deprecated code does, as the word compiled code loads.
/// `Ignore` is zero, so the branch at a use is taken only when the request
/// asked for something.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u64)]
pub enum OnDeprecated {
    /// Nothing happens.
    #[default]
    Ignore = 0,
    /// One `warning` log record per use site per request.
    Log = 1,
    /// A `Core\DeprecatedError` is thrown.
    Throw = 2,
}

impl From<nvs_config::errors::Deprecated> for OnDeprecated {
    fn from(written: nvs_config::errors::Deprecated) -> Self {
        match written {
            nvs_config::errors::Deprecated::Ignore => Self::Ignore,
            nvs_config::errors::Deprecated::Log => Self::Log,
            nvs_config::errors::Deprecated::Throw => Self::Throw,
        }
    }
}

impl Ctx {
    /// What a use of deprecated code does in this request.
    #[must_use]
    pub fn on_deprecated(&self) -> OnDeprecated {
        match self.deprecated {
            1 => OnDeprecated::Log,
            2 => OnDeprecated::Throw,
            _ => OnDeprecated::Ignore,
        }
    }

    /// Sets what a use of deprecated code does, for a request that may already
    /// be running. [`Self::refresh_limits`] is the caller that follows the
    /// configuration.
    pub fn set_on_deprecated(&mut self, on: OnDeprecated) {
        self.deprecated = on as u64;
    }

    /// `[errors] deprecated` as this request's configuration has it, or
    /// `Ignore` on a context nobody configured.
    pub(crate) fn configured_on_deprecated(&self) -> OnDeprecated {
        self.config.as_ref().map_or(OnDeprecated::Ignore, |config| {
            nvs_config::errors::Deprecated::in_force(config).into()
        })
    }
}

/// The slow path of a use of deprecated code, reached when the context's
/// deprecation word is not zero.
///
/// `message` is `W1003`'s text for the use, baked into the unit once per site,
/// and `at` is the use's `file:line`, empty at a method's entry. Under
/// `"throw"` it raises a `Core\DeprecatedError` carrying `message` and returns
/// [`crate::THROWN`]. Under `"log"` it writes one `warning` record the first
/// time a site runs in a request, keyed by the address of the site's own
/// bytes, and returns [`crate::OK`].
///
/// # Safety
///
/// `ctx` must be non-null, aligned, and valid for the duration of the call,
/// and each pointer must be readable for its length.
#[expect(
    unsafe_code,
    reason = "compiled code passes the context pointer and two byte ranges; \
              the contract cannot be expressed in the signature"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_deprecated_use(
    ctx: *mut Ctx,
    message: *const u8,
    message_len: usize,
    at: *const u8,
    at_len: usize,
) -> i32 {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the context and both ranges are valid; \
                  the only thing that can panic here is the allocator, which \
                  aborts rather than unwinding into the JIT frame above"
    )]
    let (ctx, text, site) = unsafe {
        (
            &mut *ctx,
            text_of(message, message_len),
            text_of(at, at_len),
        )
    };
    match ctx.on_deprecated() {
        OnDeprecated::Ignore => crate::OK,
        OnDeprecated::Throw => {
            ctx.set_pending_as(ThrownClass::Deprecated, text.into_owned());
            crate::THROWN
        }
        OnDeprecated::Log => {
            if ctx.deprecated_logged.insert(message as usize) {
                let line = match site.is_empty() {
                    true => text.into_owned(),
                    false => format!("{text} At {site}."),
                };
                let mut record = crate::floor::note(Level::Warn, &line);
                ctx.stamp_envelope(&mut record.envelope);
                let _ = ctx.write_log_record(&record, crate::LogChannel::Diagnostic);
            }
            crate::OK
        }
    }
}

/// The text in `len` bytes at `ptr`, or `""` when `len` is zero.
///
/// # Safety
///
/// `ptr` must be readable for `len` bytes when `len` is not zero.
#[expect(
    unsafe_code,
    reason = "the range comes from compiled code, and its contract cannot be \
              expressed in the signature"
)]
unsafe fn text_of<'a>(ptr: *const u8, len: usize) -> Cow<'a, str> {
    if len == 0 {
        return Cow::Borrowed("");
    }
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the range is readable; the zero-length \
                  case is split out because `from_raw_parts` rejects a null \
                  pointer even for an empty slice"
    )]
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(ctx: &mut Ctx, message: &'static str, at: &'static str) -> i32 {
        #[expect(unsafe_code, reason = "a live context and two static strings")]
        unsafe {
            nvs_deprecated_use(ctx, message.as_ptr(), message.len(), at.as_ptr(), at.len())
        }
    }

    #[test]
    fn ignore_returns_ok_and_raises_nothing() {
        let mut ctx = Ctx::buffered();
        assert_eq!(run(&mut ctx, "`Api::old` is deprecated.", ""), crate::OK);
        assert!(ctx.pending().is_none());
    }

    #[test]
    fn throw_raises_a_deprecated_error_carrying_the_message() {
        let mut ctx = Ctx::buffered();
        ctx.set_on_deprecated(OnDeprecated::Throw);
        assert_eq!(ctx.on_deprecated(), OnDeprecated::Throw);
        assert_eq!(
            run(&mut ctx, "`Api::old` is deprecated.", "a.nvs:3"),
            crate::THROWN
        );
        assert_eq!(ctx.pending().as_deref(), Some("`Api::old` is deprecated."));
    }

    #[test]
    fn log_remembers_each_site_once() {
        let mut ctx = Ctx::buffered();
        ctx.set_on_deprecated(OnDeprecated::Log);
        let site = "`Api::old` is deprecated.";
        assert_eq!(run(&mut ctx, site, "a.nvs:3"), crate::OK);
        assert_eq!(run(&mut ctx, site, "a.nvs:3"), crate::OK);
        assert_eq!(ctx.deprecated_logged.len(), 1);
    }
}
