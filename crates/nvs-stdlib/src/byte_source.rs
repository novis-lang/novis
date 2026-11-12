//! One source of bytes, read by every `Core` member that takes a
//! `Core\Bytes::Source` or a `Core\Bytes::Chunks`.
//!
//! `rule:core-api/a-member-takes-the-source-the-program-holds` is the rule. A
//! member hands its argument to [`ByteSource::from_arg`] once, and then reads
//! it one of three ways:
//!
//! - **forward**, with [`ByteSource::next_chunk`], one chunk per call. A lazy
//!   walk (`Core\Csv::rows`, `Core\Json::decodeEach`) keeps the source in the
//!   state it files on the request, and reads one chunk per `advance()`;
//! - **its start**, with [`ByteSource::prefix`], which consumes nothing;
//! - **in any order**, with [`ByteSource::range`] and [`ByteSource::size`].
//!
//! # An upload part and a body stream
//!
//! A `Core\Request\Part` and a `Core\Request\BodyStream` are read off the
//! wire, which cannot go back. A prefix read holds the start it read, and a
//! random read holds the whole part or body under `[limits] request_body`.
//! The held bytes stay with the request (`crate::request`), so a forward
//! read, `saveTo` or a loop gives them first, and every byte still arrives.
//! A part's held bytes are freed when the files walk moves to the next part.
//!
//! # A file is read whole, from its start
//!
//! Every kind of read starts at offset 0, wherever the program's position was.
//! A prefix or random read moves the position, and [`ByteSource::finish`]
//! puts it back. A forward read leaves it at the end of the file, which is
//! what reading a file to its end does. So `$f->read(4)` and then a member
//! that hashes `$f` hashes the whole file.
//!
//! **The source reads through a second descriptor on the program's file**,
//! made with `try_clone`, which shares the file's position on Unix (`dup`) and
//! on Windows (`DuplicateHandle`). That is what lets [`Drop`] put a position
//! back on a path that never reached [`ByteSource::finish`], such as a walk
//! the request gave up: a drop has no context to find the program's own
//! descriptor in. Every read still asks the context whether the program's
//! handle is open, so a `close()` under a walk stops it.
//!
//! **A file that cannot seek**, such as a pipe or a terminal, is read forward
//! from where it is. Anything but a regular file on a disk counts as one
//! (`nvs_runtime::os::can_seek`). Its start is kept for a prefix read, and
//! [`ByteSource::next_chunk`] gives the kept bytes first. A random read keeps
//! the whole of it, under `[limits] memory`.
//!
//! # A chunk is lent
//!
//! [`Chunk`] lends its bytes. A file's chunk is this source's one buffer,
//! reused for every read. A `bytes` value is lent in [`READ_CHUNK`] pieces of
//! itself. A pulled chunk of an `Iterable` is the reference the pull gave, and
//! the guard releases it on drop, so no caller writes that `unsafe` release.
//!
//! **What it spends:** per source, one buffer for a file, the file's size plus
//! one byte and at most [`READ_CHUNK`], and one more descriptor. A file that cannot seek also keeps what a
//! prefix read took, and a random read keeps the whole file. A `bytes` value
//! and a walk of chunks cost nothing past the chunk being read.
//!
//! # Who owns the value
//!
//! [`ByteSource::from_arg`] gives back the value the source reads from, with a
//! reference of its own: the file, the `bytes`, or the cursor it began over the
//! chunks. The member keeps it in a slot of the object it returns, so the
//! request's own release frees it, and hands it back to every read. The source
//! itself holds no Novis value, so it can be filed on the request as plain Rust
//! state. A lazy walk takes the source out of that state for one `advance()`
//! and files it back after, as it does its parse, because every read needs the
//! context the state is filed in.

use std::io::{Read, Seek, SeekFrom};

use nvs_runtime::capability::Access;
use nvs_runtime::{Ctx, Fault, Value};

/// The most bytes one forward read of a file takes, and the size of each piece
/// a `bytes` value is lent in. A random read is taken in pieces of this size
/// too, with the memory limit checked before each. Under Miri a read is 2 KiB,
/// so a test file several reads long stays small enough for Miri to walk.
pub(crate) const READ_CHUNK: usize = if cfg!(miri) { 2 << 10 } else { 64 << 10 };

/// One argument a member reads bytes from.
#[derive(Debug)]
pub(crate) struct ByteSource {
    /// The member every message names, such as `Core\Csv::rows`.
    member: &'static str,
    /// Which source it is.
    kind: Kind,
    /// Whether a forward read has reached the end. Every later one is `None`.
    ended: bool,
}

/// The sources.
#[derive(Debug)]
enum Kind {
    /// An open `Core\IO\File`.
    File(FileSource),
    /// A `bytes` or `string` value, and how far a forward read has lent it.
    Bytes { at: usize },
    /// A walk of chunks, and where an array's walk goes on.
    Chunks { from: usize },
    /// The `Core\Request\Part` the files walk is on, and the buffer each
    /// forward read copies one wire run into.
    Part { buffer: Vec<u8> },
    /// A `Core\Request\BodyStream`, which is its own cursor.
    Stream { from: usize },
}

/// An open `Core\IO\File`, read through a descriptor of its own.
#[derive(Debug)]
struct FileSource {
    /// The key the program's handle is filed under, asked before every read.
    key: u64,
    /// The path, for messages.
    path: String,
    /// A second descriptor on the program's file, sharing its position.
    file: nvs_runtime::capability::File,
    /// Whether the file is a regular file, which can seek.
    seekable: bool,
    /// The one buffer every forward read goes into. It is the file's size
    /// plus one byte, at most [`READ_CHUNK`], so a small file costs no more
    /// than its size.
    buffer: Vec<u8>,
    /// Whether the last forward read filled a buffer smaller than
    /// [`READ_CHUNK`]: the file grew after its size was read, and the next
    /// read takes a whole [`READ_CHUNK`].
    filled: bool,
    /// Where the program's position was before a prefix or random read moved
    /// it. [`ByteSource::finish`] and [`Drop`] put it back.
    origin: Option<u64>,
    /// Whether a forward read has begun, which first moves a seekable file to
    /// its start.
    started: bool,
    /// A file that cannot seek: the start a prefix read took, or the whole
    /// file a random read took. A forward read gives these bytes first.
    kept: Vec<u8>,
    /// How much of [`FileSource::kept`] a forward read has given.
    kept_at: usize,
    /// Whether [`FileSource::kept`] is the whole file.
    kept_whole: bool,
}

