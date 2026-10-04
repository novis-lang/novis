A write door resolves its path once, checks the grant on that resolved path as it is written, and
opens it one level at a time from the root, each level from the handle of the one above it, following
no link. Checking a path and then opening it by name resolves it twice, and a folder replaced by a link
between the two is followed outside the grant. A resolved path names no link, so a link the walk meets
was planted after the check, and the write fails with an `IOError` and touches nothing beneath it.

The doors are `capability::create`, `capability::write`, `capability::create_dir`, the writing
modes of `capability::open`, the destination of `capability::copy` and both ends of
`capability::rename`, and `Core\IO`'s writes, `Core\Storage::put` and `Core\Zip::extract` reach the
filesystem through them. A rename resolves the folder above each end and keeps the last name as
written, because it moves or replaces that name: a link there is the entry renamed, never its target,
and the grant is checked on the entry. A missing folder is created by the same walk, one level at a time. On Unix a level is `openat` or
`mkdirat` with `O_NOFOLLOW`; on Windows it is `NtCreateFile` with the folder above as its root, and a
level whose reparse tag is a name surrogate — a symbolic link or a junction — is refused. A reparse
point that is not a name surrogate, such as a cloud placeholder, is a real folder and is walked.

A link that already exists when the door is called is unchanged in meaning: the resolution follows it,
the grant is checked on its target, and the walk opens the target. Only a link that appears between the
check and the open is refused.
