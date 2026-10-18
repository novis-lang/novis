Every `Core` parameter that names a file works this way. Examples are the path methods of `Core\IO`,
`Core\Process::run`, `Core\Response::sendFile`, the script of `spawn script`, `Core\Socket::upgrade`
and `Core\Sse::upgrade`. The methods of `Core\Path` work on text, and their parameters are not file
paths.

`#[Core\Path]` is allowed on a `string` or `?string` parameter. On a parameter of any other type, the
program does not compile (`E0836`). A default value of such a parameter, such as `string $path =
'rates.json'`, starts at the folder of the file that declares the method.

A class constant is not joined to the folder, just like a variable. If it is a relative path, the
file method throws a `RuntimeError`. The error for `'data/' . $name` is `E0840`, and its message
shows how to start the path with the folder of the file.