/// One chunk a forward read lends. Its bytes are valid until it is dropped,
/// and the next read needs it dropped first.
#[derive(Debug)]
pub(crate) struct Chunk<'s> {
    /// Where the bytes are.
    lent: Lent<'s>,
}

/// Where a [`Chunk`]'s bytes are.
#[derive(Debug)]
enum Lent<'s> {
    /// The source's own buffer.
    Buffer(&'s [u8]),
    /// A piece of the `bytes` value the member keeps.
    Piece(Value, usize, usize),
    /// A chunk an `Iterable` gave, with the reference this guard releases.
    Pulled(Value),
}

impl Chunk<'_> {
    /// The chunk's bytes.
    #[must_use]
    pub(crate) fn bytes(&self) -> &[u8] {
        match &self.lent {
            Lent::Buffer(bytes) => bytes,
            Lent::Piece(value, start, end) => octets(value).map_or(&[], |all| &all[*start..*end]),
            Lent::Pulled(value) => octets(value).unwrap_or(&[]),
        }
    }
}

impl Drop for Chunk<'_> {
    fn drop(&mut self) {
        if let Lent::Pulled(value) = self.lent {
            #[expect(
                unsafe_code,
                reason = "the pull gave this guard its own reference, and nothing \
                          borrows the bytes once the guard is dropped"
            )]
            unsafe {
                value.release();
            }
        }
    }
}

/// The octets of a `bytes` or `string` value.
fn octets(value: &Value) -> Option<&[u8]> {
    value.as_bytes().or_else(|| value.as_str_bytes())
}

