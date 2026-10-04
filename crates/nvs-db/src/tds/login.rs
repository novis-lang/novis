//! LOGIN7: a fixed 94-byte header, twelve offset/length pairs pointing past it,
//! and the password obfuscation that is not encryption.
//!
//! Every offset in the message counts from the start of the *message*, so
//! [`LOGIN7_HEADER`] is both the header's width and the smallest value any of
//! them may hold. The password is nibble-swapped and XORed with
//! [`PASSWORD_XOR`] — a published transformation with no key, which is why the
//! socket is already TLS by the time this message is written.

use super::*;

/// LOGIN7's fixed header, and therefore where its first blob starts.
///
/// Twelve scalar fields (36 bytes), twelve offset/length pairs (48), the
/// six-byte `ClientID` and `cbSSPILong`: 94, none of which varies with what
/// this driver has to say. Every offset in the message is counted from the
/// start of the *message*, so this is also the smallest value any of them may
/// hold — a pair pointing below it points into the header that holds it.
pub(super) const LOGIN7_HEADER: u16 = 94;

/// The longest any of LOGIN7's variable-length fields may be, in characters.
///
/// MS-TDS's own limit on `cchUserName`, `cchPassword`, `cchDatabase` and the
/// rest, and it is checked here rather than left to the server: an over-long
/// field comes back as a login failure that names nothing, where the thing an
/// operator has to fix is the `[db.<name>]` block that wrote it. The length
/// field is a `u16` and would hold far more, so this is the protocol's rule
/// and not the field's width.
pub(super) const MAX_FIELD_CHARS: u16 = 128;

/// What LOGIN7 XORs each nibble-swapped password byte with.
///
/// MS-TDS calls the result an encrypted password. It is not one — the
/// transformation is fixed, public and its own inverse — which is why
/// [`TdsTarget::password`] says `rule:core-classes/db-capabilities`'s TLS is not optional on this
/// backend in the way it merely defaults elsewhere.
pub(super) const PASSWORD_XOR: u8 = 0xA5;

/// What this client calls itself, in both `AppName` and `CltIntName`.
///
/// One string in the two fields because on this driver they are the same fact:
/// there is no application name a `[db.<name>]` block carries, and the
/// interface is this crate either way. It is what
/// `sys.dm_exec_sessions.program_name` shows, which is where a SQL Server
/// operator looks to find out whose connection a session is.
pub(super) const CLIENT_NAME: &str = "Novis";

/// `ClientLCID`: US English.
///
/// Not zero, which is not a locale at all. Nothing this driver parses depends
/// on it — [ADR 0067 § 8](/docs/decisions/0067.md) normalises on the
/// error *number* and never on the message text — so all it decides is which
/// language a server writes a message Novis will only ever log.
pub(super) const CLIENT_LCID: u32 = 0x0409;

/// `fUseDB`: the server says so when the database changes under this
/// connection.
///
/// A `USE` also changes the collation the server describes columns with, so a
/// client that had not asked for the notification would be decoding against a
/// collation it could not know had moved. The notification is an `ENVCHANGE`
/// token, which is the slice after this one.
pub(super) const OPT1_USE_DB_NOTIFY: u8 = 0x20;

/// `fDatabase`: failing to open [`TdsTarget::database`] fails the login.
///
/// The decision this message owes. MS-TDS's other reading is a *warning*,
/// which leaves the connection open in whichever database the server made this
/// login's default. [`TdsTarget::database`] is required precisely so that a
/// connection cannot mean whatever that was, and a warning would give the
/// field away at the last moment: § 13's pool would then hold connections
/// whose database depends on server-side state no pool key covers, and a
/// request would read the right rows or the wrong ones depending on how the
/// login was provisioned.
pub(super) const OPT1_INIT_DB_FATAL: u8 = 0x40;

/// `fLanguage`: the same reading for the language, which this driver never
/// names.
///
/// It sends an empty `Language`, so there is no initial change to fail. The
/// bit is set anyway because the alternative reading is "warn and carry on",
/// and a login that half-succeeded is not a state anything above wants to
/// discover later.
pub(super) const OPT2_INIT_LANG_FATAL: u8 = 0x01;

/// `fODBC`, which is not about ODBC: it is how a client asks for the ANSI
/// session defaults.
///
/// The server answers it by setting `ANSI_DEFAULTS` on, `IMPLICIT_TRANSACTIONS`
/// off, `TEXTSIZE` to its maximum and `ROWCOUNT` to unlimited, and two of those
/// are load-bearing. Implicit transactions off is what makes [ADR 0067
/// § 7](/docs/decisions/0067.md)'s `transaction()` the only thing that ever
/// opens a transaction on this connection — with them on, a bare `SELECT`
/// opens one nothing commits, and § 13's reset would be destroying a connection
/// per request. `ROWCOUNT` unlimited is what stops a server-side default from
/// silently truncating a result set.
pub(super) const OPT2_ODBC: u8 = 0x02;

