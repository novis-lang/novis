//! The Windows reload endpoint: a named pipe whose name is in a small file at the endpoint's path.
//!
//! The name is `\\.\pipe\nvs-reload-` and 128 random bits from the OS CSPRNG, as 32 lowercase hex
//! digits. The first instance is created with `FILE_FLAG_FIRST_PIPE_INSTANCE`, so no other process
//! can own the name first, and with `PIPE_REJECT_REMOTE_CLIENTS`, so only this host connects.
//!
//! Every instance carries one protected DACL, built from SDDL: full access for this process's user,
//! `BUILTIN\Administrators` and `NT AUTHORITY\SYSTEM`, and nothing for anybody else. That DACL is
//! the peer check, so [`accept`] needs no other.
//!
//! The name file is written under the staged name and renamed into place once the pipe exists.
//! It inherits the private DACL of the `run/` folder it lives in.
//!
//! Cost: one pending pipe instance with 4 KiB buffers each way, one name file, and a DACL built
//! from SDDL once per accepted connection.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use ring::rand::{SecureRandom, SystemRandom};
use windows_sys::Win32::Foundation::{
    ERROR_NO_DATA, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, HANDLE, HLOCAL, INVALID_HANDLE_VALUE,
    LocalFree, WIN32_ERROR,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
    TokenUser,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX, SECURITY_IDENTIFICATION,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    WaitNamedPipeW,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// What every pipe name starts with.
const PREFIX: &str = r"\\.\pipe\nvs-reload-";

/// The size of each instance's buffer, each way. A reload line and its reply are far smaller.
const BUFFER: u32 = 4096;

/// How long a client waits for a busy pipe's next instance, in milliseconds, per try.
const BUSY_WAIT_MS: u32 = 5000;

/// How many times a client opens a busy pipe before it gives up.
const BUSY_TRIES: u32 = 4;

/// The listening side: the pipe's name, its DACL as SDDL, and the instance the next client gets.
#[derive(Debug)]
pub(super) struct Listener {
    /// The pipe name, NUL-terminated.
    name: Vec<u16>,
    /// The DACL every instance is created with.
    sddl: String,
    /// The instance [`accept`] waits on next. `None` only after creating one failed; the next
    /// [`accept`] creates it again.
    pending: Mutex<Option<OwnedHandle>>,
}

/// One end of a connected pipe. Dropping it closes the handle.
#[derive(Debug)]
pub(super) struct Stream(File);

/// Creates the pipe, then writes its name to `staged` and renames that to `path`.
pub(super) fn bind(path: &Path, staged: &Path) -> io::Result<Listener> {
    if path.symlink_metadata().is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("the reload endpoint `{}` already exists", path.display()),
        ));
    }
    let text = random_name()?;
    let name: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let sddl = format!("D:P(A;;FA;;;{})(A;;FA;;;BA)(A;;FA;;;SY)", user_sid()?);
    let first = create_instance(&name, &sddl, true)?;
    std::fs::write(staged, &text)?;
    if let Err(err) = std::fs::rename(staged, path) {
        let _ = std::fs::remove_file(staged);
        return Err(err);
    }
    Ok(Listener {
        name,
        sddl,
        pending: Mutex::new(Some(first)),
    })
}

/// Waits for the next client on the pending instance, creates the instance after it, and returns
/// the connected one.
pub(super) fn accept(listener: &Listener) -> io::Result<Stream> {
    let mut pending = listener
        .pending
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let instance = match pending.take() {
        Some(instance) => instance,
        None => create_instance(&listener.name, &listener.sddl, false)?,
    };
    loop {
        match wait_for_client(&instance) {
            Ok(()) => break,
            // The client closed before the connect: free the instance and wait again.
            Err(err) if is(&err, ERROR_NO_DATA) => disconnect(&instance),
            Err(err) => {
                *pending = Some(instance);
                return Err(err);
            }
        }
    }
    // A failure here leaves `None`, and the next accept tries again.
    *pending = create_instance(&listener.name, &listener.sddl, false).ok();
    Ok(Stream(File::from(instance)))
}

/// Opens the pipe named in the file at `path`, waiting a bounded time while it is busy.
pub(super) fn connect(path: &Path) -> io::Result<Stream> {
    let text = std::fs::read_to_string(path)?;
    if !is_pipe_name(&text) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("`{}` does not name a reload pipe", path.display()),
        ));
    }
    let mut tries = 0;
    loop {
        let opened = OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .security_qos_flags(SECURITY_IDENTIFICATION)
            .open(&text);
        match opened {
            Ok(file) => return Ok(Stream(file)),
            Err(err) if is(&err, ERROR_PIPE_BUSY) && tries < BUSY_TRIES => {
                tries += 1;
                wait_for_instance(&text)?;
            }
            Err(err) => return Err(err),
        }
    }
}