impl ByteSource {
    /// The source `value` is, and the value every later read is handed, with
    /// a reference of its own for the member to keep.
    ///
    /// Nothing is read here. A closed file is refused now, where the program
    /// can still see which call was wrong.
    ///
    /// # Errors
    ///
    /// A `RuntimeError` for a closed file, an `IOError` for a descriptor the
    /// operating system would not copy, and whatever an `Iterable`'s
    /// `iterate()` throws.
    pub(crate) fn from_arg(
        ctx: &mut Ctx,
        value: Value,
        member: &'static str,
    ) -> Result<(Self, Value), Fault> {
        let kind = if crate::instance::is_instance(value, &crate::io::FILE) {
            let (key, path) = crate::io::handle_of(value, member)?;
            let path = path.as_text().unwrap_or("?").to_owned();
            // A write-only handle throws here, by its mode, so the error is
            // one `LogicError` on every platform and comes before any read.
            let mode = crate::io::mode_of(value, member)?;
            if matches!(mode, Access::Write | Access::Append) {
                return Err(crate::io::write_only(member, mode, &path));
            }
            let open = ctx
                .open_file_mut(key)
                .ok_or_else(|| closed(member, &path))?;
            let file = open
                .try_clone()
                .map_err(|err| failed(member, &path, &err))?;
            let seekable = nvs_runtime::os::can_seek(&file);
            Kind::File(FileSource {
                key,
                path,
                file,
                seekable,
                buffer: Vec::new(),
                filled: false,
                origin: None,
                started: false,
                kept: Vec::new(),
                kept_at: 0,
                kept_whole: false,
            })
        } else if octets(&value).is_some() {
            Kind::Bytes { at: 0 }
        } else if crate::instance::is_instance(value, &crate::request::PART) {
            // A part the walk has passed throws here, where it was given.
            crate::request::part_current(ctx, value, member)?;
            Kind::Part { buffer: Vec::new() }
        } else if crate::instance::is_instance(value, &crate::request::BODY_STREAM) {
            // The stream is its own cursor, so `start` gives the stream back.
            let cursor = nvs_runtime::sequence::start(ctx, value, member)?;
            return Ok((
                Self {
                    member,
                    kind: Kind::Stream { from: 0 },
                    ended: false,
                },
                cursor,
            ));
        } else {
            let cursor = nvs_runtime::sequence::start(ctx, value, member)?;
            return Ok((
                Self {
                    member,
                    kind: Kind::Chunks { from: 0 },
                    ended: false,
                },
                cursor,
            ));
        };
        #[expect(
            unsafe_code,
            reason = "the argument is borrowed for this call, and the member keeps \
                      the reference this takes"
        )]
        unsafe {
            value.retain();
        }
        Ok((
            Self {
                member,
                kind,
                ended: false,
            },
            value,
        ))
    }

    /// The path of the file this source reads, for a message. `None` for
    /// `bytes` and for a walk of chunks.
    #[must_use]
    pub(crate) fn path(&self) -> Option<&str> {
        match &self.kind {
            Kind::File(file) => Some(&file.path),
            Kind::Bytes { .. } | Kind::Chunks { .. } | Kind::Part { .. } | Kind::Stream { .. } => {
                None
            }
        }
    }

    /// The next chunk, or `None` at the end. `holder` is the value
    /// [`Self::from_arg`] gave.
    ///
    /// # Errors
    ///
    /// A `RuntimeError` for a file closed under the read, an `IOError` for a
    /// read the operating system refused, whatever a driven `Iterable` throws,
    /// and a fatal for a chunk that is not `bytes` or `string`, which the
    /// checker does not let through.
    pub(crate) fn next_chunk(
        &mut self,
        ctx: &mut Ctx,
        holder: Value,
    ) -> Result<Option<Chunk<'_>>, Fault> {
        if self.ended {
            return Ok(None);
        }
        let member = self.member;
        let lent = match &mut self.kind {
            Kind::File(file) => file.next(ctx, member)?.map(Lent::Buffer),
            Kind::Bytes { at } => {
                let all = octets(&holder).map_or(0, <[u8]>::len);
                let start = (*at).min(all);
                let end = (start + READ_CHUNK).min(all);
                *at = end;
                (end > start).then_some(Lent::Piece(holder, start, end))
            }
            Kind::Part { buffer } => {
                if crate::request::part_pull(ctx, holder, member, buffer)? {
                    Some(Lent::Buffer(buffer))
                } else {
                    *buffer = Vec::new();
                    None
                }
            }
            Kind::Chunks { from } | Kind::Stream { from } => {
                match nvs_runtime::sequence::pull(ctx, holder, from, member)? {
                    None => None,
                    Some(chunk) if octets(&chunk).is_some() => Some(Lent::Pulled(chunk)),
                    Some(chunk) => {
                        // Unreachable from source: every forward member's
                        // parameter is `Core\Bytes::Chunks`, so a walk of
                        // anything but `bytes` is `E0401` at the call site.
                        // The guard releases the reference the pull gave.
                        drop(Chunk {
                            lent: Lent::Pulled(chunk),
                        });
                        return Err(Fault::fatal(format!(
                            "internal error: {member} was given a chunk that is not bytes"
                        )));
                    }
                }
            }
        };
        self.ended = lent.is_none();
        Ok(lent.map(|lent| Chunk { lent }))
    }

    /// Starts the forward read again at the first byte, and returns whether it
    /// could. A file that can seek and a `bytes` value can be read again,
    /// because their bytes are still there. A part, a body stream, a walk of
    /// chunks and a file that cannot seek are read once, so this returns
    /// `false` and changes nothing.
    ///
    /// A member that reads its source twice, such as
    /// `Core\Markdown::toHtmlEach`, keeps a second read cheap with this: it
    /// costs no memory, where holding the source whole costs its length.
    pub(crate) fn restart(&mut self) -> bool {
        let restarted = match &mut self.kind {
            Kind::File(file) if file.seekable => {
                // The next forward read moves the file to its start again.
                file.started = false;
                true
            }
            Kind::Bytes { at } => {
                *at = 0;
                true
            }
            Kind::File(_) | Kind::Chunks { .. } | Kind::Part { .. } | Kind::Stream { .. } => false,
        };
        if restarted {
            self.ended = false;
        }
        restarted
    }

    /// Whether [`Self::restart`] would start the read again.
    pub(crate) fn can_restart(&self) -> bool {
        match &self.kind {
            Kind::File(file) => file.seekable,
            Kind::Bytes { .. } => true,
            Kind::Chunks { .. } | Kind::Part { .. } | Kind::Stream { .. } => false,
        }
    }

    /// The first `max` bytes, or fewer where the source is shorter. Nothing
    /// is consumed: a later forward read still starts at the first byte.
    ///
    /// # Errors
    ///
    /// [`Self::next_chunk`]'s for a file, the request's for a part or a body
    /// stream a forward read has started, and a fatal for a walk of chunks,
    /// which the checker does not let through.
    pub(crate) fn prefix(
        &mut self,
        ctx: &mut Ctx,
        holder: Value,
        max: usize,
    ) -> Result<Vec<u8>, Fault> {
        let member = self.member;
        match &mut self.kind {
            Kind::File(file) => {
                let max = nvs_runtime::affordable(Some(max), member)?;
                if file.seekable {
                    return file.read_at(ctx, member, 0, max);
                }
                file.keep(ctx, member, max)?;
                Ok(file.kept[..max.min(file.kept.len())].to_vec())
            }
            Kind::Bytes { .. } => Ok(range_of(&holder, 0, max)),
            Kind::Part { .. } => crate::request::part_prefix(ctx, holder, max, member),
            Kind::Stream { .. } => crate::request::body_stream_prefix(ctx, holder, max, member),
            Kind::Chunks { .. } => Err(no_random_read(member)),
        }
    }

    /// At most `len` bytes from `offset`, fewer where the source ends first.
    ///
    /// # Errors
    ///
    /// [`Self::prefix`]'s, and the memory limit's fatal for a read the request
    /// cannot hold, checked before each [`READ_CHUNK`].
    pub(crate) fn range(
        &mut self,
        ctx: &mut Ctx,
        holder: Value,
        offset: u64,
        len: u64,
    ) -> Result<Vec<u8>, Fault> {
        let member = self.member;
        let len = usize::try_from(len).unwrap_or(usize::MAX);
        match &mut self.kind {
            Kind::File(file) if file.seekable => file.read_at(ctx, member, offset, len),
            Kind::File(file) => {
                file.keep(ctx, member, usize::MAX)?;
                Ok(slice_of(&file.kept, offset, len, member)?.to_vec())
            }
            Kind::Bytes { .. } => {
                let start = usize::try_from(offset).unwrap_or(usize::MAX);
                let all = octets(&holder).map_or(0, <[u8]>::len);
                nvs_runtime::affordable(Some(len.min(all.saturating_sub(start))), member)?;
                Ok(range_of(&holder, start, len))
            }
            Kind::Part { .. } => crate::request::part_whole(ctx, holder, member, |all| {
                window(all, offset, len, member)
            })?,
            Kind::Stream { .. } => crate::request::body_stream_whole(ctx, holder, member, |all| {
                window(all, offset, len, member)
            })?,
            Kind::Chunks { .. } => Err(no_random_read(member)),
        }
    }

    /// The source's length in bytes. A file that cannot seek is read whole to
    /// learn it, and kept.
    ///
    /// # Errors
    ///
    /// [`Self::range`]'s.
    pub(crate) fn size(&mut self, ctx: &mut Ctx, holder: Value) -> Result<u64, Fault> {
        let member = self.member;
        match &mut self.kind {
            Kind::File(file) => {
                file.check_open(ctx, member)?;
                if file.seekable {
                    return file
                        .file
                        .metadata()
                        .map(|meta| meta.len())
                        .map_err(|err| failed(member, &file.path, &err));
                }
                file.keep(ctx, member, usize::MAX)?;
                Ok(file.kept.len() as u64)
            }
            Kind::Bytes { .. } => Ok(octets(&holder).map_or(0, <[u8]>::len) as u64),
            Kind::Part { .. } => {
                crate::request::part_whole(ctx, holder, member, |all| all.len() as u64)
            }
            Kind::Stream { .. } => {
                crate::request::body_stream_whole(ctx, holder, member, |all| all.len() as u64)
            }
            Kind::Chunks { .. } => Err(no_random_read(member)),
        }
    }

    /// Puts a file's position back where a prefix or random read found it.
    ///
    /// # Errors
    ///
    /// The `IOError` the seek back failed with.
    pub(crate) fn finish(&mut self) -> Result<(), Fault> {
        let member = self.member;
        if let Kind::File(file) = &mut self.kind
            && let Some(origin) = file.origin.take()
        {
            file.file
                .seek(SeekFrom::Start(origin))
                .map_err(|err| failed(member, &file.path, &err))?;
        }
        Ok(())
    }
}

