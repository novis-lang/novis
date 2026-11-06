---
summary: every file and folder under a folder, which `Core\IO::walk` returns. You read the paths with a `foreach` loop
keywords: walk a folder, list files recursively, folder tree, directory tree, sub-folders, find files, recursive directory listing
---

`Core\IO\Walk` contains every file and folder under one folder, including the ones in its
sub-folders. `Core\IO::walk` returns it. The class has no methods. You read it with a `foreach` loop.

Each item is a `string` path relative to the folder you started from, such as `images/logo.svg`. On
Windows, the folder names are separated by `\`. `Core\Path::SEPARATOR` is the separator of the system
the program runs on. `Core\Path::join` puts the starting folder in front again. A symbolic link is an
item, but `walk` does not follow it.

The items of one folder come before the items of the folders inside it. Inside one folder, the order
is the order the operating system gives. Sort the paths if you need a fixed order.

`Core\IO::walk` reads the whole tree before it returns. A file written after that is not in the
result. You can loop over the result as often as you like. `walk` throws an `IOError` when the path
does not exist, when it is a file, or when a folder cannot be read. It throws a `RuntimeError` when
the program may not read a folder. The loop itself does not throw an error.

```txt file=site/index.html
<h1>Home</h1>
```
```txt file=site/images/logo.svg
<svg/>
```
```txt file=site/images/icons/cart.svg
<svg/>
```
```nvs
<?nvs
array<string> $paths = [];
foreach (Core\IO::walk('site') as string $entry) {
    // This changes `\` to `/`, so the output is the same on every system.
    $paths[] = Core\Str::replace($entry, Core\Path::SEPARATOR, '/');
}

// The folders are items too. This prints five paths in alphabetical order.
foreach (Core\Arr::sort($paths) as string $path) {
    echo $path, "\n";
}
```
```output
images
images/icons
images/icons/cart.svg
images/logo.svg
index.html
```