impl Read for Stream {
    /// A closed peer reads as the end of the stream.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Whether `err` is the Win32 error `code`.
fn is(err: &io::Error, code: WIN32_ERROR) -> bool {
    err.raw_os_error().and_then(|raw| u32::try_from(raw).ok()) == Some(code)
}

/// Whether `text` is [`PREFIX`] and 32 lowercase hex digits.
fn is_pipe_name(text: &str) -> bool {
    text.strip_prefix(PREFIX).is_some_and(|hex| {
        hex.len() == 32 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    })
}

/// A fresh pipe name with 128 random bits.
fn random_name() -> io::Result<String> {
    let mut bits = [0u8; 16];
    SystemRandom::new()
        .fill(&mut bits)
        .map_err(|_| io::Error::other("the OS random source failed"))?;
    let mut name = String::with_capacity(PREFIX.len() + 32);
    name.push_str(PREFIX);
    for byte in bits {
        name.push_str(&format!("{byte:02x}"));
    }
    Ok(name)
}

/// One pipe instance named `name`, with the DACL `sddl` describes. `first` adds
/// `FILE_FLAG_FIRST_PIPE_INSTANCE`, so the create fails if the name is taken.
#[expect(
    unsafe_code,
    reason = "one `CreateNamedPipeW` over a NUL-terminated name the caller owns and attributes \
              whose descriptor outlives the call; the handle it returns is owned at once"
)]
fn create_instance(name: &[u16], sddl: &str, first: bool) -> io::Result<OwnedHandle> {
    let descriptor = Descriptor::from_sddl(sddl)?;
    let attributes = descriptor.attributes();
    let open_mode = if first {
        PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE
    } else {
        PIPE_ACCESS_DUPLEX
    };
    // SAFETY: `name` is NUL-terminated, and `attributes` points at `descriptor`, alive to the end
    // of this function.
    let handle = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            open_mode,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES,
            BUFFER,
            BUFFER,
            0,
            &raw const attributes,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `handle` is a valid pipe handle nothing else owns.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

/// Blocks until a client connects to `instance`. A client that connected first counts.
#[expect(
    unsafe_code,
    reason = "a synchronous `ConnectNamedPipe` on a pipe handle the caller owns, with no overlapped"
)]
fn wait_for_client(instance: &OwnedHandle) -> io::Result<()> {
    // SAFETY: the handle is a valid server pipe handle, opened without `FILE_FLAG_OVERLAPPED`, so a
    // null `OVERLAPPED` is allowed.
    let connected = unsafe { ConnectNamedPipe(instance.as_raw_handle(), std::ptr::null_mut()) };
    if connected != 0 {
        return Ok(());
    }
    let err = io::Error::last_os_error();
    if is(&err, ERROR_PIPE_CONNECTED) {
        return Ok(());
    }
    Err(err)
}

/// Ends the connection on `instance`, so it can wait for a client again.
#[expect(
    unsafe_code,
    reason = "`DisconnectNamedPipe` on a server pipe handle the caller owns"
)]
fn disconnect(instance: &OwnedHandle) {
    // SAFETY: the handle is a valid server pipe handle.
    unsafe { DisconnectNamedPipe(instance.as_raw_handle()) };
}

/// Waits up to [`BUSY_WAIT_MS`] for an instance of the pipe `name` to be free.
#[expect(
    unsafe_code,
    reason = "`WaitNamedPipeW` over a NUL-terminated name this frame owns"
)]
fn wait_for_instance(name: &str) -> io::Result<()> {
    let wide: Vec<u16> = std::ffi::OsStr::new(name)
        .encode_wide()
        .chain(Some(0))
        .collect();
    // SAFETY: `wide` is NUL-terminated and alive for the call.
    if unsafe { WaitNamedPipeW(wide.as_ptr(), BUSY_WAIT_MS) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// This process's user SID, as SDDL writes it.
#[expect(
    unsafe_code,
    reason = "the size-then-read shape `GetTokenInformation` requires, on a token handle owned at \
              once, into a buffer this function owns and sized as the first call said"
)]
fn user_sid() -> io::Result<String> {
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: the out-parameter is a stack handle this call owns.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `token` is a valid handle nothing else owns.
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    let mut needed = 0u32;
    // SAFETY: the size query; a null buffer of length 0 is allowed, and it fails by design.
    unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            std::ptr::null_mut(),
            0,
            &raw mut needed,
        )
    };
    // `u64`s, so the `TOKEN_USER` at the front is aligned.
    let mut buffer: Vec<u64> = vec![0; (needed as usize).div_ceil(8)];
    // SAFETY: the buffer has at least `needed` bytes.
    let read = unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            buffer.as_mut_ptr().cast(),
            needed,
            &raw mut needed,
        )
    };
    if read == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the call succeeded, so the buffer starts with an aligned `TOKEN_USER` whose SID
    // points inside the buffer, which outlives this read.
    let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
    sid_string(user.User.Sid)
}

