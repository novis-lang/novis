---
summary: what the compiler knows about the whole program — every class implementing an interface, enumerated at compile time
keywords: get_declared_classes, class_implements, implementing, plugin discovery, registry, autoload, enumerate implementors, service locator
---

`Core\Program::implementing<T>()` is written with its type argument — `T` is an interface — and expands
**at compile time** to an array literal of `new` expressions, one per non-abstract class in the program that
implements `T`, sorted by fully-qualified name. Nothing is scanned at run time, and every element is a fresh
instance typed `T`. A class is "in the program" when an `autoload` root reaches it: the compiler parses every
file under every declared root for this one query, so a class never named by any `require` is still found.
Abstract classes and the interface itself are not entries; a class reaching `T` through a parent class or
through an interface that extends `T` is.

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
```
```output
found 2
Hey!
Good day.
```
