//! The one error this crate returns: a kind a program branches on, the LDAP result code when a server sent one, and a message with no credential in it
//!
//! [`Kind`] is ADR 0278 § 10's list one for one, so `Core\Ldap`'s `LdapError`
//! reads its `$kind` off [`Kind::name`] and its `$code` off [`Error::code`]
//! with nothing to translate. The mapping from a result code to a kind lives
//! here, beside the kinds, because it is protocol knowledge: which code AD
//! sends for a cleartext bind it will not take, and which sub-code in a
//! `invalidCredentials` diagnostic means a locked account.
//!
//! **No message carries a password or a filter value.** A message is built
//! from the operation's name, the result code and the server's own diagnostic,
//! and a server's diagnostic names neither.

use std::fmt;
use std::io;

/// What went wrong, as a program branches on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A wrong password, a name with no account, or an empty password.
    InvalidCredentials,
    /// AD `data 533`.
    AccountDisabled,
    /// AD `data 775`.
    AccountLocked,
    /// AD `data 532`.
    PasswordExpired,
    /// AD `data 773`.
    MustChangePassword,
    /// AD `data 701`.
    AccountExpired,
    /// AD `data 530` and `531`: outside the logon hours or from another workstation.
    NotAllowedNow,
    /// The server wants TLS, or the client will not bind without it.
    EncryptionRequired,
    /// A new password the directory's policy does not accept.
    PasswordPolicy,
    /// `noSuchObject`.
    NoSuchObject,
    /// `entryAlreadyExists` and `attributeOrValueExists`.
    AlreadyExists,
    /// `insufficientAccessRights`.
    InsufficientAccess,
    /// A value or a change the schema does not allow.
    ConstraintViolation,
    /// The server stopped a search at its size limit.
    SizeLimitExceeded,
    /// The server stopped a search at its time limit.
    TimeLimitExceeded,
    /// The server answered with a referral, which this client never follows.
    Referral,
    /// A write to a server that only reads.
    ReadOnly,
    /// An operation or a control the server does not support.
    Unsupported,
    /// The server could not be reached, or went away.
    Unavailable,
    /// The deadline passed.
    Timeout,
    /// A message the protocol does not allow.
    Protocol,
}

impl Kind {
    /// The kind's name as `Core\Ldap` spells its case.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::InvalidCredentials => "InvalidCredentials",
            Self::AccountDisabled => "AccountDisabled",
            Self::AccountLocked => "AccountLocked",
            Self::PasswordExpired => "PasswordExpired",
            Self::MustChangePassword => "MustChangePassword",
            Self::AccountExpired => "AccountExpired",
            Self::NotAllowedNow => "NotAllowedNow",
            Self::EncryptionRequired => "EncryptionRequired",
            Self::PasswordPolicy => "PasswordPolicy",
            Self::NoSuchObject => "NoSuchObject",
            Self::AlreadyExists => "AlreadyExists",
            Self::InsufficientAccess => "InsufficientAccess",
            Self::ConstraintViolation => "ConstraintViolation",
            Self::SizeLimitExceeded => "SizeLimitExceeded",
            Self::TimeLimitExceeded => "TimeLimitExceeded",
            Self::Referral => "Referral",
            Self::ReadOnly => "ReadOnly",
            Self::Unsupported => "Unsupported",
            Self::Unavailable => "Unavailable",
            Self::Timeout => "Timeout",
            Self::Protocol => "Protocol",
        }
    }

    /// The kind a result code means, with AD's sub-code read out of
    /// `diagnostic` where the code alone does not say enough.
    #[must_use]
    pub fn of_result(code: u32, diagnostic: &str) -> Self {
        match code {
            3 => Self::TimeLimitExceeded,
            // `adminLimitExceeded` is AD's answer to a search past its
            // `MaxPageSize` with no paging control, which is the same stop.
            4 | 11 => Self::SizeLimitExceeded,
            7 | 12 => Self::Unsupported,
            8 | 13 => Self::EncryptionRequired,
            10 => Self::Referral,
            19 | 53
                if leading_code(diagnostic) == Some(0x52D)
                    || ad_sub_code(diagnostic) == Some(0x52D) =>
            {
                Self::PasswordPolicy
            }
            // `noSuchAttribute` is a `remove` of a value the entry does not have.
            16 | 19 | 21 | 65 | 67 | 69 => Self::ConstraintViolation,
            20 | 68 => Self::AlreadyExists,
            32 => Self::NoSuchObject,
            49 => match ad_sub_code(diagnostic) {
                Some(0x530 | 0x531) => Self::NotAllowedNow,
                Some(0x532) => Self::PasswordExpired,
                Some(0x533) => Self::AccountDisabled,
                Some(0x701) => Self::AccountExpired,
                Some(0x773) => Self::MustChangePassword,
                Some(0x775) => Self::AccountLocked,
                // 525 (no such user) and 52e (wrong password) both land here,
                // so a caller cannot learn which accounts exist.
                _ => Self::InvalidCredentials,
            },
            50 => Self::InsufficientAccess,
            51 | 52 => Self::Unavailable,
            53 => Self::Unsupported,
            _ => Self::Protocol,
        }
    }
}

