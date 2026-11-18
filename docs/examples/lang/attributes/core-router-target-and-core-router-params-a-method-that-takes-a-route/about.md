`#[Core\Router\Target]` on a `callable` parameter says that the method takes a route method.
`#[Core\Router\Params]` on an `array` parameter of the same method says that the array fills that
route's path. Your own method then takes a route the way `Core\Router::url` does.

When a call writes the route method directly, such as `Links::entry(Shop::show, ['id' => 3])`, the
compiler checks it like a link. The method needs a `#[Core\Route]`. The array needs a key for every
capture, and each key must be a capture or a `#[Core\Query]` parameter. A value the capture cannot
convert is an error too.

The call is an ordinary call. Your method gets the route method as a `callable` value. It can pass
it to `Core\Router::url`, or call it. A route method in a variable is checked when the program runs.

A `private` route method can only be passed from inside its own class. To link to another
module, call `Core\Router::url` directly.

**The examples below** show a method that builds a link, one that signs a link, and a menu built
from route methods.
