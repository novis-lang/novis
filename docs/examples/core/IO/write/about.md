Saves text or bytes to a file.

`Core\IO::write` takes a path and the content, which is a `string` or `bytes`. It writes the content
to the file exactly as it is. If the file does not exist, `write` creates it. If the file exists,
its old content is deleted first. After the call, the file contains only the new content.

Pass `bytes` for a file that is not text, such as an image or the result of
`Core\Serialize::encode`. `Core\IO::readBytes` reads it back. To add to the end of a file, use
`Core\IO::append`.

`write` does not create folders. `Core\IO::makeDir` creates them. `write` throws an `IOError` when
the operating system cannot write the file: the folder does not exist, the path is a folder, or the
disk is read-only. So when the call returns, the content is really in the file.

A program you start from the command line may write any path. On a server, a program may write
only in the folders that `Core\IO::temporaryDir` made for it. Any other path needs the `fs.write`
capability. Without it, `write` throws a `RuntimeError` before any file is created.