/// `sid` as text, such as `S-1-5-18`.
#[expect(
    unsafe_code,
    reason = "`ConvertSidToStringSidW` allocates the text with `LocalAlloc`, so it is measured, \
              copied and freed here"
)]
fn sid_string(sid: PSID) -> io::Result<String> {
    let mut text: *mut u16 = std::ptr::null_mut();
    // SAFETY: `sid` is a valid SID the caller keeps alive; the out-parameter is this frame's.
    if unsafe { ConvertSidToStringSidW(sid, &raw mut text) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut len = 0;
    // SAFETY: the call returned a NUL-terminated string.
    while unsafe { *text.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: `len` units before the NUL are initialized and alive until the free below.
    let out = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, len) });
    // SAFETY: the string came from `LocalAlloc` and is freed once.
    unsafe { LocalFree(text.cast()) };
    Ok(out)
}

/// A security descriptor built from SDDL, freed with `LocalFree` on drop.
#[derive(Debug)]
struct Descriptor(PSECURITY_DESCRIPTOR);

impl Descriptor {
    /// The descriptor `sddl` describes.
    #[expect(
        unsafe_code,
        reason = "one `advapi32` call over a NUL-terminated buffer this frame owns; the guard \
                  returned owns the descriptor it allocates"
    )]
    fn from_sddl(sddl: &str) -> io::Result<Self> {
        let wide: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
        let mut raw: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        // SAFETY: `wide` is NUL-terminated; the out-parameter is this frame's.
        let built = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                wide.as_ptr(),
                SDDL_REVISION_1,
                &raw mut raw,
                std::ptr::null_mut(),
            )
        };
        if built == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(raw))
    }

    /// The attributes a create call takes, pointing at this descriptor. They must not outlive it.
    fn attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: u32::try_from(std::mem::size_of::<SECURITY_ATTRIBUTES>())
                .expect("a Win32 struct is far smaller than a `u32` counts"),
            lpSecurityDescriptor: self.0,
            bInheritHandle: 0,
        }
    }
}