impl Drop for ByteSource {
    fn drop(&mut self) {
        if let Kind::File(file) = &mut self.kind
            && let Some(origin) = file.origin.take()
        {
            // The position is only the program's convenience, and a drop has
            // nobody to tell, so a failed seek back changes nothing else.
            let _ = file.file.seek(SeekFrom::Start(origin));
        }
    }
}

/// Reads `value` forward to its end and hands `each` one chunk at a time, for
/// a member that consumes its source in one call (`Core\Hash::of`,
/// `Core\Storage::put`). A file's position is left at its end.
///
/// **What it spends:** what [`ByteSource`] holds, and nothing per chunk.
///
/// # Errors
///
/// [`ByteSource::from_arg`]'s and [`ByteSource::next_chunk`]'s, and whatever
/// `each` returns, which stops the read.
pub(crate) fn for_each_chunk(
    ctx: &mut Ctx,
    value: Value,
    member: &'static str,
    mut each: impl FnMut(&mut Ctx, &[u8]) -> Result<(), Fault>,
) -> Result<(), Fault> {
    let (mut source, holder) = ByteSource::from_arg(ctx, value, member)?;
    let read = (|| {
        while let Some(chunk) = source.next_chunk(ctx, holder)? {
            each(ctx, chunk.bytes())?;
        }
        Ok(())
    })();
    let finished = source.finish();
    drop(source);
    #[expect(
        unsafe_code,
        reason = "`from_arg` gave this call a reference of its own, and nothing \
                  keeps it past the read"
    )]
    unsafe {
        holder.release();
    }
    read?;
    finished
}

/// A source read forward as an [`std::io::Read`], for a codec that pulls its
/// input itself (`Core\Compress`). It counts what the codec has taken, so a
/// decode can bound its output by the input read so far.
///
/// **A failed read ends the input**, and the fault is kept for
/// [`Reader::close`], because a codec flattens an error into its own message
/// and the zstd encoder panics on one. The request's deadline ends the input
/// the same way, and the caller polls it once the codec returns.
///
/// **What it spends:** what [`ByteSource`] holds, and one copy of the chunk
/// being read.
pub(crate) struct Reader<'c> {
    /// The source.
    source: ByteSource,
    /// The value every read is handed, released on drop.
    holder: Value,
    /// The request, asked before each chunk.
    ctx: &'c mut Ctx,
    /// The chunk being read, and how much of it the codec has taken.
    pending: Vec<u8>,
    /// How much of [`Reader::pending`] is read.
    at: usize,
    /// How many bytes the codec has taken.
    consumed: std::rc::Rc<std::cell::Cell<u64>>,
    /// The fault a read stopped at.
    fault: Option<Fault>,
}

impl<'c> Reader<'c> {
    /// A forward read of `value`, which `member` names in a message.
    ///
    /// # Errors
    ///
    /// [`ByteSource::from_arg`]'s.
    pub(crate) fn open(
        ctx: &'c mut Ctx,
        value: Value,
        member: &'static str,
    ) -> Result<Self, Fault> {
        let (source, holder) = ByteSource::from_arg(ctx, value, member)?;
        Ok(Self {
            source,
            holder,
            ctx,
            pending: Vec::new(),
            at: 0,
            consumed: std::rc::Rc::default(),
            fault: None,
        })
    }

    /// The count of bytes the codec has taken, which stays readable while the
    /// codec owns the reader.
    pub(crate) fn counter(&self) -> std::rc::Rc<std::cell::Cell<u64>> {
        std::rc::Rc::clone(&self.consumed)
    }

    /// Ends the read: the fault a read stopped at, or the file's error.
    ///
    /// # Errors
    ///
    /// The fault a read stopped at, which is truer than the codec's own
    /// message, and [`ByteSource::finish`]'s.
    pub(crate) fn close(mut self) -> Result<(), Fault> {
        if let Some(fault) = self.fault.take() {
            return Err(fault);
        }
        self.source.finish()
    }
}

impl Read for Reader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.fault.is_some() || buf.is_empty() {
            return Ok(0);
        }
        if self.at == self.pending.len() {
            if self.ctx.deadline_expired() {
                return Ok(0);
            }
            match self.source.next_chunk(self.ctx, self.holder) {
                Ok(Some(chunk)) => {
                    self.pending.clear();
                    self.pending.extend_from_slice(chunk.bytes());
                    self.at = 0;
                }
                Ok(None) => return Ok(0),
                Err(fault) => {
                    self.fault = Some(fault);
                    return Ok(0);
                }
            }
        }
        let n = buf.len().min(self.pending.len() - self.at);
        buf[..n].copy_from_slice(&self.pending[self.at..self.at + n]);
        self.at += n;
        self.consumed.set(self.consumed.get() + n as u64);
        Ok(n)
    }
}

impl Drop for Reader<'_> {
    fn drop(&mut self) {
        #[expect(
            unsafe_code,
            reason = "`from_arg` gave the reader a reference of its own, and \
                      nothing keeps it past the reader"
        )]
        unsafe {
            self.holder.release();
        }
    }
}

