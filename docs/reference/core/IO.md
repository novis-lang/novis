---
summary: whole-file read and write on paths the configuration has granted
keywords: file, read a file, write a file, file system, fs.read, fs.write, capability, permission, nvs.toml, path
---

`Core\IO` reads or replaces a whole file as text. `readBytes` reads a file that is not text, such as
an image, as `bytes`. Every call first checks a capability. The path
must be inside a folder that `nvs.toml` grants as `fs.read` or `fs.write`. A read grant does not
allow writing. A path the configuration does not grant throws a `RuntimeError` that names the
capability. Your program can catch it and continue. A path the configuration allows but the
operating system does not throws an `IOError`. Examples are a missing file and a directory. The
check follows the path through its deepest folder that exists. Write a new file's path from such a
folder, as `./out.txt` and not `out.txt`. Otherwise the check finds it not granted.

When `nvs.toml` does not set `fs.read` or `fs.write`, a program started from the command line reads
and writes any path. Server work reads its application folder, except `nvs.toml` and names that
start with `.`. It writes only the temporary folders that `Core\IO::temporaryDir` made for it.

`stdinStream` reads the program's standard input in pieces of `bytes`, one piece per step of a
`foreach` loop.

```toml file=nvs.toml
[capabilities.fs]
read = ["."]
write = ["."]
```
```txt file=data.txt
alpha
beta
```
```nvs
<?nvs
string $text = Core\IO::read('data.txt');
echo Core\Str::length($text), " bytes\n";

Core\IO::write('./copy.txt', $text . "gamma\n");
echo Core\Str::trim(Core\IO::read('./copy.txt')), "\n";

try {
    Core\IO::read('./missing.txt');
} catch (IOError $io) {
    echo "missing: no such file\n";
}
try {
    Core\IO::read('../outside.txt');
} catch (RuntimeError $denied) {
    echo "outside: refused\n";
}
```
```output
11 bytes
alpha
beta
gamma
missing: no such file
outside: refused
```
