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
them. A
read creates nothing, and a read-write handle checks `fs.read` and `fs.write` on the one resolution. A rename resolves the folder above each end and keeps the last name as
written, because it moves or replaces that name: a link there is the entry renamed, never its target,
and the grant is checked on the entry. A missing folder is created by the same walk, one level at a time. On Unix a level is `openat` or
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
one attribute read per level on Windows.

- **Memory:** nothing past the call: every handle but the deepest is closed as soon as the next one is
  open.

**Declined:**

- **The walk for `Core\Zip::extract` alone.** Its per-folder comparison against the destination left
  the moment between the last comparison and the create, and did nothing for `Core\IO`'s own writes.
- **Keeping the window and stating it as a bound.** A folder swapped for a link would still send a
  write outside the grant.
- **`openat2` with `RESOLVE_BENEATH` on Linux.** It would be a third platform half for one kernel, and
  the walk already gives the same guarantee there.
- **Anchoring the walk at the granted root** instead of the filesystem root. It would refuse a link
  inside a granted tree that points to another place in the same tree, which a write may follow.