impl FileSource {
    /// Throws where the program has closed its handle.
    fn check_open(&self, ctx: &mut Ctx, member: &str) -> Result<(), Fault> {
        if ctx.open_file_mut(self.key).is_none() {
            return Err(closed(member, &self.path));
        }
        Ok(())
    }

    /// The next forward chunk: the kept bytes first, then the file.
    fn next(&mut self, ctx: &mut Ctx, member: &str) -> Result<Option<&[u8]>, Fault> {
        self.check_open(ctx, member)?;
        if self.kept_at < self.kept.len() {
            let start = self.kept_at;
            let end = (start + READ_CHUNK).min(self.kept.len());
            self.kept_at = end;
            return Ok(Some(&self.kept[start..end]));
        }
        if self.kept_whole {
            return Ok(None);
        }
        if !self.started {
            self.started = true;
            // A forward read reads the whole file and leaves the position at
            // its end, so a prefix read before it has nothing to put back.
            self.origin = None;
            if self.seekable {
                self.file
                    .seek(SeekFrom::Start(0))
                    .map_err(|err| failed(member, &self.path, &err))?;
            }
        }
        if self.buffer.is_empty() {
            // The one extra byte finds the end of a file that did not change
            // in the same read as its last byte.
            let size = self.seekable.then(|| self.file.metadata().ok()).flatten();
            let want = size
                .and_then(|m| usize::try_from(m.len()).ok())
                .map_or(READ_CHUNK, |n| n.saturating_add(1).min(READ_CHUNK));
            self.buffer = vec![0_u8; want];
        } else if self.filled {
            self.buffer = vec![0_u8; READ_CHUNK];
        }
        let read = self
            .file
            .read(&mut self.buffer)
            .map_err(|err| failed(member, &self.path, &err))?;
        self.filled = read == self.buffer.len() && read < READ_CHUNK;
        if read == 0 {
            // The end: the source holds no buffer once it is read.
            self.buffer = Vec::new();
            self.kept = Vec::new();
            return Ok(None);
        }
        Ok(Some(&self.buffer[..read]))
    }

    /// At most `len` bytes at `offset` of a seekable file, read in
    /// [`READ_CHUNK`] pieces with the memory limit checked before each. The
    /// position the program had is kept for [`ByteSource::finish`].
    fn read_at(
        &mut self,
        ctx: &mut Ctx,
        member: &str,
        offset: u64,
        len: usize,
    ) -> Result<Vec<u8>, Fault> {
        self.check_open(ctx, member)?;
        let failed = |err: &std::io::Error| failed(member, &self.path, err);
        if self.origin.is_none() {
            self.origin = Some(self.file.stream_position().map_err(|err| failed(&err))?);
        }
        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|err| failed(&err))?;
        let mut out = Vec::new();
        while out.len() < len {
            let piece = (len - out.len()).min(READ_CHUNK);
            nvs_runtime::affordable(Some(piece), member)?;
            let read = (&self.file)
                .take(piece as u64)
                .read_to_end(&mut out)
                .map_err(|err| failed(&err))?;
            if read < piece {
                break;
            }
        }
        Ok(out)
    }

    /// Reads a file that cannot seek forward into [`FileSource::kept`] until
    /// it has `want` bytes or the file ends, with the memory limit checked
    /// before each [`READ_CHUNK`].
    fn keep(&mut self, ctx: &mut Ctx, member: &str, want: usize) -> Result<(), Fault> {
        self.check_open(ctx, member)?;
        if self.started && !self.kept_whole && self.kept_at >= self.kept.len() {
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                format!(
                    "{member}: {} cannot seek, and it has already been read from its start",
                    self.path
                ),
            ));
        }
        while self.kept.len() < want && !self.kept_whole {
            let piece = (want - self.kept.len()).min(READ_CHUNK);
            nvs_runtime::affordable(Some(piece), member)?;
            let read = (&self.file)
                .take(piece as u64)
                .read_to_end(&mut self.kept)
                .map_err(|err| failed(member, &self.path, &err))?;
            if read < piece {
                self.kept_whole = true;
            }
        }
        Ok(())
    }
}

/// Every byte `input` gives, read no further than the request can hold.
///
/// The one reader of a whole source in this crate: `Core\IO::read`, its text
/// and line forms, `Core\IO::stdin` and `Core\Storage::get` all read through
/// it, and `no_member_reads_a_whole_source_outside_the_bounded_reader` fails
/// on a `read_to_end` or `read_to_string` anywhere else that is not behind a
/// `take` (`rule:errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself`).
///
/// [`nvs_runtime::budget::ByteWriter`] asks the request's memory ceiling
/// before each growth, and stops at [`nvs_runtime::Ctx::intake_limit`]. So a
/// source of any size costs at most what the request may hold, and the read
/// stops there. Once the bytes are read, the one copy every caller makes of
/// them, into a `string`, `bytes` or lines, is asked for too. A copy that does
/// not fit is then refused before it is made. **What it spends:** one compare
/// per growth of the buffer.
///
/// # Errors
///
/// [`nvs_runtime::capability::io_failure`]'s `IOError` when the read fails;
/// `intake_breach`'s `RuntimeError` for a source past `max_output`, which is a
/// refusal to hold it rather than a failure to read it; and the memory `FATAL`
/// once the request's ceiling refused a growth or the copy.
pub(crate) fn read_bounded(
    ctx: &Ctx,
    input: &mut impl Read,
    member: &str,
    path: &std::path::Path,
) -> Result<Vec<u8>, Fault> {
    use nvs_runtime::budget::{ByteWriter, Stopped};

    let limit = match ctx.intake_limit() {
        0 => usize::MAX,
        limit => limit,
    };
    let mut bytes = ByteWriter::within(limit);
    bytes
        .read_from(input)
        .map_err(|err| nvs_runtime::capability::io_failure(member, path, &err))?;
    let over_memory = || {
        ctx.memory_breach().unwrap_or_else(|| {
            Fault::fatal(format!(
                "{member}: what was read does not fit in the request's memory limit"
            ))
        })
    };
    match bytes.finish() {
        Ok(bytes) if nvs_runtime::budget::affords(bytes.len()) => Ok(bytes),
        Ok(_) | Err(Stopped::Refused) => Err(over_memory()),
        Err(Stopped::Over) => Err(ctx
            .intake_breach(member, limit.saturating_add(1))
            .unwrap_or_else(over_memory)),
    }
}

