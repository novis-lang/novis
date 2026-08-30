---
summary: path text taken apart and put together — basename, extension, join, normalize, relative paths — without touching the filesystem
keywords: basename, dirname, pathinfo, PATHINFO_EXTENSION, PATHINFO_BASENAME, realpath, DIRECTORY_SEPARATOR, explode(DIRECTORY_SEPARATOR), path, file extension, absolute path, relative path, normalize, ..
---

`Core\Path` is string algebra over path text: no member reads the disk, so nothing here needs a
capability, follows a symlink or checks that a file exists — that is `Core\IO`. Both `/` and `\`
are accepted on every platform, and a member that answers a path renders it with
`Core\Path::SEPARATOR` — `\` on Windows, `/` elsewhere. `join` only appends: a segment's own leading
separator or drive is dropped rather than allowed to replace the base. `normalize` resolves `.` and
`..` lexically; neither member makes an untrusted path safe, which is `Core\IO::within`. Absence
is `null`: `extension` on a name without one, `relativeTo` where no relative path exists.

```nvs
<?nvs
class Show {
    // A rendered path carries the platform's separator; print it as `/` either way.
    public static function slash(string $p): string {
        return Core\Str::replace($p, Core\Path::SEPARATOR, "/");
    }
}
string $p = "/var/www/html/index.php";
echo Core\Path::basename($p), " ", Core\Path::basename($p, {withoutExtension: true}), " ", Core\Path::extension($p) ?? "none", "\n";
echo Show::slash(Core\Path::dirname($p)), " ", Show::slash(Core\Path::dirname($p, {levels: 2})), "\n";
echo Show::slash(Core\Path::withExtension($p, "html")), "\n";
echo Show::slash(Core\Path::join("/var/www", "html", "/index.php")), "\n";
echo Show::slash(Core\Path::normalize("/var/www/../log/./app.log")), "\n";
echo Show::slash(Core\Path::relativeTo("/var/log/app.log", "/var/www") ?? "none"), "\n";
echo Core\Str::join(Core\Path::split("etc//hosts/"), ","), " ", Core\Path::isAbsolute("C:/tmp") ? "abs" : "rel", " ", Core\Path::isAbsolute("tmp") ? "abs" : "rel", "\n";
echo Core\Path::extension("Makefile") ?? "none", "\n";
```
```output
index.php index php
/var/www/html /var/www
/var/www/html/index.html
/var/www/html/index.php
/var/log/app.log
../log/app.log
etc,hosts abs rel
none
```
