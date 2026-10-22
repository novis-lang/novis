A door that opens a file resolves its path once, checks the grant on that resolved path as it is
written, and opens it one level at a time from the root, each level from the handle of the one above
it, following no link. Checking a path and then opening it by name resolves it twice, and a folder
replaced by a link between the two is followed outside the grant. A resolved path names no link, so a
link the walk meets was planted after the check, and the call fails with an `IOError` and touches
nothing beneath it.

The write doors are `capability::create`, `capability::write`, `capability::create_dir`, the writing
modes of `capability::open`, the destination of `capability::copy` and both ends of
`capability::rename`, and `Core\IO`'s writes, `Core\Storage::put` and `Core\Zip::extract` reach the
filesystem through them. The read doors are `capability::open_read`, the reading mode of
`capability::open` and the source of `capability::copy`, and `Core\IO`'s reads, `Core\Storage::get`,
`Core\Response::sendFile` and a multipart file part sent by `Core\Http` reach the filesystem through
them. The removal doors are `capability::remove_file` and `capability::remove_dir`, and the doors
that read what a path is are `capability::metadata`, `capability::metadata_if_present`,
`capability::exists` and `capability::read_dir`. `Core\IO`'s removals, sizes, kinds, `exists`,
`list` and `walk`, `Core\Storage`'s `get`, `delete` and `list`, and `Core\Response::sendFile`'s
size reach the filesystem through them. An extension's `wasi:filesystem` checks a path against its
instance's preopens (`rule:security/extension-grants-are-an-intersection`) and then opens a file,
creates a folder, renames, removes, reads metadata or lists through the same walk, which
`nvs_runtime::guest_files` exposes to it with no grant check of its own. A read creates nothing, and a read-write handle checks `fs.read` and `fs.write` on the one resolution. A rename resolves the folder above each end and keeps the last name as
written, because it moves or replaces that name: a link there is the entry renamed, never its target,
and the grant is checked on the entry. A removal does the same with its one end, so a link there is
the entry removed. A stat and a listing open the entry itself, or the folder, beneath the folder above,
and read it from that handle. A link at a stat's last name led nowhere when the path was resolved, or
was planted after, so it is not followed and the stat finds nothing there. A listing reads each
entry's kind without following a link. A missing folder is created by the same walk, one level at a time. On Unix a level is `openat` or
`mkdirat` with `O_NOFOLLOW`; on Windows it is `NtCreateFile` with the folder above as its root, and a
level whose reparse tag is a name surrogate — a symbolic link or a junction — is refused. A reparse
point that is not a name surrogate, such as a cloud placeholder, is a real folder and is walked.

A link that already exists when the door is called is unchanged in meaning: the resolution follows it,
the grant is checked on its target, and the walk opens the target. Only a link that appears between the
check and the open is refused.

The grant is asked about the resolved path as written, never resolved again inside the check, because a
second resolution could see a different path than the one the walk opens. On Windows an overwrite
opens the existing file without emptying it, refuses it if it is a link, and only then sets its length
to zero, so a link is never emptied. A folder the process may search but not list is walked on Linux,
where a level is opened `O_PATH`; on every other platform it fails the walk. A volume mount point
inside a granted tree on Windows has a name-surrogate tag, so the walk refuses it unless the resolved
path is spelled through its volume.

The walk costs one open per level of the path, the count the operating system's own lookup makes, plus
one attribute read per level on Windows. A stat costs one more open than `stat(2)` would, for the
entry's own handle, and a listing one more for reopening the folder to read it. On Windows a listed
reparse point is opened once more to read its tag.

- **Memory:** nothing past the call for an open, a removal or a stat: every handle but the deepest is
  closed as soon as the next one is open. A listing is read whole, one name and one kind per entry,
  and is kept until the caller drops it, where `std::fs::read_dir` read it lazily. Every caller
  collected the names already.

**Declined:**

- **The walk for `Core\Zip::extract` alone.** Its per-folder comparison against the destination left
  the moment between the last comparison and the create, and did nothing for `Core\IO`'s own writes.
- **Keeping the window and stating it as a bound.** A folder swapped for a link would still send a
  write outside the grant.
- **`openat2` with `RESOLVE_BENEATH` on Linux.** It would be a third platform half for one kernel, and
  the walk already gives the same guarantee there.
- **Anchoring the walk at the granted root** instead of the filesystem root. It would refuse a link
  inside a granted tree that points to another place in the same tree, which a write may follow.