/// The hexadecimal number after `data ` in an AD diagnostic, such as
/// `80090308: LdapErr: DSID-0C09050F, comment: AcceptSecurityContext error, data 52e, v4f7c`.
fn ad_sub_code(diagnostic: &str) -> Option<u32> {
    let (_, after) = diagnostic.split_once("data ")?;
    let digits: &str = after
        .split(|c: char| !c.is_ascii_hexdigit())
        .next()
        .filter(|digits| !digits.is_empty())?;
    u32::from_str_radix(digits, 16).ok()
}

/// The Windows error code that opens a diagnostic, such as the `0000052D` of
/// `0000052D: Constraint violation - check_password_restrictions: ...`, which
/// is how AD and Samba say a password write broke the domain's policy.
fn leading_code(diagnostic: &str) -> Option<u32> {
    let (digits, _) = diagnostic.split_once(':')?;
    if digits.len() != 8 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(digits, 16).ok()
}

/// A failed LDAP operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    kind: Kind,
    code: Option<u32>,
    message: String,
}

impl Error {
    /// An error this client found, with no result code.
    #[must_use]
    pub fn new(kind: Kind, message: impl Into<String>) -> Self {
        Self {
            kind,
            code: None,
            message: message.into(),
        }
    }

    /// An error a server sent: `operation` failed with `code`, and the
    /// server said `diagnostic`.
    #[must_use]
    pub fn from_result(operation: &str, code: u32, diagnostic: &str) -> Self {
        let kind = Kind::of_result(code, diagnostic);
        let message = if diagnostic.is_empty() {
            format!("{operation} failed with LDAP result {code}")
        } else {
            format!("{operation} failed with LDAP result {code}: {diagnostic}")
        };
        Self {
            kind,
            code: Some(code),
            message,
        }
    }

    /// An error the socket or the TLS session reported during `operation`.
    #[must_use]
    pub fn from_io(operation: &str, error: &io::Error) -> Self {
        let kind = match error.kind() {
            io::ErrorKind::TimedOut => Kind::Timeout,
            _ => Kind::Unavailable,
        };
        Self::new(kind, format!("{operation}: {error}"))
    }

    /// What went wrong.
    #[must_use]
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The LDAP result code, when a server sent one.
    #[must_use]
    pub fn code(&self) -> Option<u32> {
        self.code
    }

    /// The message, which names no credential.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    const AD: &str =
        "80090308: LdapErr: DSID-0C09050F, comment: AcceptSecurityContext error, data ";

    #[test]
    fn ad_sub_codes_name_the_reason_and_hide_whether_the_account_exists() {
        for (sub, kind) in [
            ("525", Kind::InvalidCredentials),
            ("52e", Kind::InvalidCredentials),
            ("530", Kind::NotAllowedNow),
            ("531", Kind::NotAllowedNow),
            ("532", Kind::PasswordExpired),
            ("533", Kind::AccountDisabled),
            ("701", Kind::AccountExpired),
            ("773", Kind::MustChangePassword),
            ("775", Kind::AccountLocked),
        ] {
            let diagnostic = format!("{AD}{sub}, v4f7c");
            assert_eq!(Kind::of_result(49, &diagnostic), kind, "{sub}");
        }
        assert_eq!(Kind::of_result(49, ""), Kind::InvalidCredentials);
    }

    #[test]
    fn a_password_policy_refusal_is_read_from_the_leading_code() {
        for diagnostic in [
            "0000052D: Constraint violation - check_password_restrictions: the password is too short.",
            "0000052D: AtrErr: DSID-03191083, #1:",
        ] {
            assert_eq!(
                Kind::of_result(19, diagnostic),
                Kind::PasswordPolicy,
                "{diagnostic}"
            );
        }
        assert_eq!(
            Kind::of_result(19, "00002082: AtrErr: DSID-03151E8A"),
            Kind::ConstraintViolation
        );
    }

    #[test]
    fn a_refused_cleartext_bind_is_encryption_required() {
        assert_eq!(Kind::of_result(8, ""), Kind::EncryptionRequired);
        let error = Error::from_result("the bind", 8, "BindSimple: Transport encryption required.");
        assert_eq!(error.kind(), Kind::EncryptionRequired);
        assert_eq!(error.code(), Some(8));
    }
}
