Returns the access rule of the route that matched, as a case of the enum you write between `<`
and `>`. For a route with `#[Access(allow: App\Role::Admin)]`, `accessAs<App\Role>()` returns
`App\Role::Admin`. A public route has the rule `Core\Audience::Public`, so you read it with
`accessAs<Core\Audience>()`.

If the rule is not a case of that enum, the result is `null`. Treat `null` as "access denied".

The server does not check this rule for you. Your program checks it once, before it calls the
route's handler, so one check covers every route.

The compiler checks every comparison with the result. If you rename a case, a comparison with the
old name does not compile. The type argument must be an enum. A rule that names a class constant
has no enum, so read that rule with `access()`.

The examples show the rule of each route, a check for one role, and one check that runs before
every handler.
