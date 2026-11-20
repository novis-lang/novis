---
summary: reads the attributes written on a class, a method, a property or a parameter. Each call names an attribute declaration, and the compiler replaces it with the values written with that name
keywords: attribute, attribute declaration, repeatable, metadata, annotation, #[...], read attributes, attribute arguments
---

`Core\Attributes::get<T>` returns the one attribute declared as `T` on a target, or `null`. `T` is
the name of an `attribute` declaration. `all<T>` returns every attribute declared as `T`, in the order
they are written, as an `array`. An attribute with the same fields but another declaration is not
found.

The target is `Class::constructor` for a class, or `Class::method` for a method. The second argument
names one of its properties or parameters. The compiler replaces the call with its result, so a member
name computed while the program runs finds nothing. When `get` finds two attributes, the program does
not compile, and the error names `all`.

```nvs
<?nvs
repeatable attribute Route on class, method { path: string }
attribute Column on property { column: string, unique?: bool }
attribute Inject on parameter { inject: string }
attribute Audit on class { audit: bool }

#[Route(path: '/users')]
#[Audit(audit: true)]
class Controller
{
    #[Column(column: 'name', unique: true)]
    public string $name = '';

    #[Route(path: '/show')]
    public static function show(#[Inject(inject: 'repo')] int $id): int
    {
        return $id;
    }
}

var $route = Core\Attributes::get<Route>(Controller::constructor);
echo 'class: ', $route?->path ?? 'none', "\n";
var $audit = Core\Attributes::get<Audit>(Controller::constructor);
echo 'audit: ', ($audit != null ? 'yes' : 'no'), "\n";
var $column = Core\Attributes::get<Column>(Controller::constructor, 'name');
echo 'column: ', $column?->column ?? 'none', "\n";
var $method = Core\Attributes::get<Route>(Controller::show);
echo 'method: ', $method?->path ?? 'none', "\n";
var $param = Core\Attributes::get<Inject>(Controller::show, 'id');
echo 'param: ', $param?->inject ?? 'none', "\n";
var $none = Core\Attributes::all<Route>(Controller::show, 'id');
echo 'none: ', Core\Arr::count($none), "\n";
```
```output
class: /users
audit: yes
column: name
method: /show
param: repo
none: 0
```
