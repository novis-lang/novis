---
summary: whole-file read and write on paths the configuration has granted
keywords: file_get_contents, file_put_contents, fopen, fread, fwrite, fs.read, fs.write, capability, nvs.toml, path
---

`Core\File` reads or replaces a whole file as text. Every call is a capability check first: the path
must fall under a root that `nvs.toml` grants as `fs.read` or `fs.write`, and a read grant is not a
write grant. A path the configuration does not grant throws a `RuntimeError` naming the capability,
and the program may catch it and carry on; a path the configuration allows but the operating system
refuses — no such file, a directory — throws an `IOError`. The check resolves the path through its
deepest existing directory, so a file that does not exist yet is spelled from one — `./out.txt`, not
`out.txt` — or the check refuses it as not granted. Without an `nvs.toml`, every call is refused.

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
string $text = Core\File::read("data.txt");
echo Core\Str::length($text), " bytes\n";

Core\File::write("./copy.txt", $text . "gamma\n");
echo Core\Str::trim(Core\File::read("./copy.txt")), "\n";

try {
    Core\File::read("./missing.txt");
} catch (IOError $io) {
    echo "missing: no such file\n";
}
try {
    Core\File::read("../outside.txt");
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
