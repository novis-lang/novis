---
summary: what the compiler knows about the whole program — every class implementing an interface, enumerated at compile time, each with one attribute read off it — and the one thing only the host knows, its identity
keywords: get_declared_classes, class_implements, implementing, implementingWith, plugin discovery, registry, autoload, enumerate implementors, service locator, attributes of discovered classes, routes of every page, program id, build id, deployment fingerprint, cache busting, asset version, revision hash
---

`Core\Program::implementing<T>()` is written with its type argument — `T` is an interface — and expands
**at compile time** to an array literal of `new` expressions, one per non-abstract class in the program that
implements `T`, sorted by fully-qualified name. Nothing is scanned at run time, and every element is a fresh
instance typed `T`. A class is "in the program" when an `autoload` root reaches it: the compiler parses every
file under every declared root for this one query, so a class never named by any `require` is still found.
Abstract classes and the interface itself are not entries; a class reaching `T` through a parent class or
through an interface that extends `T` is.

`Core\Program::implementingWith<I, T>($member)` is the same list with one attribute read off each class.
Every row is `{instance: I, attribute: ?T}`: the instance, and the one attribute on that class's own
`$member` whose fields satisfy the shape `T`, or `null` when there is none. `$member` is a method name, a
property name, or the empty string for the attributes on the class itself. This is how a framework finds
its pages and their routes in one call: `implementingWith<View, {path: string}>("render")` gives each
page beside the `path` its `#[Core\Route]` carries, so the path is written once. Everything is resolved
while compiling, and a member some implementor does not declare, or two matching attributes on one
class, do not compile.

`Core\Program::id()` is the other member, and the only one here that runs. It answers this program's
identity: `BLAKE3` over every compiled unit's content hash, in program order, folded with the digest of the
environment they were compiled for — so the same code on the same host answers the same 64 lowercase hex
characters on every run, and any edit to any file in the program answers something else. All 32 bytes are
returned; take a prefix if a shorter one is wanted, because a caller handed eight characters cannot get the
other fifty-six back. It is computed once, before the program starts, so calling it in a loop costs nothing.
Use it to bust a cache, version an asset URL, or tell one deployment's log lines from another's — it is a
digest, so it is safe to echo and reveals no source, though a reader who watches it learns when the
deployment last changed. It cannot be a constant: writing the id into a file as a literal would change that
file's bytes, and so the id it just wrote down.

```nvs file=lib/Greeter.nvs
<?nvs
namespace App;

interface Greeter {
    public function greet(): string;
}
```
```nvs file=lib/Formal.nvs
<?nvs
namespace App;

class Formal implements Greeter {
    public function greet(): string {
        return "Good day.";
    }
}
```
```nvs file=lib/Casual.nvs
<?nvs
namespace App;

class Casual implements Greeter {
    public function greet(): string {
        return "Hey!";
    }
}
```
```nvs
<?nvs
autoload 'App' from './lib';

var $greeters = Core\Program::implementing<App\Greeter>();
echo "found ", Core\Arr::count($greeters), "\n";
foreach ($greeters as App\Greeter $greeter) {
    echo $greeter->greet(), "\n";
}

// The id is a digest of these very files, so no example can print it and stay
// reproducible. Its shape is what holds from run to run.
string $id = Core\Program::id();
echo "id is ", Core\Str::length($id), " characters, and stable within the run: ", Core\Program::id() == $id ? "yes" : "no", "\n";
```
```output
found 2
Hey!
Good day.
id is 64 characters, and stable within the run: yes
```