/// A copy of at most `len` bytes of `all` from `offset`, with the memory limit
/// checked before the copy.
fn window(all: &[u8], offset: u64, len: usize, member: &str) -> Result<Vec<u8>, Fault> {
    let start = usize::try_from(offset).unwrap_or(usize::MAX).min(all.len());
    let piece = &all[start..start.saturating_add(len).min(all.len())];
    nvs_runtime::affordable(Some(piece.len()), member)?;
    Ok(piece.to_vec())
}

/// A copy of at most `len` bytes of `value` from `start`.
fn range_of(value: &Value, start: usize, len: usize) -> Vec<u8> {
    let all = octets(value).unwrap_or(&[]);
    let start = start.min(all.len());
    all[start..start.saturating_add(len).min(all.len())].to_vec()
}

/// At most `len` bytes of `kept` from `offset`.
fn slice_of<'k>(kept: &'k [u8], offset: u64, len: usize, member: &str) -> Result<&'k [u8], Fault> {
    let start = usize::try_from(offset)
        .unwrap_or(usize::MAX)
        .min(kept.len());
    let end = start.saturating_add(len).min(kept.len());
    nvs_runtime::affordable(Some(end - start), member)?;
    Ok(&kept[start..end])
}

/// The `RuntimeError` for a file the program has closed.
fn closed(member: &str, path: &str) -> Fault {
    Fault::thrown(format!("{member}: this handle is closed — {path}"))
}

/// The `IOError` for a read or a seek the operating system refused.
fn failed(member: &str, path: &str, err: &std::io::Error) -> Fault {
    nvs_runtime::capability::io_failure(member, std::path::Path::new(path), err)
}

/// What a message of a guest's read of a lent source names.
const LENT_MEMBER: &str = "an extension's read of its source";

/// The size of `value` when it is a source `nvs_cli`'s extension host lends to a
/// guest as a source of byte ranges: an open `Core\IO\File`, a
/// `Core\Request\Part` or a `Core\Request\BodyStream`
/// (`rule:core-classes/spreadsheet-reads-as-a-stream`). `None` for any other
/// value. The host serves a `bytes` value from its own string.
///
/// Each read is one [`ByteSource`] of its own, made and finished in the call,
/// because a lent value is valid only for the call that lends it. A part or a
/// body is held whole on the request by the first read, under
/// `[limits] request_body`, so the later reads cost no wire read.
///
/// # Errors
///
/// [`ByteSource::from_arg`]'s and [`ByteSource::size`]'s.
pub fn extension_source_size(ctx: &mut Ctx, value: Value) -> Option<Result<u64, Fault>> {
    let lendable = crate::instance::is_instance(value, &crate::io::FILE)
        || crate::instance::is_instance(value, &crate::request::PART)
        || crate::instance::is_instance(value, &crate::request::BODY_STREAM);
    lendable.then(|| lent_read(ctx, value, |source, ctx, holder| source.size(ctx, holder)))
}

/// At most `length` bytes at `offset` of the source [`extension_source_size`]
/// lent, fewer where it ends first. A file's position is where it was before.
///
/// # Errors
///
/// [`ByteSource::from_arg`]'s and [`ByteSource::range`]'s, such as the memory
/// limit's for a part larger than the request may hold.
pub fn extension_source_range(
    ctx: &mut Ctx,
    value: Value,
    offset: u64,
    length: u64,
) -> Result<Vec<u8>, Fault> {
    lent_read(ctx, value, |source, ctx, holder| {
        source.range(ctx, holder, offset, length)
    })
}

/// `read` over a [`ByteSource`] of `value`, which is then finished and freed,
/// with the reference [`ByteSource::from_arg`] took released.
fn lent_read<T>(
    ctx: &mut Ctx,
    value: Value,
    read: impl FnOnce(&mut ByteSource, &mut Ctx, Value) -> Result<T, Fault>,
) -> Result<T, Fault> {
    let (mut source, holder) = ByteSource::from_arg(ctx, value, LENT_MEMBER)?;
    let read = read(&mut source, ctx, holder);
    let finished = source.finish();
    drop(source);
    #[expect(
        unsafe_code,
        reason = "`from_arg` gave this call a reference of its own, and nothing \
                  keeps it past the read"
    )]
    unsafe {
        holder.release();
    }
    let read = read?;
    finished?;
    Ok(read)
}

/// The fatal for a prefix or random read of a walk of chunks, which the
/// checker does not let through: a prefix read would consume it, and a random
/// read cannot go back.
fn no_random_read(member: &str) -> Fault {
    // Unreachable from source: a prefix or random member takes
    // `Core\Bytes::Source`, so a walk of chunks given to one is `E0401`.
    Fault::fatal(format!(
        "internal error: {member} read a walk of chunks out of order"
    ))
}

#[cfg(test)]
mod tests {
    use std::io::{Seek, SeekFrom, Write};

    use nvs_runtime::{Ctx, NvsArray, NvsStr, OutputSink, Value};

    use super::{ByteSource, READ_CHUNK};

    /// Frees the one reference this frame owns.
    fn released(value: Value) {
        #[expect(unsafe_code, reason = "the case owns the one reference it releases")]
        unsafe {
            value.release();
        }
    }

    /// A scratch file holding `content`.
    fn scratch(name: &str, content: &[u8]) -> (nvs_repo::Scratch, std::path::PathBuf) {
        let dir = nvs_repo::scratch("byte-source");
        let path = dir.join(name);
        std::fs::write(&path, content).expect("the case writes its file");
        (dir, path)
    }

