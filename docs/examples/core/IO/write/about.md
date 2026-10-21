Saves a text to a file.

`Core\IO::write` takes a path and a text. It writes the text to the file at that path. If the file
does not exist, `write` creates it. If the file exists, its old content is deleted first, so after
the call the file contains exactly the new text and nothing else.

To add to the end of a file and keep what is already there, use `Core\IO::append`.

`write` does not create folders. The folder of the file must already exist. `Core\IO::makeDir`
creates it.

`write` throws an `IOError` when the operating system cannot write the file: the folder does not
exist, the path is a folder, or the disk is read-only. So when the call returns, the text is really
in the file.

A program you start from the command line may write any path. On a server, a program may write
only in the folders that `Core\IO::temporaryDir` made for it. Any other path needs the `fs.write`
capability. Without it, `write` throws a `RuntimeError` before any file is created.