impl Drop for Descriptor {
    #[expect(
        unsafe_code,
        reason = "`ConvertStringSecurityDescriptorToSecurityDescriptorW` allocates with \
                  `LocalAlloc`, so this is the documented way to release it"
    )]
    fn drop(&mut self) {
        // SAFETY: the descriptor came from `LocalAlloc` and is freed once.
        unsafe { LocalFree(self.0 as HLOCAL) };
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;

    use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, HLOCAL, LocalFree};
    use windows_sys::Win32::Security::Authorization::{GetSecurityInfo, SE_KERNEL_OBJECT};
    use windows_sys::Win32::Security::{
        ACCESS_ALLOWED_ACE, ACL, ACL_SIZE_INFORMATION, AclSizeInformation,
        DACL_SECURITY_INFORMATION, GetAce, GetAclInformation, PSECURITY_DESCRIPTOR,
    };
    use windows_sys::Win32::Storage::FileSystem::READ_CONTROL;
    use windows_sys::Win32::System::SystemServices::ACCESS_ALLOWED_ACE_TYPE;

    use super::{PREFIX, accept, bind, connect, sid_string, user_sid};

    /// One line from `stream`, without its newline.
    fn line(stream: &mut impl Read) -> String {
        let mut text = String::new();
        BufReader::new(stream).read_line(&mut text).unwrap();
        text.trim_end().to_string()
    }

    #[test]
    fn a_line_goes_each_way_and_two_connections_are_accepted_in_turn() {
        let dir = nvs_repo::scratch("endpoint-pipe-round-trip");
        let path = dir.join("serve-1-1");
        let listener = bind(&path, &dir.join("serve-1-1.new")).unwrap();
        let name = std::fs::read_to_string(&path).unwrap();
        assert!(name.starts_with(PREFIX) && name.len() == PREFIX.len() + 32);
        assert!(!dir.join("serve-1-1.new").exists());
        let server = std::thread::spawn(move || {
            for reply in ["first", "second"] {
                let mut stream = accept(&listener).unwrap();
                let got = line(&mut stream);
                stream
                    .write_all(format!("{reply} {got}\n").as_bytes())
                    .unwrap();
            }
        });
        for (send, want) in [("a", "first a"), ("b", "second b")] {
            let mut client = connect(&path).unwrap();
            client.write_all(format!("{send}\n").as_bytes()).unwrap();
            assert_eq!(line(&mut client), want);
        }
        server.join().unwrap();
    }

    #[test]
    fn binding_over_an_existing_path_is_already_exists() {
        let dir = nvs_repo::scratch("endpoint-pipe-exists");
        let path = dir.join("serve-1-1");
        std::fs::write(&path, "taken").unwrap();
        let err = bind(&path, &dir.join("serve-1-1.new")).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn a_name_file_whose_pipe_is_gone_is_stale() {
        let dir = nvs_repo::scratch("endpoint-pipe-stale");
        let path = dir.join("serve-1-1");
        std::fs::write(&path, format!("{PREFIX}{}", "0".repeat(32))).unwrap();
        let err = connect(&path).unwrap_err();
        assert!(super::super::is_stale(&err), "{err}");
        std::fs::write(&path, r"\\.\pipe\somebody-else").unwrap();
        let err = connect(&path).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(super::super::is_stale(&err));
    }

    #[test]
    #[expect(
        unsafe_code,
        reason = "reads a pipe's DACL with `GetSecurityInfo`, walks it with `GetAce`, and frees \
                  the descriptor once, while the client handle it was read from is alive"
    )]
    fn the_pipe_admits_only_this_account_administrators_and_system() {
        let dir = nvs_repo::scratch("endpoint-pipe-dacl");
        let path = dir.join("serve-1-1");
        let _listener = bind(&path, &dir.join("serve-1-1.new")).unwrap();
        let name = std::fs::read_to_string(&path).unwrap();
        let client = std::fs::OpenOptions::new()
            .access_mode(GENERIC_READ | GENERIC_WRITE | READ_CONTROL)
            .open(&name)
            .unwrap();
        let mut dacl: *mut ACL = std::ptr::null_mut();
        let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        // SAFETY: the handle is open with `READ_CONTROL`; the out-parameters are this frame's.
        let status = unsafe {
            GetSecurityInfo(
                client.as_raw_handle(),
                SE_KERNEL_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &raw mut dacl,
                std::ptr::null_mut(),
                &raw mut descriptor,
            )
        };
        assert_eq!(status, 0, "GetSecurityInfo failed");
        assert!(!dacl.is_null(), "the pipe has a DACL");
        let mut size = ACL_SIZE_INFORMATION {
            AceCount: 0,
            AclBytesInUse: 0,
            AclBytesFree: 0,
        };
        // SAFETY: `dacl` points into `descriptor`, alive until the free below.
        let read = unsafe {
            GetAclInformation(
                dacl,
                (&raw mut size).cast(),
                u32::try_from(std::mem::size_of::<ACL_SIZE_INFORMATION>()).unwrap(),
                AclSizeInformation,
            )
        };
        assert_ne!(read, 0);
        let mut sids = BTreeSet::new();
        for index in 0..size.AceCount {
            let mut ace: *mut core::ffi::c_void = std::ptr::null_mut();
            // SAFETY: `index` is below the ACE count.
            assert_ne!(unsafe { GetAce(dacl, index, &raw mut ace) }, 0);
            // SAFETY: every ACE starts with the header an `ACCESS_ALLOWED_ACE` starts with, and
            // the type is checked before the SID is read.
            let allowed = unsafe { &*ace.cast::<ACCESS_ALLOWED_ACE>() };
            assert_eq!(u32::from(allowed.Header.AceType), ACCESS_ALLOWED_ACE_TYPE);
            let sid = (&raw const allowed.SidStart).cast_mut().cast();
            sids.insert(sid_string(sid).unwrap());
        }
        // SAFETY: `GetSecurityInfo` allocated it with `LocalAlloc`; it is freed once.
        unsafe { LocalFree(descriptor as HLOCAL) };
        assert_eq!(size.AceCount, 3);
        let want: BTreeSet<String> = [
            user_sid().unwrap(),
            "S-1-5-32-544".into(),
            "S-1-5-18".into(),
        ]
        .into_iter()
        .collect();
        assert_eq!(sids, want);
    }
}