    /// A `Core\IO\File` over `file`, filed on `ctx` as `Core\IO::open` files
    /// one, and its key.
    fn handle(ctx: &mut Ctx, file: std::fs::File, path: &str) -> (Value, u64) {
        let key = ctx.hold_open_file(file);
        let opened = crate::io::file_handle(
            key,
            Value::text(path),
            nvs_runtime::capability::Access::ReadWrite,
        );
        (opened, key)
    }

    /// Every chunk of a forward read, joined, and how many chunks there were.
    fn drained(ctx: &mut Ctx, source: &mut ByteSource, holder: Value) -> (Vec<u8>, usize) {
        let (mut all, mut count) = (Vec::new(), 0);
        while let Some(chunk) = source.next_chunk(ctx, holder).expect("the source reads") {
            all.extend_from_slice(chunk.bytes());
            count += 1;
        }
        (all, count)
    }

    /// `len` bytes counting up to `modulo` and starting again, so a byte read
    /// from the wrong offset shows.
    fn pattern(len: usize, modulo: u8) -> Vec<u8> {
        (0..len)
            .map(|at| u8::try_from(at % usize::from(modulo)).expect("a byte under the modulo"))
            .collect()
    }

    /// The position of the program's own descriptor.
    fn position(ctx: &mut Ctx, key: u64) -> u64 {
        ctx.open_file_mut(key)
            .expect("the handle is open")
            .stream_position()
            .expect("a regular file has a position")
    }

    /// A file is read from its first byte even where the program has moved
    /// its position, and every chunk comes out of the one buffer.
    #[test]
    fn next_chunk_reads_a_file_from_its_start_in_one_reused_buffer() {
        let content = pattern(READ_CHUNK * 2 + 7, 251);
        let (_dir, path) = scratch("forward.bin", &content);
        let mut ctx = Ctx::new(OutputSink::Sink);
        let mut file = std::fs::File::open(&path).expect("the case opens its file");
        file.seek(SeekFrom::Start(5))
            .expect("the case moves the position");
        let (value, key) = handle(&mut ctx, file, "forward.bin");

        let (mut source, holder) =
            ByteSource::from_arg(&mut ctx, value, "Test::read").expect("an open file is a source");
        let mut buffers = Vec::new();
        let mut all = Vec::new();
        while let Some(chunk) = source.next_chunk(&mut ctx, holder).expect("the file reads") {
            buffers.push(chunk.bytes().as_ptr());
            all.extend_from_slice(chunk.bytes());
        }
        assert_eq!(all, content, "the whole file, from offset 0");
        // Miri returns short reads from a file on purpose, so it may take more
        // chunks than a native read of the same file.
        if !cfg!(miri) {
            assert_eq!(buffers.len(), 3);
        }
        assert!(
            buffers.iter().all(|at| *at == buffers[0]),
            "every chunk is lent from one buffer"
        );
        assert_eq!(
            position(&mut ctx, key),
            content.len() as u64,
            "a forward read ends at the end"
        );
        assert!(
            source
                .next_chunk(&mut ctx, holder)
                .expect("the end")
                .is_none()
        );
        drop(source);
        released(holder);
        released(value);
    }

    /// A small file is read into a buffer of its size plus one byte, and a
    /// file that grows after its first read is still read to its new end.
    #[test]
    fn next_chunk_sizes_its_buffer_to_a_small_file_and_reads_one_that_grows() {
        let content = pattern(360, 251);
        let (_dir, path) = scratch("small.bin", &content);
        let mut ctx = Ctx::new(OutputSink::Sink);
        let file = std::fs::File::open(&path).expect("the case opens its file");
        let (value, _key) = handle(&mut ctx, file, "small.bin");

        let (mut source, holder) =
            ByteSource::from_arg(&mut ctx, value, "Test::read").expect("an open file is a source");
        let first = source
            .next_chunk(&mut ctx, holder)
            .expect("the file reads")
            .expect("a first chunk")
            .bytes()
            .to_vec();
        // Miri returns short reads from a file on purpose, so its first chunk
        // may be a part of the file, and the rest comes before the growth.
        let short = cfg!(miri) && first.len() < content.len();
        if !short {
            assert_eq!(first, content, "the whole small file in one chunk");
        }
        let super::Kind::File(file) = &source.kind else {
            panic!("a file source")
        };
        assert_eq!(
            file.buffer.len(),
            content.len() + 1,
            "the buffer is the file's size plus one"
        );

        let extra = pattern(1000, 7);
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .and_then(|mut f| f.write_all(&extra))
            .expect("the case grows its file");
        let (rest, _) = drained(&mut ctx, &mut source, holder);
        if short {
            assert_eq!(
                [first, rest].concat(),
                [content, extra].concat(),
                "the whole file, then the bytes written after the first read"
            );
        } else {
            assert_eq!(rest, extra, "the bytes written after the first read");
        }
        drop(source);
        released(holder);
        released(value);
    }

    /// An array of chunks and a `bytes` value give the same bytes a file
    /// does, and a pulled chunk is released by its guard.
    #[test]
    fn next_chunk_reads_chunks_and_bytes_alike() {
        let content = pattern(READ_CHUNK + 300, 13);
        let mut ctx = Ctx::new(OutputSink::Sink);

        let mut array = NvsArray::new();
        for piece in content.chunks(100) {
            array.append(Value::bytes(NvsStr::new(piece)));
        }
        let chunks = Value::array(array);
        let (mut source, holder) =
            ByteSource::from_arg(&mut ctx, chunks, "Test::read").expect("an array is a source");
        let (all, count) = drained(&mut ctx, &mut source, holder);
        assert_eq!(all, content);
        assert_eq!(count, content.len().div_ceil(100), "one chunk per element");
        drop(source);
        released(holder);
        released(chunks);

        let bytes = Value::bytes(NvsStr::new(&content));
        let (mut source, holder) =
            ByteSource::from_arg(&mut ctx, bytes, "Test::read").expect("bytes are a source");
        let (all, count) = drained(&mut ctx, &mut source, holder);
        assert_eq!(all, content);
        assert_eq!(count, 2, "bytes are lent in pieces of one read");
        assert_eq!(
            source
                .prefix(&mut ctx, holder, 4)
                .expect("bytes have a start"),
            content[..4]
        );
        drop(source);
        released(holder);
        released(bytes);
    }