/// `fUnknownCollationHandling`: this client accepts a collation newer than the
/// ones TDS 7.0 knew about.
///
/// Nothing here reads a collation — § 9's map decodes a column by its type —
/// so what the bit buys is that the server is never pushed into describing one
/// the older, lossier way on this client's account.
pub(super) const OPT3_UNKNOWN_COLLATION: u8 = 0x08;

/// `OptionFlags1`.
///
/// Every other bit in it — byte order, character set, float format, dump/load
/// — has exactly one reading this driver could mean, and zero is that reading:
/// little-endian, the ASCII family, IEEE 754, and no BCP.
pub(super) const OPTION_FLAGS_1: u8 = OPT1_USE_DB_NOTIFY | OPT1_INIT_DB_FATAL;

/// `OptionFlags2`.
///
/// `fUserType` stays zero — an ordinary login, not a replication or
/// remote-user one — and `fIntegratedSecurity` stays off, which is what makes
/// the `Password` field the credential rather than an SSPI blob this driver
/// has no way to produce.
pub(super) const OPTION_FLAGS_2: u8 = OPT2_INIT_LANG_FATAL | OPT2_ODBC;

/// `TypeFlags`: an ordinary SQL client, no OLE DB behaviour, and **not**
/// `fReadOnlyIntent` — an availability-group routing hint no `[db.<name>]`
/// field asks for and which this driver would therefore be inventing.
pub(super) const TYPE_FLAGS: u8 = 0x00;

/// `OptionFlags3`.
///
/// `fChangePassword` and `fUserInstance` name features this driver does not
/// offer, and `fExtension` is the only way to send a `FeatureExt` block, which
/// nothing here has anything to put in.
pub(super) const OPTION_FLAGS_3: u8 = OPT3_UNKNOWN_COLLATION;

/// One variable-length LOGIN7 field: its characters appended to `blobs`, and
/// the offset/length pair the fixed header carries for it.
///
/// **The pair's two numbers are in different units**, which is the trap this
/// function exists to have exactly once: the offset is *bytes* from the start
/// of the message, the length is *characters*, so a field at `(at, n)` spans
/// `2n` bytes. Both are little-endian, unlike PRELOGIN's table in the same
/// conversation. A supplementary character is two UTF-16 code units and
/// therefore costs two of [`MAX_FIELD_CHARS`], which is the protocol's
/// accounting and not this driver's.
///
/// `obfuscate` is LOGIN7's nibble swap and XOR, which only the password takes.
///
/// # Errors
///
/// `InvalidInput` for a field past [`MAX_FIELD_CHARS`], naming the key whose
/// `[db.<name>]` value is too long rather than the protocol field it fills.
pub(super) fn placed(
    blobs: &mut Vec<u8>,
    text: &str,
    field: &'static str,
    obfuscate: bool,
) -> io::Result<[u8; 4]> {
    let characters = text.encode_utf16().count();
    let length = u16::try_from(characters)
        .ok()
        .filter(|&n| n <= MAX_FIELD_CHARS)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "this connection's {field} is {characters} characters, past the \
                     {MAX_FIELD_CHARS} MS-TDS gives a LOGIN7 field"
                ),
            )
        })?;

    let at = LOGIN7_HEADER
        + u16::try_from(blobs.len())
            .expect("six non-empty fields of at most 128 characters fit a u16 offset");

    blobs.extend(text.encode_utf16().flat_map(u16::to_le_bytes).map(|byte| {
        if obfuscate {
            // The nibble swap, which on one byte is a rotation by four.
            byte.rotate_left(4) ^ PASSWORD_XOR
        } else {
            byte
        }
    }));

    let mut pair = [0; 4];
    pair[..2].copy_from_slice(&at.to_le_bytes());
    pair[2..].copy_from_slice(&length.to_le_bytes());
    Ok(pair)
}