    /// A random read moves the position, and `finish` puts it back where the
    /// program had it.
    #[test]
    fn range_finish_puts_the_file_position_back() {
        let content = pattern(4_000, 7);
        let (_dir, path) = scratch("range.bin", &content);
        let mut ctx = Ctx::new(OutputSink::Sink);
        let mut file = std::fs::File::open(&path).expect("the case opens its file");
        file.seek(SeekFrom::Start(9))
            .expect("the case moves the position");
        let (value, key) = handle(&mut ctx, file, "range.bin");

        let (mut source, holder) =
            ByteSource::from_arg(&mut ctx, value, "Test::read").expect("an open file is a source");
        assert_eq!(
            source
                .range(&mut ctx, holder, 1_000, 10)
                .expect("the range reads"),
            content[1_000..1_010]
        );
        assert_eq!(
            source.prefix(&mut ctx, holder, 3).expect("the start reads"),
            content[..3]
        );
        assert_eq!(
            source
                .range(&mut ctx, holder, 3_995, 100)
                .expect("a short range reads"),
            content[3_995..],
            "a range past the end is short"
        );
        assert_eq!(source.size(&mut ctx, holder).expect("the size"), 4_000);
        source.finish().expect("the position goes back");
        assert_eq!(position(&mut ctx, key), 9);
        drop(source);
        released(holder);
        released(value);
    }

    /// A random read that fails at the memory limit never reaches `finish`,
    /// and dropping the source still puts the position back.
    #[test]
    fn range_puts_the_position_back_after_a_failed_read() {
        let size = READ_CHUNK * 8;
        let (_dir, path) = scratch("failed.bin", &vec![7_u8; size]);
        let mut ctx = Ctx::new(OutputSink::Sink);
        let mut file = std::fs::File::open(&path).expect("the case opens its file");
        file.seek(SeekFrom::Start(3))
            .expect("the case moves the position");
        let (value, key) = handle(&mut ctx, file, "failed.bin");
        let (mut source, holder) =
            ByteSource::from_arg(&mut ctx, value, "Test::read").expect("an open file is a source");

        ctx.set_memory_limit(READ_CHUNK * 2);
        let failed = source.range(&mut ctx, holder, 0, size as u64);
        ctx.clear_memory_ceiling();
        assert!(failed.is_err(), "the read is past the limit");
        drop(source);
        assert_eq!(
            position(&mut ctx, key),
            3,
            "the drop puts the position back"
        );
        released(holder);
        released(value);
    }

    /// A random read larger than the request's memory stops at the limit,
    /// with no more than one piece past it.
    #[test]
    fn a_range_read_stops_at_the_memory_limit() {
        let size = READ_CHUNK * 32;
        let (_dir, path) = scratch("large.bin", &vec![1_u8; size]);
        let limit = READ_CHUNK * 4;
        let mut ctx = Ctx::new(OutputSink::Sink);
        let file = std::fs::File::open(&path).expect("the case opens its file");
        let (value, _) = handle(&mut ctx, file, "large.bin");
        let (mut source, holder) =
            ByteSource::from_arg(&mut ctx, value, "Test::read").expect("an open file is a source");

        // The limit and the peak count from here, so they measure the read
        // alone. Building the handle, the context's first object, takes about
        // 156 KiB whatever READ_CHUNK is: more than this whole bound under
        // Miri's 2 KiB chunk, and nothing the read itself holds.
        ctx.rebase_memory();
        ctx.set_memory_limit(limit);
        let failed = source.range(&mut ctx, holder, 0, size as u64);
        let peak = ctx.memory_peak();
        ctx.clear_memory_ceiling();
        let fault = failed.expect_err("the read is past the limit");
        assert!(format!("{fault:?}").contains("memory limit"), "{fault:?}");
        // The read holds at most the limit and one piece past it, and growing
        // its buffer briefly holds the old and the new copy together.
        assert!(
            peak < 3 * limit,
            "the read stopped at the limit: peak {peak}"
        );
        drop(source);
        released(holder);
        released(value);
    }

    /// A pipe cannot seek. Its start is kept for a prefix read, a longer
    /// prefix read reads further, and a forward read gives the kept bytes and
    /// then the rest.
    #[test]
    #[cfg_attr(
        miri,
        ignore = "opens a pipe, which Miri does not emulate on every target"
    )]
    fn a_file_that_cannot_seek_is_read_forward_and_buffered_for_its_start() {
        let content = pattern(READ_CHUNK + 50, 31);
        let (reader, mut writer) = std::io::pipe().expect("the case opens a pipe");
        // A thread writes, since a pipe holds only a few KiB, and dropping the
        // writer at its end is the end of the file.
        let writing = {
            let all = content.clone();
            std::thread::spawn(move || writer.write_all(&all))
        };
        #[cfg(unix)]
        let file = std::fs::File::from(std::os::fd::OwnedFd::from(reader));
        #[cfg(windows)]
        let file = std::fs::File::from(std::os::windows::io::OwnedHandle::from(reader));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let (value, _) = handle(&mut ctx, file, "pipe");
        let (mut source, holder) =
            ByteSource::from_arg(&mut ctx, value, "Test::read").expect("a pipe is a source");
        assert_eq!(
            source.prefix(&mut ctx, holder, 8).expect("the start"),
            content[..8]
        );
        assert_eq!(
            source.prefix(&mut ctx, holder, 16).expect("more of it"),
            content[..16]
        );
        let (all, _) = drained(&mut ctx, &mut source, holder);
        writing
            .join()
            .expect("the writer thread ends")
            .expect("the writer writes");
        assert_eq!(all, content, "the kept start, then the rest");
        drop(source);
        released(holder);
        released(value);
    }
}