/// The LOGIN7 message this driver sends, as a payload for [`Wire::send`].
///
/// The whole of what a SQL Server connection is: who is logging in, with what,
/// into which database, and how the session behaves once it is open. It is
/// built from [`TdsTarget`] alone — nothing here reads the environment, so two
/// hosts running one `[db.<name>]` block send byte-identical logins.
///
/// **Everything an eavesdropper would want is in it**, and only the password
/// is disguised at all: the nibble swap and [`PASSWORD_XOR`] are public and
/// reversible, so this message is a credential in the clear unless
/// [`negotiate_tls`] has already run. That is the whole reason § 3's TLS is
/// mandatory rather than defaulted on this backend. The returned buffer holds
/// that credential and nothing zeroes it — deliberately, since the plaintext
/// it was built from lives in the configuration tree for the process's life,
/// and zeroing the copy while the source stays would be a gesture rather than
/// a defence.
///
/// `packet_size` is what the connection is currently framing at
/// ([`Codec::packet_size`]); the server may answer with a different one in an
/// `ENVCHANGE` token, which is what [`Codec::set_packet_size`] is for.
///
/// The fields that go out empty are each a decision. `HostName` is the
/// *client's* machine name, which nothing approved this driver to read and
/// which no server acts on; `Language` is empty so the server's own default
/// governs, and § 8 reads error numbers rather than message text; `SSPI`
/// belongs to integrated security, which [`OPTION_FLAGS_2`] turns off;
/// `AtchDBFile` attaches a database file by path, which is a capability
/// nothing in `rule:core-classes/db-one-api` grants; and `ChangePassword` changes the login's
/// password as a side effect of connecting. `ClientID` is a six-byte MAC
/// address and goes out as zeroes for `HostName`'s reason — it is a stable
/// identifier for the machine, sent to buy nothing.
///
/// # Errors
///
/// [`placed`]'s: `InvalidInput` for a field past [`MAX_FIELD_CHARS`].
pub fn login7_request(target: &TdsTarget<'_>, packet_size: u16) -> io::Result<Vec<u8>> {
    // In the fixed header's own order, because each pair's offset is where the
    // previous field's characters ended.
    let mut blobs = Vec::new();
    let host_name = placed(&mut blobs, "", "client host name", false)?;
    let user = placed(&mut blobs, target.user, "user", false)?;
    let password = placed(&mut blobs, target.password, "password", true)?;
    let app_name = placed(&mut blobs, CLIENT_NAME, "application name", false)?;
    let server_name = placed(&mut blobs, target.host, "host", false)?;
    let extension = placed(&mut blobs, "", "extension", false)?;
    let interface = placed(&mut blobs, CLIENT_NAME, "interface name", false)?;
    let language = placed(&mut blobs, "", "language", false)?;
    let database = placed(&mut blobs, target.database, "database", false)?;
    let sspi = placed(&mut blobs, "", "SSPI blob", false)?;
    let attached_file = placed(&mut blobs, "", "attached database file", false)?;
    let new_password = placed(&mut blobs, "", "new password", false)?;

    let total = usize::from(LOGIN7_HEADER) + blobs.len();
    let length =
        u32::try_from(total).expect("twelve fields of at most 128 characters fit a u32 length");

    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(&TDS_VERSION.to_le_bytes());
    out.extend_from_slice(&u32::from(packet_size).to_le_bytes());
    // `ClientProgVer` and `ClientPID`. The version PRELOGIN already sent is the
    // one fact worth telling a server about this client, and a process id is
    // the host's business.
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    // `ConnectionID`, which only a connection being resumed carries.
    out.extend_from_slice(&0u32.to_le_bytes());
    out.push(OPTION_FLAGS_1);
    out.push(OPTION_FLAGS_2);
    out.push(TYPE_FLAGS);
    out.push(OPTION_FLAGS_3);
    // `ClientTimZone`. MS-TDS documents it as unused, and this is the one
    // driver with nowhere to send [`TdsTarget::time_zone`] anyway: § 9's zone
    // decodes rows here rather than configuring a session.
    out.extend_from_slice(&0i32.to_le_bytes());
    out.extend_from_slice(&CLIENT_LCID.to_le_bytes());

    for pair in [
        host_name,
        user,
        password,
        app_name,
        server_name,
        extension,
        interface,
        language,
        database,
    ] {
        out.extend_from_slice(&pair);
    }
    out.extend_from_slice(&[0; 6]);
    for pair in [sspi, attached_file, new_password] {
        out.extend_from_slice(&pair);
    }
    // `cbSSPILong`, the 32-bit length an SSPI blob past 65535 bytes would need.
    out.extend_from_slice(&0u32.to_le_bytes());

    debug_assert_eq!(
        out.len(),
        usize::from(LOGIN7_HEADER),
        "every offset above was counted against this width"
    );
    out.extend_from_slice(&blobs);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tds::testing::*;

    #[test]
    fn a_login7_carries_the_targets_four_fields_and_says_how_long_it_is() {
        let message = login7(&block());

        assert_eq!(
            usize::try_from(u32::from_le_bytes(
                message[0..4].try_into().expect("four bytes")
            ))
            .expect("a length this small"),
            message.len(),
            "the length field counts the whole message, itself included"
        );
        assert_eq!(message[4..8], TDS_VERSION.to_le_bytes());
        assert_eq!(message[8..12], u32::from(DEFAULT_PACKET_SIZE).to_le_bytes());

        assert_eq!(field_at(&message, 40), "sa");
        assert_eq!(
            field_at(&message, 52),
            "mssql.test",
            "ServerName is the host"
        );
        assert_eq!(field_at(&message, 68), "novis_test");
        assert_eq!(field_at(&message, 48), CLIENT_NAME);
        assert_eq!(field_at(&message, 60), CLIENT_NAME);

        for name in [
            "HostName",
            "Language",
            "SSPI",
            "AtchDBFile",
            "ChangePassword",
        ] {
            let at = PAIRS
                .iter()
                .find(|(field, _)| *field == name)
                .expect("the table names it")
                .1;
            assert_eq!(pair(&message, at).1, 0, "{name} goes out empty");
        }
    }

    /// Every pair points inside the message and past the header that holds it.
    ///
    /// Counted over the whole table rather than read off the fields that carry
    /// text: a pair whose offset is short by the six bytes of `ClientID` still
    /// points at plausible characters, and only the sweep says which one of the
    /// twelve moved.
    #[test]
    fn a_login7s_offsets_all_point_past_its_header_and_inside_the_message() {
        let message = login7(&block());

        assert_eq!(
            pair(&message, 36).0,
            usize::from(LOGIN7_HEADER),
            "the first blob starts where the fixed header ends"
        );
        for (name, at) in PAIRS {
            let (offset, characters) = pair(&message, at);
            assert!(
                offset >= usize::from(LOGIN7_HEADER),
                "{name} points into the header at {offset}"
            );
            assert!(
                offset + characters * 2 <= message.len(),
                "{name} spans past the end of a {}-byte message",
                message.len()
            );
        }
    }

    /// § 3's reason the TLS is not optional, asserted from the wire side.
    #[test]
    fn a_password_is_nibble_swapped_and_xored_and_is_nowhere_in_the_message_in_the_clear() {
        let message = login7(&block());
        let (offset, characters) = pair(&message, 44);
        let sent = &message[offset..offset + characters * 2];

        let plain: Vec<u8> = "hunter2"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(characters, 7, "a length in characters, not in bytes");
        assert_ne!(sent, plain.as_slice());

        // The transform is its own inverse, which is the whole of what MS-TDS
        // calls encryption here.
        let back: Vec<u8> = sent
            .iter()
            .map(|byte| (byte ^ PASSWORD_XOR).rotate_left(4))
            .collect();
        assert_eq!(back, plain);

        for spelling in [plain.as_slice(), b"hunter2".as_slice()] {
            assert!(
                !message
                    .windows(spelling.len())
                    .any(|window| window == spelling),
                "the password appears in the message unobscured"
            );
        }
    }

    /// The option flags, and the decision among them that carries the most: a
    /// failed initial database is fatal, so a login never lands in whichever
    /// database the server made this login's default.
    #[test]
    fn a_failed_initial_database_is_fatal_rather_than_a_warning() {
        let message = login7(&block());

        assert_eq!(
            message[24] & OPT1_INIT_DB_FATAL,
            OPT1_INIT_DB_FATAL,
            "a warning would make the connection's database server-side state"
        );
        assert_eq!(
            message[25] & OPT2_ODBC,
            OPT2_ODBC,
            "implicit transactions off is what makes § 7's `transaction()` the only transaction"
        );
        assert_eq!(message[24], OPTION_FLAGS_1);
        assert_eq!(message[25], OPTION_FLAGS_2);
        assert_eq!(message[26], TYPE_FLAGS);
        assert_eq!(message[27], OPTION_FLAGS_3);

        assert_eq!(
            message[28..32],
            0i32.to_le_bytes(),
            "the block declares +02:00 and § 9's zone still decodes rows rather than \
             configuring a session there is no setting for"
        );
    }

    /// Both sides of the field bound, and the units it counts in.
    #[test]
    fn a_field_one_character_past_the_protocols_limit_is_refused_by_its_key() {
        let mut block = block();
        let limit = usize::from(MAX_FIELD_CHARS);

        block.database = Some("d".repeat(limit));
        assert_eq!(pair(&login7(&block), 68).1, limit, "the last one that fits");

        // Half as many supplementary characters spend the same allowance: the
        // length is code units, and one of these is two of them.
        block.database = Some("🦀".repeat(limit / 2));
        assert_eq!(pair(&login7(&block), 68).1, limit);

        block.database = Some("d".repeat(limit + 1));
        let target = TdsTarget::resolve(&block).expect("an over-long name is still a name");
        let refused =
            login7_request(&target, DEFAULT_PACKET_SIZE).expect_err("one character past the limit");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
        assert!(
            refused.to_string().contains("database"),
            "the refusal names the key an operator has to fix, not the protocol field"
        );
    }
}
