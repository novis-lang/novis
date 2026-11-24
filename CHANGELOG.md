# Changelog

Every released version of Novis, newest first. Generated from the commit log by
`bun nv release` and prepended by the release workflow -- edit a section only to correct it,
never to add one by hand.

What a version number promises is
[`rule:packaging/the-versioned-surface-is-enumerated`](docs/rules/packaging.md#packaging-the-versioned-surface-is-enumerated):
it covers the language, the `Core` library, `nvs.toml`, the CLI, diagnostic identity, the
extension ABI and `serialize()` output -- and explicitly not the Rust APIs of the `nvs-*` crates.
Before 1.0 the breaking slot moves left: `0.MINOR` carries breaking changes.

A release is cut by [the release workflow](.github/workflows/release.yml), fired by hand from the
Actions tab; [docs/release.md](docs/release.md) is the procedure and what it needs configured.

## [0.0.5] - 2026-11-21

### Breaking changes

- `Core\Time::fromIso` and `Core\Time::fromEpoch` are removed. The same methods are on `Core\Time\Instant`, with the same arguments. Use `Core\Time\Instant::fromIso` and `Core\Time\Instant::fromEpoch`. The compiler error names the new method and offers a fix that writes it.
- `Core\Time::parse` is removed. The same method is on `Core\Time\DateTime`, with the same arguments. Use `Core\Time\DateTime::parse`. The compiler error names the new method and offers a fix that writes it.
- `Core\Time::at` is removed. It built a `DateTime` from a year, a month, a day, a zone and time options. Use `Core\Time\DateTime::fromIso('2024-03-01 09:30', $zone)`, or `Core\Time\DateTime::at(Core\Time\Date::at(2024, 3, 1), Core\Time\TimeOfDay::at(9, 30), $zone)`.
- The enum `Core\Time\DateTime::Weekday` is removed. The same seven cases are in `Core\Time\Weekday`. Write `Core\Time\Weekday::Monday` where the program has `Core\Time\DateTime::Weekday::Monday`.
- `Core\Time\DateTime::weekday` returns a `Core\Time\Weekday` case. `Core\Time\DateTime::next` and `Core\Time\DateTime::previous` take a `Core\Time\Weekday` case. Write `Core\Time\Weekday::Friday` where the program has `Core\Time\DateTime::Weekday::Friday`.
- `Core\Session::start` is removed. The first call of any `Core\Session` method opens the session from the session cookie. Delete the call to `Core\Session::start`. If your program reads the session identifier itself, for example from a header, call `Core\Session::openWithIdentifier` first.
- `Core\Session::set` is removed. Each value in the session has a lifetime, and you choose it when you save the value. The `[session] ttl` setting is replaced by `short_lived` and `long_lived`. Use `Core\Session::setShortLived` for a login or other value that ends when the browser closes. Use `Core\Session::setLongLived` for a choice the visitor should keep, such as their language.
- `Core\Session::setSecret` is removed. A secret in the session has a lifetime, the same as any other value. Use `Core\Session::setShortLivedSecret` for a secret of a login, such as an access token. Use `Core\Session::setLongLivedSecret` for a secret of the device.
- `Core\Session::clear` and `Core\Session::destroy` are removed. Three new methods delete values by their lifetime, or delete the whole session. To sign a user out, use `Core\Session::clearShortLived`. To delete the whole session, use `Core\Session::clearAll`. To delete only the long-lived values, use `Core\Session::clearLongLived`.
- `Core\Session::remove` inside `Core\Html::later` or `Core\Task::runAfterResponse` does not compile. Where the compiler cannot see the call, it throws a `LogicError`. Call `Core\Session::remove` in the main part of the request, before the response is sent.
- An attribute without a name, `#[{ path: '/users' }]`, is removed. A `type` alias after `#[` does not compile any more. Declare the attribute with `attribute Route on method { path: string }` and write `#[Route(path: '/users')]`. The error on a `type` alias offers a fix that writes the declaration.
- `Core\Attributes::get<Table>` returns only the attributes written as `#[Table(...)]`, so two attributes with the same fields are not mixed up. For a method that a class inherits and does not override, `get` and `all` return the attributes its parent class wrote on it. `Core\Reflect\AttributeInfo::name` returns the full name of the declaration. Name an attribute declaration between `<` and `>`, as in `Core\Attributes::get<Table>(User::constructor)`. A shape such as `get<{name: string}>` does not compile.

### New

- A class with no parent class can implement `Revocable`, so that the program can say an object is no longer valid. After `$object->revoke()`, every `Core\ObjectMap`, `Core\ObjectSet` and `Core\Heap` removes the object immediately. Any other use of it throws `RevokedObjectError`, and its message names the line of the `revoke()` call.
- `nvs test --mutate=Shop\Cart` makes mutants only in the code you name, and `#[NoMutate]` keeps a class or a method out. `--since main` makes mutants only on the lines you edited, and a run compiles the program once. Survivors you checked go in `mutants.toml`, and the report lists the others by method.
- An attribute is declared with `attribute Table on class { name: string }`, and the words after `on` list where it may be written. An attribute declared inside a class may be used only by the classes that extend it. `repeatable` allows more than one use on the same declaration.
- `nvs reload` makes a running `nvs serve` use the code and configuration on disk at once, and waits until the server has compiled each changed program. It prints each error with its file and line. The exit code is `0` when the server uses every new file, `1` when a file has an error and `2` when no server runs.
- `nvs code` reads and saves the editor's index file, a list of every name in the project and every place it is used. A call analyses only the files whose content changed, so it is faster. `nvs code calls` and `nvs code reach` take several methods in one call.
- `Core\Test::run` runs your program as a command, with the arguments you give and an optional `stdin`. It returns a `Core\Test\RunResult` with the exit code, the standard output and the standard error. The program runs inside the test's own process.
- `Core\Test::logs` returns the log records written in the current test. The list includes the records of each request and each run the test started. Each record has `level()`, `message()` and `fields()`.
- `Core\Test::sentMail` returns the messages your program sent in the current test. Each one is a `Core\Test\SentMail`, and its methods return the values given to `Core\Mail::send`.
- Under `nvs test`, `Core\Mail::send` sends nothing and opens no connection. It still checks the capability, the mail settings and every address. `Core\Test::sentMail` returns the message.
- `Core\Test\Response::error` returns the class and message of the error that stopped a request, such as `RuntimeError: no stock`. It returns `null` when the request did not fail.
- `Core\Test::request` has new options: `clientIp`, `scheme`, `cookies`, and the body options `json`, `form`, `multipart` and `contentType` of `Core\Http\Client`. The response is the one a server sends. A write from another site gets `403`, and a `HEAD` request gets no body.
- `#[Test(env: [...])]` sets environment variables for one test. `Core\Env` reads them in the test and in each request and run it starts. Other tests do not see them.
- In a test with `#[Test(at:)]`, `Core\Time::sleep` ends at once and moves the test clock forward. A timeout of `Core\Task::all` and the time to live of a `Core\Cache::memory` entry use the same clock. Without `at:`, a sleep waits in real time.
- A callable in `Core\Html::later` can read the request with `Core\Request`. The path, the query, a header, a cookie and the route parameters are the same as in the main script. A body read in the callable throws a `LogicError`.
- `Core\Time\Weekday` is the enum of the days of the week, from `Monday` to `Sunday`. `Core\Time\Date::weekday` and `Core\Time\DateTime::weekday` return one of its cases.
- `Core\Time\Instant::fromIso` reads a timestamp such as `2024-03-01T12:00:00Z` and returns an `Instant`. `Core\Time\Instant::fromEpoch` builds an `Instant` from the number of seconds since 1 January 1970.
- `Core\Time\Date::fromIso` reads a date such as `2024-03-01`. `Core\Time\TimeOfDay::fromIso` reads a time such as `09:30` or `09:30:15.5`. A string literal in either method is checked when the program compiles.
- `Core\Time\DateTime::fromIso` reads a local date and time, such as `2024-03-01 09:30`, in the zone you give. `Core\Time\DateTime::at` builds a `DateTime` from a `Date`, a `TimeOfDay` and a zone. `Core\Time\DateTime::parse` reads text in a pattern you choose.
- `Core\Time\Date::year`, `Core\Time\Date::month` and `Core\Time\Date::day` return the parts of a date as numbers. `Core\Time\Date::weekday` returns a `Core\Time\Weekday` case, and `Core\Time\Date::dayOfYear` returns the day of the year from `1`. `Core\Time\Date::isLeapYear` checks if the year is a leap year.
- `Core\Time\DateTime::year`, `Core\Time\DateTime::month`, `Core\Time\DateTime::day`, `Core\Time\DateTime::hour`, `Core\Time\DateTime::minute`, `Core\Time\DateTime::second` and `Core\Time\DateTime::nanos` return the parts of a date and time as numbers. Each one reads the clock in the zone of the value. `Core\Time\DateTime::year` returns an `int`, and the others return a `uint`.
- `Core\Time\TimeOfDay::hour`, `Core\Time\TimeOfDay::minute`, `Core\Time\TimeOfDay::second` and `Core\Time\TimeOfDay::nanos` return the parts of a time as numbers. Each one returns a `uint`.
- `Core\Time\Instant::in` takes a zone written as text, such as `$instant->in('Europe/Berlin')` or `$instant->in('+05:30')`. The name is checked when the program compiles, and a wrong name does not compile. Every other method that takes a zone accepts text in the same way.
- `Core\Time\Zone::of` with a name written in quotes is checked when the program compiles. An unknown name, or an offset such as `'+02:00'`, does not compile. A name in a variable throws a `RuntimeError` when the program runs.
- `Core\Session::setShortLived` saves a value that is deleted when the browser closes, or after `[session] short_lived` without a request. `Core\Session::setLongLived` saves a value that stays after the browser closes, for `[session] long_lived`. Use the short lifetime for a login and the long one for a choice such as a language.
- `Core\Session::setShortLivedSecret` and `Core\Session::setLongLivedSecret` save an encrypted secret with a short or a long lifetime. `Core\Session::getSecret` reads both, and `Core\Session::get` returns `null` for them.
- `Core\Session::clearShortLived` deletes every short-lived value, and `Core\Session::clearLongLived` deletes every long-lived value. Both give the session a new identifier. `Core\Session::clearAll` deletes the whole session and tells the browser to delete both session cookies.
- `Core\Session::openWithIdentifier` opens the session under an identifier your program read itself, for example from a header. It reads no cookie and sends no cookie. It must be the first `Core\Session` call of the request.
- `Core\Session::get`, `Core\Session::remove` and `Core\Session::regenerate` open the session on their first use. In a program that handles no request, they throw a `LogicError`. `Core\Session::get` returns `null` for a short-lived value after the browser closes.
- The session saves only the keys that a request changed. Two requests that change different keys at the same time keep both changes. `Core\Session::regenerate` keeps every value, also a value that another request saved while it ran.
- `Core\Csrf::token` and the `Core\Passkey` methods that save or check a challenge open the session on their first use. You do not call anything before them. In a program that handles no request, they throw a `LogicError`.
- `Core\Reflect\ClassInfo::parent` returns the name of the class this class extends. It returns `null` for an interface and for a class that extends nothing. Call it in a loop to read the attributes of each parent class.
- You can write `#[Core\Router\Target]` on a `callable` parameter and `#[Core\Router\Params]` on an `array` parameter. Your method then takes a route method and its values, the way `Core\Router::url` does. When a call writes the route method directly, the compiler checks it like a link.
- A `[[restrict]]` table in `package.toml` says that a class, a method or a namespace may be used only in some places. A use outside those places does not compile. A call through `mixed` or `Core\Reflect` throws a `LogicError` when it runs.
- After an array property, a property path can use `@key` for an element's key and `@value` for the element, such as `authors.@value.name`. A path converted while the program runs is checked in the same way. `Core\Reflect::steps` lists both steps.
- You can pass a string literal to a parameter whose type is a union of a class and one `string<propertyPath<T>>`, `property<T>` or `propertyPath<T>`. The compiler checks the literal as that type.
- `nvs code refs` lists a route method passed to a `#[Core\Router\Target]` parameter as a use of that method.
- In the output of `nvs meta --json`, the description of a `@param` keeps its line breaks.
- When a request is recorded, each call to `Core\IO::read`, `Core\IO::readText`, `Core\IO::readBytes`, `Core\IO::write` and `Core\IO::append` is saved with its line and its file path. The editor shows the number of these calls and their time above the line.
- `Core\Json::isValid` checks a document without building its values, so it is faster and uses less memory. It returns `true` for a valid document that is too large to decode under the memory limit.
- On a file that cannot seek, a method called after `Core\IO\File::readLine` throws an error. No bytes of the file are lost.
- `foreach` gives the key and the value of a `Core\ObjectMap`: write `foreach ($map as var $key => var $value)`. A class of your own can do the same by implementing `KeyedIterable<K, V>`.
- `nvs test CartTest.nvs:42` runs the test that contains line 42. `nvs test --since main` runs only the tests that reach code you changed since the git branch `main`.
- A `///` comment above a field of a shape or an attribute describes that field. The editor shows the comment when you type the field's name or point at it.
- The attributes of `Core`, such as `#[Core\Route]`, are checked like your own attributes. One written in a place it does not allow is the error `E0874`. A second `#[Core\Access]` or `#[Core\Api]` on one declaration is the error `E0763`.
- A `[[schedule]]` entry with `on = "reload"` also runs when a check of the program files replaces a program or finds an error. It runs once for the whole check, not once for each program. `Core\Script::arguments()` has two more keys: `programs`, the number of programs replaced, and `failed`, the number of programs with errors.
- The server writes one `program reloaded` line to its log for each program it replaces. A program that does not compile writes one `program reload failed` line, with the first error and its file and line.

### Faster

- `Core\ObjectMap` and `Core\ObjectSet` are faster. A lookup with a `string`, `int` or object key costs about the same as a lookup in an array. A `foreach` over either does not copy it.
- `Core\Time\Zone::fixed` with a written offset, such as `Core\Time\Zone::fixed(5h30m)`, is built once when the program compiles. Each use of it costs no call and no memory.
- A request with a body of 16 KiB or less is sent in one write, together with its headers. A larger body, or a body that reads a file, is sent in two writes.
- `Core\IO\File::readLine` reads ahead into a buffer, so reading a file line by line is faster. Each open file that has read a line uses about 8 KB more memory while it is open.
- `Core\Arr::groupBy` and `Core\Arr::countBy` copy a group's key only when the group is new. They allocate less memory and run faster on large arrays.
- `Core\Db\Connection::query` and `Core\Db\Connection::stream` create the column names once for each result. Every row uses the same names, so reading many rows is faster and allocates less memory.
- `Core\Json::decodeAs` matches each key with less work and finds each class's constructor once per call. Decoding a list of objects allocates only the objects and their strings.

### Fixes

- `Core\Session::get` inside `Core\Html::later` returns the value saved in the session of the request. This also works when the main part of the request did not use the session.
- A `PropertyGuard` whose `beforePropertyUnset` replaced the array could make `unset($object->property[$key])` use the old, freed array. Now the `unset` removes the key from the array the property has after the guard.

### Internal changes

600 further commits: docs (396), chore (109), test (71), refactor (18), build (4), wip (2).

### Details

#### Features

- **bench**: bench-compare --direct prints each HTTP peer's CPU time per request with no proxy ([`2e308811`](https://github.com/novis-lang/novis/commit/2e3088115e32873eb9fc579c0d801ae39d55d992))
- **bench-compare**: --reps-check reads whether three of five passes give the recorded medians ([`0a0aa84b`](https://github.com/novis-lang/novis/commit/0a0aa84bbd0ed8a906f9aacd8c2027091894368b))
- **bench-compare**: .NET is a peer, an ASP.NET Core minimal API on Kestrel behind the same nginx ([`95fb05fa`](https://github.com/novis-lang/novis/commit/95fb05faa6639142f7f8dde4fb4ee8afdf99cb6f))
- **bench-compare**: Node.js is a peer, behind the same nginx, with a cluster worker per cpu ([`99303230`](https://github.com/novis-lang/novis/commit/993032307d68ff3e4d4bf1e0176bbda342bd667a))
- **bench-compare**: Python is a peer, an ASGI program under uvicorn behind the same nginx ([`3c167478`](https://github.com/novis-lang/novis/commit/3c1674783c70d6a5ea49e3e049f64abb152466e5))
- **bench-compare**: a recording builds each image once, checks answers on rep 1, and keeps an unchanged row ([`841fd3f5`](https://github.com/novis-lang/novis/commit/841fd3f50fbb3b219d69db026471726385b159b9))
- **bench-compare**: a row run writes the record after each row, so a stopped run leaves its finished rows ([`d676c2f8`](https://github.com/novis-lang/novis/commit/d676c2f8fb6e5bb9ac52eedb25781486663fe5bc))
- **bench-compare**: a run walks a plan counted before it starts, and the record says where its time went ([`09d59175`](https://github.com/novis-lang/novis/commit/09d5917573475cc99dc3886e03a9ea5644b0e159))
- **cache**: the shared tier's client reads a hash and moves a key ([`9bdd0634`](https://github.com/novis-lang/novis/commit/9bdd06344d4db25964083dff0d4033e50f9cb0ef))
- **check**: a copy that moving one statement removes is warned as W1032 or W1033 ([`f26dc6ad`](https://github.com/novis-lang/novis/commit/f26dc6add7cad9a257ff78e1cc660d88ae92e66b))
- **cli**: a hidden --no-cheaper-forms compiles every form as written ([`d39d487b`](https://github.com/novis-lang/novis/commit/d39d487b3f29b75d85fbed0411baf158c65eb5de))
- **cli**: nvs code calls and reach take several methods ([`3d11d542`](https://github.com/novis-lang/novis/commit/3d11d542dba3472f7fcfe55065b0e224a47e6ef2))
- **cli**: nvs code loads, checks by content and saves the editor's index file ([`87704784`](https://github.com/novis-lang/novis/commit/87704784c8d4d21085846e9c025f9e8d05335558))
- **cli**: nvs reload reloads every server of a data folder and exits 0, 1 or 2 ([`57295046`](https://github.com/novis-lang/novis/commit/5729504620be6cade547fbdbe0c033fb174535a5))
- **cli**: run, test and serve take a hidden --no-in-place, and nv suite --no-in-place-too runs a suite both ways ([`bdb63ad3`](https://github.com/novis-lang/novis/commit/bdb63ad386024eb9dfaa7f0509e2d5ba3d7b54f8))
- **codegen**: an object's inline release is bounded by the revoked flag ([`7c396b8c`](https://github.com/novis-lang/novis/commit/7c396b8c1f3f08247d812f8c29c4d1948dbe00e2))
- **codegen**: every use of a revoked object throws, and the check is compiled only where it may be revocable ([`81702801`](https://github.com/novis-lang/novis/commit/8170280181962f9fface1d337970114ba5a9b498))
- **codegen**: revoke() and isRevoked() run the runtime's functions, written or by name ([`18fcb978`](https://github.com/novis-lang/novis/commit/18fcb9781c30b8a129ec1e3b7ac02d476fb654b3))
- **config**: a [[restrict]] table is read from the application's and each package's package.toml ([`ffbc01c7`](https://github.com/novis-lang/novis/commit/ffbc01c73925e9fed950a9a63d12bcf3df1f7a08))
- **debug**: a recorded request counts the copies it made at each kept-copy site ([`191df1eb`](https://github.com/novis-lang/novis/commit/191df1eb68958cf1761ab6a13a67839a477e0c0d))
- **debug**: the viewer lists a request's copy sites, the most copied first, with their help ([`cbb0bbfa`](https://github.com/novis-lang/novis/commit/cbb0bbfaad69141d370de112c217245326c70f63))
- **forms**: a form only an edit can make faster is W1034 with its fast form as an alternative ([`a77c9f24`](https://github.com/novis-lang/novis/commit/a77c9f24b2d4293fc1da4724ebe332017d47f235))
- **ir**: `.=` on a property or static property is lent to its append by the ownership pass ([`179ba0c5`](https://github.com/novis-lang/novis/commit/179ba0c5beafea2fbfb757ba501f87dcb3d65d48))
- **ir**: a caught conversion that assigns null compiles as the conversion that returns null ([`54478254`](https://github.com/novis-lang/novis/commit/54478254ef7d54bb23a72edd073c010f4ba8d469))
- **ir**: a comparator that compares one expression of each side compiles as by ([`fca242a5`](https://github.com/novis-lang/novis/commit/fca242a50ed2f720d9c69fe0f0681ba8a2deda2f))
- **ir**: a debug build poisons a lent property and traps a read of it ([`3dea4f02`](https://github.com/novis-lang/novis/commit/3dea4f024effddd726794bc5b521eb3f8d6ad523))
- **ir**: a filter that tests contains against a captured array compiles as diff or intersect ([`3364b554`](https://github.com/novis-lang/novis/commit/3364b55414f917c3864a8f767bbc50750a8b7f94))
- **ir**: a first, firstKey or any of a filter whose callback is proven compiles as find, findKey or any ([`632a416c`](https://github.com/novis-lang/novis/commit/632a416c064e476bdf8aa5041dbbda31a24b4a11))
- **ir**: a generator's resume takes its parked locals, and every loop body reaches the ownership pass ([`1a7338e8`](https://github.com/novis-lang/novis/commit/1a7338e8ab8496f30165efa9ff69927c5cfedcbc))
- **ir**: a join and a literal's first spread take the local they open with ([`bc590aaa`](https://github.com/novis-lang/novis/commit/bc590aaaefef3b2de94235015ad0a5d72101aa85))
- **ir**: a keyed foreach runs over a Core\ObjectMap and a user KeyedIterable ([`483a0a68`](https://github.com/novis-lang/novis/commit/483a0a68fc12ec88d23924867de20b41e726d742))
- **ir**: a local a catch reads is lent to a known Novis callee's lent body ([`f7472189`](https://github.com/novis-lang/novis/commit/f747218948813dd4e48ad6042be97044454e27b8))
- **ir**: a loop over keys, values or a range walks without a list, and an unread key is not made ([`55fccabb`](https://github.com/novis-lang/novis/commit/55fccabbe85a7940b26e6d648ff48e069de6b923))
- **ir**: a method call with one body at or below its receiver's class is direct ([`070c4517`](https://github.com/novis-lang/novis/commit/070c451772e8915c167b959b640eae864dc690a7))
- **ir**: a null test of a match and a count or first of keys or values compile as their fast forms ([`7851ecd0`](https://github.com/novis-lang/novis/commit/7851ecd0d72a186ea01e0b83535980a1eb51f50a))
- **ir**: a property replaced by a known Novis callee's result is lent to its lent body ([`d091beec`](https://github.com/novis-lang/novis/commit/d091beec4f7c18a0cec521e4e09c82d8efecba50))
- **ir**: a second holder is released at its last use, and a loop that writes only its own entry walks its variable ([`af0e518f`](https://github.com/novis-lang/novis/commit/af0e518f1ee385fb651ee1e4c9af89bab29c887f))
- **ir**: a static:: call of a method no class overrides is a direct call ([`ef06d797`](https://github.com/novis-lang/novis/commit/ef06d79700af5eaff0365e49c347304eb3f92a5b))
- **ir**: a static::NAME no class declares again is its constant ([`c35f5b81`](https://github.com/novis-lang/novis/commit/c35f5b817b1cdced616f257dde5912ec8a433a77))
- **ir**: an array element a statement replaces is lent to an owning Core member ([`4fe6079b`](https://github.com/novis-lang/novis/commit/4fe6079bed668e5218cf4fbaf8a8c99c3ae06f3b))
- **ir**: an array literal whose keys are all different literals is built in one step ([`ee8e527a`](https://github.com/novis-lang/novis/commit/ee8e527a8b6877488ea3933c33e193730667f2b6))
- **ir**: each function's effect summary covers its callees, solved over the call graph ([`ebb8197b`](https://github.com/novis-lang/novis/commit/ebb8197b74f6763b2a898c37e9da4487877c5e58))
- **ir**: each instruction states its effects, and a property slot is lent to an owning member ([`9154c507`](https://github.com/novis-lang/novis/commit/9154c507b6d55ad517b787889c324d5daa416023))
- **ir**: one ownership pass decides every move from the IR ([`437b803b`](https://github.com/novis-lang/novis/commit/437b803bc198c18246c706aa45133fe7ebc5578c))
- **ir**: one table of cheaper forms, with the rule and the test that counts its cases ([`5c52826a`](https://github.com/novis-lang/novis/commit/5c52826a06e25747d3158fc67c043caa487104bd))
- **ir**: the ownership pass lends a guarded or `lateinit` element-write root ([`6ffbaf62`](https://github.com/novis-lang/novis/commit/6ffbaf6224476409bdcf068f33f731c5f0877868))
- **ir**: the ownership pass lends a plain slot to a nested element write and a nested `.=` ([`c768561b`](https://github.com/novis-lang/novis/commit/c768561b242fbe20042435c6a38e11fa27cc1d4d))
- **ir**: the ownership pass lends a plain slot to a one-level element write ([`cffcc132`](https://github.com/novis-lang/novis/commit/cffcc132672f7f2affc0ee7d10fd64a59c15cfc2))
- **ir**: the ownership pass records every copy it keeps, and a unit carries the table ([`5cbfb182`](https://github.com/novis-lang/novis/commit/5cbfb1822df2d502015cc65a8dfd8181ff42ca57))
- **lsp**: a Core stub documents its members with @param, @return and @throws ([`31c0a117`](https://github.com/novis-lang/novis/commit/31c0a117ffd2603278aa4deed97fd598f529dd71))
- **lsp**: a Core\Router\Target call completes, hovers and finds its route like a link ([`dbb1e4bf`](https://github.com/novis-lang/novis/commit/dbb1e4bf346ef8e8ee623e7377942028cede2100))
- **lsp**: a bare callable or mixed parameter is offered the type every caller passes ([`bdf22372`](https://github.com/novis-lang/novis/commit/bdf22372fe4f35a88231026bd17ff2e08cf5d8fd))
- **lsp**: a content-checked load of the index file answers as a cold build ([`deffd3d2`](https://github.com/novis-lang/novis/commit/deffd3d2d35df369988fc4ab324713687b28276b))
- **lsp**: a hover on Revocable, revoke() and isRevoked() shows its card ([`ad12cfbb`](https://github.com/novis-lang/novis/commit/ad12cfbbb75d4aa61c404b0890c47a66b48bf128))
- **lsp**: a key in any object literal at a shape type shows the doc of its shape field ([`24fdde95`](https://github.com/novis-lang/novis/commit/24fdde95fbcb378cf293dd0eaa5f0a9ff05acafa))
- **lsp**: a line where a recorded request copied a value shows its count and its help ([`c6e9bc8e`](https://github.com/novis-lang/novis/commit/c6e9bc8ecc2b401999597745ed468bf60d80f74f))
- **lsp**: a property path completes, explains and jumps through @key and @value ([`5de977ff`](https://github.com/novis-lang/novis/commit/5de977ffb62cb16e530d5e0bc8ad1b3cdb73d65e))
- **lsp**: a string at a property<static> field of an attribute completes and jumps to the property ([`67e6cfcd`](https://github.com/novis-lang/novis/commit/67e6cfcd34132c48374885cbf0e1f2f106ad7e43))
- **lsp**: a userland attribute's payload completes its shape's fields and allowed values ([`ed441259`](https://github.com/novis-lang/novis/commit/ed441259577623b98255f9c032a980859920b278))
- **lsp**: an edit's re-index shows a progress run, and lsp-start measures memory ([`36615c96`](https://github.com/novis-lang/novis/commit/36615c9658d8bfd8e565e497ea374ac0f10b8148))
- **lsp**: the editor offers at #[ only the attribute declarations that fit, and shows each key's doc ([`9b154d39`](https://github.com/novis-lang/novis/commit/9b154d39c46ca600414f3e661b47c48e9b018dae))
- **lsp**: the editor shows W1032 and W1033 and offers each fix as a code action ([`7e4e36d8`](https://github.com/novis-lang/novis/commit/7e4e36d85fdd6ae4f27d57c3724af62cb035d361))
- **lsp**: the editor shows a restricted use, its rule, its fix, its hover and its completion ([`618bf682`](https://github.com/novis-lang/novis/commit/618bf6827d8c0a0d8a5811bd8be73bfcc324ffa0))
- **lsp**: the recorded lens counts HTTP, cache, session and file calls by line and names their kind ([`707ccd67`](https://github.com/novis-lang/novis/commit/707ccd67bd24bdc99114a4326818743260d24012))
- **mutate**: a log call's arguments and a thrown exception's message make no mutant ([`04e37561`](https://github.com/novis-lang/novis/commit/04e37561f723077e6b3483779dc84eabf55727cb))
- **mutate**: a mutation run compiles the program once, with every mutant behind one switch ([`c394b68d`](https://github.com/novis-lang/novis/commit/c394b68d6bead7f45a36361b0e596d3bc873b567))
- **nv**: bun nv close --goal runs a goal's end check in about three minutes ([`b3721460`](https://github.com/novis-lang/novis/commit/b37214606cd1a6db9e7eed99d40bf777d40821e8))
- **nv**: proofs --goal and --since judge the features a goal's work reaches ([`c01ab405`](https://github.com/novis-lang/novis/commit/c01ab405c0e523534b0c0260b58f64371f391f96))
- **nv**: serve-mem measures memory and threads across compiler panics ([`dda46734`](https://github.com/novis-lang/novis/commit/dda4673469c8fef5126c0d450e70ae23f2c4309a))
- **nv**: serve-mem measures memory per recompile and counts answers under keep-alive load while the source is edited ([`ae2513b4`](https://github.com/novis-lang/novis/commit/ae2513b436a2bf2fc0d5e74fa520287b06bcbe8b))
- **nv**: serve-mem sets the copy's memory limit and keeps the server's log ([`ea9a5527`](https://github.com/novis-lang/novis/commit/ea9a552757bb8cec3df6c6ac31f31644ad4646cd))
- **pkg**: nvs pkg api lists a package's restrictions and nvs pkg diff calls a new one a break ([`a9e9f9f9`](https://github.com/novis-lang/novis/commit/a9e9f9f96fa10a7d9f5c594bac8bcfddbf929736))
- **proofs**: Reviewed is a feature proof, and the website stale guard runs review ([`60a2bfc8`](https://github.com/novis-lang/novis/commit/60a2bfc89b72d6822abbabd8f6ebc5573fc26274))
- **proofs**: the perf ledger keeps only the records a reader uses ([`049daebf`](https://github.com/novis-lang/novis/commit/049daebf4a16c740c95e3d920d60d0b19472cee1))
- **reflect**: ClassInfo::parent names the class a described class extends ([`309a58e6`](https://github.com/novis-lang/novis/commit/309a58e6f10aa9ca53b82269a583d5151ad2d588))
- **reflect**: a reflective act on a restricted member outside its allowed places throws ([`81ff7023`](https://github.com/novis-lang/novis/commit/81ff702318e5442123b023e7d2bf6ad871dbcc50))
- **release**: the release notes start with the change entries, and a release moves them out ([`482eb9e3`](https://github.com/novis-lang/novis/commit/482eb9e3928ef0d4c77833835493c341af1f3078))
- **rereads**: a second read of one cache key, query or file path is W1035 ([`930a1dec`](https://github.com/novis-lang/novis/commit/930a1dec6642672ee14184c08e04066eb6511ed3))
- **runtime**: a Core member can own its subject through nvs_helper!'s check and apply form ([`729cb594`](https://github.com/novis-lang/novis/commit/729cb594bffc52b8f802f4f69cbee59f02e35c32))
- **runtime**: a Core\Html::later slot reads the request it belongs to, and a body read there throws ([`df66796b`](https://github.com/novis-lang/novis/commit/df66796b7aeae95bc2d40f9815247acaeb0b56f0))
- **runtime**: a list has room at both ends, and a map knows its first live entry ([`cd5e776f`](https://github.com/novis-lang/novis/commit/cd5e776fbc9f595380e935fb5a5a409e05fc087d))
- **runtime**: a run-time path and Core\Reflect::steps take @key and @value ([`fa63365d`](https://github.com/novis-lang/novis/commit/fa63365df3b4d6dcef9ebeee4dccd6b5db19da1b))
- **runtime**: a sole string drops its front without moving the rest ([`d7b3e5ea`](https://github.com/novis-lang/novis/commit/d7b3e5ea8ab6ab10c20cdd55a196c9e05b33c5ea))
- **runtime**: an object carries a revoked flag in its count word, and revoke keeps its first site ([`a03baee8`](https://github.com/novis-lang/novis/commit/a03baee8d404a1c43637afbbf2af37575c6b0805))
- **scaling**: a run ladder with rounds is judged by the difference of half and all of its rounds ([`0285ab2c`](https://github.com/novis-lang/novis/commit/0285ab2c5a0966f386ba9d879d37654f125575ce))
- **serve**: a reload through the endpoint checks now, waits for its compiles and replies ([`8590f6cb`](https://github.com/novis-lang/novis/commit/8590f6cb0a02a42acdd91b5b30f1b310cb9f6a42))
- **serve**: a swapped or broken program is one log record and one reload outcome with its counts ([`f24fe19a`](https://github.com/novis-lang/novis/commit/f24fe19ae2a2ef96b776298abe8ebcdfba2fd740))
- **serve**: each nvs serve opens a private reload endpoint in its data folder's run/ ([`0439a220`](https://github.com/novis-lang/novis/commit/0439a2202fef475727320702b96b33445554aaa8))
- **server**: a request body that arrived with its head reaches the program with no park ([`a545d6a0`](https://github.com/novis-lang/novis/commit/a545d6a01ef031525aa2545c55477345fc3e5cd6))
- **session**: Core\Csrf and Core\Passkey open the session on first use and name no start ([`a30e4aad`](https://github.com/novis-lang/novis/commit/a30e4aad76173a87969168297810cef16f94086d))
- **session**: a session is kept and written back per key on both stores ([`bd25be9d`](https://github.com/novis-lang/novis/commit/bd25be9d6615dd8777f00c609595b285de24ee31))
- **session**: both stores keep each value's lifetime, two clocks and a short token ([`6ea7ba22`](https://github.com/novis-lang/novis/commit/6ea7ba2275a83dab93fcdd23cd5822bb10df68fd))
- **session**: the session opens on its first use and every write states its lifetime ([`471783bb`](https://github.com/novis-lang/novis/commit/471783bbb7cdd9ce9aeea5423e337b66d57a8de8))
- **site**: a Configuration, CLI or Types page shows each feature's changelog ([`0c4a1d8c`](https://github.com/novis-lang/novis/commit/0c4a1d8c38adbdb0d80eb77eb5da13dbee577f97))
- **site**: a feature added, removed or changed since the snapshot needs an entry in changes.json ([`694602d6`](https://github.com/novis-lang/novis/commit/694602d6b263083e7010aa94b227b0fca3b915f8))
- **site**: a feature's covering tests are in its page hash, and its about.md and help card are stamped ([`3317b913`](https://github.com/novis-lang/novis/commit/3317b9137b41b4120499424aba9a778378c3dc9d))
- **site**: a tip box has a rainbow border in the six site tones, a subtle ground and a blue title ([`56a2fd54`](https://github.com/novis-lang/novis/commit/56a2fd5417ee6fdfb83d58e424aacc11f7c68d4a))
- **site**: every text a user reads has a key and a hash, and a changed one needs a review stamp ([`94311314`](https://github.com/novis-lang/novis/commit/94311314567c32ec0e4a2fabcd749a7f2a50a1d9))
- **site**: the home page links the comparison above the walkthroughs, with the two things only Novis and PHP do ([`e3d5e59a`](https://github.com/novis-lang/novis/commit/e3d5e59ab3487020bb8591a7f9829cbbd8fb982f))
- **site**: the website stale guard runs the changes part ([`9ac03900`](https://github.com/novis-lang/novis/commit/9ac03900041e4707ef88a098c2af8b9571bc28ed))
- …and 44 more, in the full changes below.

#### Fixes

- **apps**: chat's signed-in test browser sends the short session cookie with the identifier ([`b43ff609`](https://github.com/novis-lang/novis/commit/b43ff6098037ece480911c1aa53a792546a6e1ac))
- **apps**: the registry's test browser keeps every cookie a response sets, both session cookies too ([`8d945cbe`](https://github.com/novis-lang/novis/commit/8d945cbeb21f2d6886f443d90ff0884c74c4df98))
- **bench-compare**: the parallel row grants Novis no setting, because its default runs a script in the app folder ([`10463e08`](https://github.com/novis-lang/novis/commit/10463e08d1f883b8454944a8df71524021142d74))
- **ci**: a green push to main publishes the dev image again ([`64a1e129`](https://github.com/novis-lang/novis/commit/64a1e12999879c4fbc89cc56f8d6f2269a03b58a))
- **cli**: a compile that panics is a failed compile for its content ([`8d54b620`](https://github.com/novis-lang/novis/commit/8d54b62014ffaffaf191a9d5a9af45d801ae8c2b))
- **codegen**: a finished unit frees its JIT module and keeps only its pages ([`f05b7b44`](https://github.com/novis-lang/novis/commit/f05b7b447234a8851f212a5cc49224dff4f7ef85))
- **codegen**: a unit whose code generation fails or panics frees its pages ([`eff7d8fa`](https://github.com/novis-lang/novis/commit/eff7d8faf0530200c7413735b57fe974b2c5b764))
- **deps**: the fuzz lock has cranelift 0.137.2 and wasmtime 50.0.2, as the workspace does ([`6093d021`](https://github.com/novis-lang/novis/commit/6093d02168d121da6ebb74dcb36ca5a4aee8e190))
- **deps**: wasmtime is 50.0.2 and cranelift 0.137.2, past RUSTSEC-2026-0566 to -0573 ([`eb5c02e9`](https://github.com/novis-lang/novis/commit/eb5c02e983f0951f59c598c89c738cac05cf09c3))
- **docs**: the goal-end gates are green but for the perf records ([`47a4f3af`](https://github.com/novis-lang/novis/commit/47a4f3af2a2d19abe14d456fb40c83f51c6ccc38))
- **examples**: `Storage::delete`'s cleanup example reads back only its own keys ([`c1e505db`](https://github.com/novis-lang/novis/commit/c1e505db3d6da52faa23f45080462fcf23f4b390))
- **footprint**: a footprint log's own allocations are in no count, from the first line of nvs ([`c7d57c34`](https://github.com/novis-lang/novis/commit/c7d57c34705f4b3208f5647577d53fa0a778129d))
- **gate**: the revoke primitives deny a panic, and three tests count what the chain added ([`f4403813`](https://github.com/novis-lang/novis/commit/f4403813094973646477c0c183c7e564623d19f6))
- **goals**: the plan and less-work checks name each test as bun prints it ([`3f4a82c8`](https://github.com/novis-lang/novis/commit/3f4a82c8a1fd63b5e0dc60c37cbb23584254e040))
- **http**: a test request reads a file part within the request's memory limit ([`0d84d303`](https://github.com/novis-lang/novis/commit/0d84d303ea7f74eb3e4fde9a6cff22946a274aaf))
- **ir**: `is Iterable` and `is KeyedIterable` hold for the Core classes the checker reads as one ([`438e9c44`](https://github.com/novis-lang/novis/commit/438e9c44af256cdf6b47a7d9ff5b552d5b50547c))
- **ir**: a generator reads its called class from its state object ([`160d5069`](https://github.com/novis-lang/novis/commit/160d506950a2568b985d9aa150dbf54ce0b600f3))
- **ir**: a getter read owns its value, and a program with doubles calls every method virtually ([`cde817c1`](https://github.com/novis-lang/novis/commit/cde817c1538a16b29943e15a51fd8b96a107854f))
- **ir**: a join led by a string consumes that string and appends onto it ([`668d5f1c`](https://github.com/novis-lang/novis/commit/668d5f1c4e8cda512a64bae21154f8eb4a8c9549))
- **ir**: a loop that reassigns a local, and a call with a later argument that can fail, move it ([`d58f7224`](https://github.com/novis-lang/novis/commit/d58f7224eb984bf2880cf6501c690fb760a611c0))
- **ir**: an element `unset` holds its root across a guard, and the ownership pass lends a plain root ([`ed5e121c`](https://github.com/novis-lang/novis/commit/ed5e121c61935e2c4e70bf247f7940a01ad1d363))
- **ir**: the called class is loaded right after the receiver is defined ([`16aeb27d`](https://github.com/novis-lang/novis/commit/16aeb27d5adca81a5b8eeca704253ec0fac94767))
- **json**: the constructor's door into compiled code is constructor_of, and its row says who typed it ([`9c62bc7f`](https://github.com/novis-lang/novis/commit/9c62bc7fe37fc33e7c1cd1dff217304f5780b891))
- **kit**: a DONE claim rewrites last-sweep.md in either sweep mode ([`3fea85e8`](https://github.com/novis-lang/novis/commit/3fea85e8042cc51a768b646685da4be7ac116aaa))
- **kit**: a check id names one check, and a close leaves checks to DONE ([`074512e4`](https://github.com/novis-lang/novis/commit/074512e442808c979ed5096d0f6ddf965368b8ad))
- **lsp**: a cold index plans no entry ahead whose program's plan still runs ([`909455e4`](https://github.com/novis-lang/novis/commit/909455e4aba4deac9866893fba1802ec3e586fcd))
- **lsp**: a worker frees its allocator's cached blocks before its thread ends ([`edb767b4`](https://github.com/novis-lang/novis/commit/edb767b46f449ba07c8e2fac0a10188366c52360))
- **lsp**: an open document reads the output the index stored for its graph ([`b78c7b21`](https://github.com/novis-lang/novis/commit/b78c7b215192d317b508fa322d7759ccc7398fa2))
- **nv**: a red app test run prints the tests that failed, not its last lines ([`74495519`](https://github.com/novis-lang/novis/commit/74495519c705470d07289653c5bd858b1375e88d))
- **nv**: a red differential suite run saves its whole output and names its failing cases ([`d48e25fc`](https://github.com/novis-lang/novis/commit/d48e25fc13b476fe7074d944a235e67aee33f2ec))
- **nv**: a sweep empties the discard pool after its covws build ([`aa61ff87`](https://github.com/novis-lang/novis/commit/aa61ff870947670507ba026a1e04de80d1a1835e))
- **nv**: ci --push merges origin's new commits, and a push it cannot make pauses the loop ([`23312c6b`](https://github.com/novis-lang/novis/commit/23312c6bb7cadd37d6afd957a44f851ddeef16a6))
- **nv**: site --roadmap and its retire hook sort the themes by chain order, so a goal end's push keeps the order green ([`b6599884`](https://github.com/novis-lang/novis/commit/b6599884227810d134ed1c49361b70fc89ada307))
- **nv**: start-cost counts with the pipeline's debug build, never a hand build ([`67bda0c8`](https://github.com/novis-lang/novis/commit/67bda0c8c09debd26f61dc1092bf602a78a6b385))
- **nv**: the sweep's cargo queries stop inside the hook's timeout ([`ac8eb6c8`](https://github.com/novis-lang/novis/commit/ac8eb6c878f8e9aadaadb22ca9ec3c5bbc003514))
- **proofs**: a proof: test program reads its directory's nvs.toml ([`f331b121`](https://github.com/novis-lang/novis/commit/f331b121b1cb2615e3d1907f77ce94b85361a5fb))
- **registry**: the backup tests write their copies whatever another test copied ([`4ea75226`](https://github.com/novis-lang/novis/commit/4ea7522647f775a0a4a29cf7957ff0277b4cc443))
- **rereads**: a file handle, a process or another cache class between two reads stops W1035 ([`536c8551`](https://github.com/novis-lang/novis/commit/536c8551f826a63d46c1f7c317831f8b6d560490))
- **runtime**: a child's context box is freed inside its tree's turn ([`52839b8e`](https://github.com/novis-lang/novis/commit/52839b8e9103add01ac14190cd24d85e14705094))
- **runtime**: a dump of an ObjectMap or ObjectSet keyed by string or int shows its keys ([`e66698da`](https://github.com/novis-lang/novis/commit/e66698da29e07e98f2c9157fceda9144171d42a1))
- **runtime**: an immortal string's handle may read its payload ([`4c82a240`](https://github.com/novis-lang/novis/commit/4c82a2409afc7638fb840aa4b4288097ccad0fbd))
- **runtime**: an owning member's borrowed entry holds its subject before the callbacks run ([`6e3f5185`](https://github.com/novis-lang/novis/commit/6e3f51856b0507dced4d09e5df1828a07e6be0a6))
- **runtime**: echo checks a string's UTF-8 at the sink again ([`2b7fafda`](https://github.com/novis-lang/novis/commit/2b7fafda84ca3c9c1fea66b78cb67dc03f011321))
- **runtime**: one ASCII byte of text is a shared immortal string ([`23a4aa7a`](https://github.com/novis-lang/novis/commit/23a4aa7a254d85ceed826862274613cecd1fcf1c))
- **runtime**: the lent and owned array primitives and the lend trap deny a panic in their bodies ([`e30622eb`](https://github.com/novis-lang/novis/commit/e30622ebce998df9db1fa74a6297278cddfce76d))
- **server**: a served hello request makes 25 allocations on every platform, and each finds its row ([`a4dcfc92`](https://github.com/novis-lang/novis/commit/a4dcfc929e0769724cbfbc45a5dc963d7ecdcec1))
- **server**: an unknown host from this machine is a logged 421 in development, and development_hosts admits a proxy ([`5c0aad1f`](https://github.com/novis-lang/novis/commit/5c0aad1f11e5cfd71be7e34b2a9bc98d7fa1b6a9))
- **server**: the development host check is formatted, rendered and has its search questions ([`17769d00`](https://github.com/novis-lang/novis/commit/17769d00688d2d3e01ffe78f22acc4506fe147e4))
- **session**: a Core\Html::later slot reads the request's session, and opens it on its first use there ([`5c3f087e`](https://github.com/novis-lang/novis/commit/5c3f087ed9dd06c1967122750621cfb71fab5721))
- **site**: `Storage::delete`'s page questions are stamped for its edited example ([`7a419a1d`](https://github.com/novis-lang/novis/commit/7a419a1d02a3aa4cf30aaf4f026137f151b1d044))
- **site**: roadmap themes follow the chain, with request work after recompile-leaks and lsp-memory ([`c9a7959a`](https://github.com/novis-lang/novis/commit/c9a7959a689a0b2d8bf903c26caa28e2ea15c6e6))
- **stdlib**: Core\Arr::mapKeys holds one copy of each distinct key it names ([`bf7a4115`](https://github.com/novis-lang/novis/commit/bf7a41159881fe663ff77c573090641bf9451915))
- **stdlib**: Core\Ast has a class for the attribute declaration node ([`fa0e6ea7`](https://github.com/novis-lang/novis/commit/fa0e6ea768a858b5020861d677bc5c470f9abd12))
- **stdlib**: File::readLine keeps its buffer and reads a long line a whole block at a time ([`9a52242a`](https://github.com/novis-lang/novis/commit/9a52242aba7fa1ade1307db32a8b0601d45595f9))
- **stdlib**: RunResult, LogRecord and SentMail have capability and extension rows ([`d09b02d7`](https://github.com/novis-lang/novis/commit/d09b02d79503b0c8cf292474284bd80349863a39))
- **stdlib**: a call to a member's owning entry is traced under the member's name ([`aabba46a`](https://github.com/novis-lang/novis/commit/aabba46a3e1ab248bbd16b256e21896a5dd1db91))
- **stdlib**: a string position walks only to itself, from whichever end it counts ([`51ba0c4b`](https://github.com/novis-lang/novis/commit/51ba0c4bf65cfe6d3dcd7501ce8f9480e684b2d4))
- **stdlib**: padStart and padEnd throw when the allocator cannot hold their run ([`ecc6d5ab`](https://github.com/novis-lang/novis/commit/ecc6d5abd994756b1707f0ef030e1a9aa4a9f242))
- **stdlib**: the XML source test builds the class descriptors before it measures a request ([`e5a100b1`](https://github.com/novis-lang/novis/commit/e5a100b18f762d06595ed3e0d8c76f7aec15d4fa))
- **stdlib**: the strings-in-place gate is green ([`0e634e80`](https://github.com/novis-lang/novis/commit/0e634e80c20df58bf1ba4c7dd1528c5850b22273))
- **test**: Core\Test\LogRecord::fields keeps the order the fields were written in ([`f27de799`](https://github.com/novis-lang/novis/commit/f27de79971cc43aa29f41c3a223daf4dd7215a33))
- **test**: a method reference passed as a value finds its route under nvs test ([`3304eedd`](https://github.com/novis-lang/novis/commit/3304eeddb9d9552f68eccf3214b7847e11643040))
- **test**: four gate checks read the tree as it is now written ([`cbcfc89e`](https://github.com/novis-lang/novis/commit/cbcfc89ee3e0ced51c84c968e6e74336aad52d25))
- **test**: the matrix readers and the component cache pass the goal's gate ([`1d5c5195`](https://github.com/novis-lang/novis/commit/1d5c5195a4b955ed7c5e6f7de8ffd2f46be8ecad))
- **test**: the new Core\Test classes say what their slots hold and the unit tests know them ([`cae95007`](https://github.com/novis-lang/novis/commit/cae95007e41610bce762c62826918e084fb9a3a2))
- **test**: the reload command tests use a data folder whose endpoint fits a macOS socket path ([`fd7f0cd1`](https://github.com/novis-lang/novis/commit/fd7f0cd171eb56b8ded6fb20d94b29c8e9d077fe))
- **tests**: the suites match the attribute declarations, and two tests no longer race ([`ea956ecb`](https://github.com/novis-lang/novis/commit/ea956ecb3a423a2fe14bee3ad1b2c953171845ad))
- **time**: a written zone name is looked up as Zone::of looks it up, case ignored ([`dd0d61a2`](https://github.com/novis-lang/novis/commit/dd0d61a242109c16306d5241f5bfbcacbf701e22))
- **types**: `is property<T>`, `is propertyPath<T>` and `is string<K>` are E0867 and no longer crash lowering ([`fb1025ac`](https://github.com/novis-lang/novis/commit/fb1025ac37087f28c05dc51f6f7fdc1d3dbf2c3b))
- **types**: a call or property access through mixed is held to a restriction ([`4a57b10d`](https://github.com/novis-lang/novis/commit/4a57b10d17f72fb7f9361c4b0d0de92822336883))
- **types**: a call through a native class or interface stays virtual ([`5ad76267`](https://github.com/novis-lang/novis/commit/5ad76267b5585b0eb308698774fcba02e5a6d1ac))
- **types**: a keyed property access is a use of every property its key may name ([`981359cf`](https://github.com/novis-lang/novis/commit/981359cfb41fe42f56393d89fb1f47b7d91378a7))
- **types**: a literal in an attribute payload keeps the single-value type its field names ([`4708c293`](https://github.com/novis-lang/novis/commit/4708c2935a9b72d072d8caf7d59562f60ef6a91c))
- **types**: a restriction's error names its manifest relative to the project ([`ec80faf6`](https://github.com/novis-lang/novis/commit/ec80faf6f48a55b5c53afc6dabd87e536c66a69b))
- **types**: a retrieval of an inherited method reads the declaration its parent wrote ([`f8c7d5ee`](https://github.com/novis-lang/novis/commit/f8c7d5ee758e5f3c2658f427907dffc4c73bc535))
- **types**: a second #[Json\Field] on one property is E0763 alone ([`f632eefe`](https://github.com/novis-lang/novis/commit/f632eefe611cacce1e3ba894a1b9307216025e43))
- **types**: a secret passed to setShortLived or setLongLived does not compile ([`50244a84`](https://github.com/novis-lang/novis/commit/50244a8447a87c3e713dfeec9d26a9afc4c0484b))
- **types**: a tainted map key and a two-argument interface get help that names them ([`26d8821b`](https://github.com/novis-lang/novis/commit/26d8821bc2ac0f836f1dda24e0220c6ab9ff8975))
- **types**: an attribute declaration lowers and autoloads, and stage 3's nine cases are green ([`ce5c1663`](https://github.com/novis-lang/novis/commit/ce5c1663c36704a187914eefcdda6624d018bd05))
- **types**: the attribute diagnostics name declarations, never shapes or aliases ([`2542b753`](https://github.com/novis-lang/novis/commit/2542b753017575e9bc3b45ddcc5dd293a2bb5988))
- **vscode**: the Test Explorer lists and runs a project folder as one program ([`f00b7e7d`](https://github.com/novis-lang/novis/commit/f00b7e7d3e1a416ebe4d063bd323579d31c3fc0e))

#### Performance

- **attributes**: the route-target bench has its budget share, and the types and attributes chapters are measured again ([`3255b345`](https://github.com/novis-lang/novis/commit/3255b34541f0e9a65881f0d0ea7847265aa46e31))
- **bench**: ObjectMap and ObjectSet benches measure a string-keyed map and a loop start ([`dc86057c`](https://github.com/novis-lang/novis/commit/dc86057cc0f9afc034b9f5b885074b83538ee86b))
- **bench**: a keyed map loop is measured beside an array loop ([`271663e6`](https://github.com/novis-lang/novis/commit/271663e6700cb16e9f47183bd36b4ed20519b30c))
- **bench**: a loop start over an ObjectMap or ObjectSet is flat in the collection's size, at seven allocations ([`fe4baee1`](https://github.com/novis-lang/novis/commit/fe4baee1db5f6e290847acdb2a62de1c0dbc0e42))
- **bench**: the ObjectMap, ObjectSet and Heap records are measured on the keyed stores and the shared loop ([`d137a713`](https://github.com/novis-lang/novis/commit/d137a713bf19a1cdebb94d1bcd48797bc80503fc))
- **bench**: the comparison's cases and feature rows are recorded again at 44aad66b90 ([`fe890525`](https://github.com/novis-lang/novis/commit/fe89052504176f744a810d94a668da4cb3a8c813))
- **callable**: a Core\Arr callback the checker proved is called without the check of each call ([`762c7202`](https://github.com/novis-lang/novis/commit/762c720290ab9ac29407f57dd09d051d4eacb0be))
- **cli**: a cold run compiles its artifact's object on a scoped thread ([`73d88d37`](https://github.com/novis-lang/novis/commit/73d88d3766e9ab36b9d86e351c575a2d3d545b37))
- **cli**: an artifact is written without a sync, and a torn one is deleted ([`1f84d10f`](https://github.com/novis-lang/novis/commit/1f84d10fde9745624f1774c0adbbd727c6b242df))
- **cli**: nvs run starts its revalidation thread with the first compiled path ([`722cdbc0`](https://github.com/novis-lang/novis/commit/722cdbc00159c351b3c7e7afa0e646939fe53688))
- **cli**: one start is counted by phase, and bun nv start-cost measures it ([`33f3dfb6`](https://github.com/novis-lang/novis/commit/33f3dfb622c104af16f780e2ee8010fdc89c2c50))
- **codegen**: a release that leaves the count above zero is inline ([`a9eedffe`](https://github.com/novis-lang/novis/commit/a9eedffeaa465fff076706de77b82449ea3ac688))
- **codegen**: a scalar test, a widening, a string `==` and a class test answer inline ([`32223460`](https://github.com/novis-lang/novis/commit/32223460f4a99d041b842fecce3c8f742b93cec2))
- **config**: a configuration file is parsed once ([`20662ee5`](https://github.com/novis-lang/novis/commit/20662ee57be69baa65776048b5c4713971b56488))
- **config**: the data folder's nvs.toml is read once per start ([`258fd0a9`](https://github.com/novis-lang/novis/commit/258fd0a9276be4ec6dc52ca2cc78e4d4a30418fc))
- **db**: a MySQL executeMany writes its executes in windows of 64 KiB ([`6b777305`](https://github.com/novis-lang/novis/commit/6b777305fbe84076f4079d1afc9e1d3a052d2584))
- **db**: a MySQL pool reset sends its four commands in one write ([`186cf283`](https://github.com/novis-lang/novis/commit/186cf2834c6a9a7c8df016786505fa2b522592f2))
- **db**: a SQLite pool reset prepares its reads once per connection ([`29e77d4a`](https://github.com/novis-lang/novis/commit/29e77d4a4a0731fd0f0a3349ecca68ac3d04a26e))
- **db**: a block's slow_query threshold is parsed once per snapshot ([`375f114e`](https://github.com/novis-lang/novis/commit/375f114efcf69d32f5b44a939c04d1d3c018999f))
- **db**: a result's rows share one set of keys, built in one row loop ([`f7b2e9ac`](https://github.com/novis-lang/novis/commit/f7b2e9accfd58d6952926f745fedf1b11c00a8fc))
- **ext**: a component's Novis source is shared, not copied ([`0d543a41`](https://github.com/novis-lang/novis/commit/0d543a41e64feac7fc1201007613b0f4f4c4d301))
- **ext**: an extension resolves its imports once, and its cached artifact is checked once ([`cf007e29`](https://github.com/novis-lang/novis/commit/cf007e29b5caeed0860214bd1ae798e0db1c9b1b))
- **hir**: a written name is split into segments in one place, with no copy ([`51cd83bd`](https://github.com/novis-lang/novis/commit/51cd83bdbb395a304f36cb4851bc4cee4b4c0a0e))
- **host**: every TCP stream sends without Nagle's delay ([`07c5438b`](https://github.com/novis-lang/novis/commit/07c5438bcae60d5019bd07daa8e3e5bdc7919522))
- **host**: the slice thread starts with a scheduler's second task ([`0a2cf124`](https://github.com/novis-lang/novis/commit/0a2cf1246df50d102219137dba93df42ec648609))
- **html**: a Markup composition and an escape write their text straight into the result ([`4d6ab125`](https://github.com/novis-lang/novis/commit/4d6ab125f51a838ad260c23321c7e969b38ccec5))
- **http**: a client request with a small held body is one write ([`9c9dd375`](https://github.com/novis-lang/novis/commit/9c9dd37578740f0a360975e57062ec612e7b5b45))
- **image**: wrapped text costs its length, and each shaping plan is compiled once ([`3ee59f2a`](https://github.com/novis-lang/novis/commit/3ee59f2ac005b1a87b700b54db7564f4304202a8))
- **ir**: an element of a local array is lent to an owning call ([`db9e5bed`](https://github.com/novis-lang/novis/commit/db9e5bedb03c56383d92ddd28c0f9d40ddd185d0))
- **json**: a decoded object is constructed from its argument vector, with no second copy ([`a39508bc`](https://github.com/novis-lang/novis/commit/a39508bcac7890cce5363d17d54c798cf9e83f9c))
- **json**: a parse counts its deadline poll in a plain integer, and jsonAs checks its codec once ([`cbd0ccf3`](https://github.com/novis-lang/novis/commit/cbd0ccf37e817bb01e8c739b784938520127294d))
- **json**: a typed decode finds each constructor once and reuses each object's argument vector ([`570645e8`](https://github.com/novis-lang/novis/commit/570645e8fb7301b91b30a12b1547792e2297037a))
- **json**: a typed decode keeps its spare constructors and argument vectors inline ([`264af5f1`](https://github.com/novis-lang/novis/commit/264af5f1d56efa0eb8204a4f29a8b6826ded7521))
- **json**: a typed decode matches a declared key on its bytes, from where the last match left off ([`4d85a34a`](https://github.com/novis-lang/novis/commit/4d85a34a92d625a63943e42d23147120b08c14d7))
- **lsp**: a cold index plans on every worker and checks each body once per graph ([`2322cffa`](https://github.com/novis-lang/novis/commit/2322cffa620fb9b1f888ecbb2e80f4c8d1bbcf95))
- **lsp**: a completion lists the index's types once, off the stored entries ([`c6e361b6`](https://github.com/novis-lang/novis/commit/c6e361b673eb4c8f9716c59277945108556256f5))
- **lsp**: a document resolves its path once, when it opens ([`cdccbac6`](https://github.com/novis-lang/novis/commit/cdccbac6f5e79c5b5f701018f1c7c65a63efae97))
- **lsp**: an analysis checks the bodies of the files it will own and no others ([`b3225828`](https://github.com/novis-lang/novis/commit/b3225828aad2943e35b5d74be511247ad21d69a0))
- **lsp**: every worker reads the server's one parse of each file ([`16e3ae7e`](https://github.com/novis-lang/novis/commit/16e3ae7ee2e321e1cd79df0123d5b168205ed944))
- **lsp**: inlay hints walk only the statements the asked range reaches ([`04303f1e`](https://github.com/novis-lang/novis/commit/04303f1e7d3ca8cc4172dceb2c67ac25775a3514))
- **lsp**: the parse cache resolves each path once until the tree changes ([`682c5f7d`](https://github.com/novis-lang/novis/commit/682c5f7da86ba8b7e64c67187d3090ecc2b5b146))
- **lsp**: the workers split one graph's bodies and check each file once ([`55244b39`](https://github.com/novis-lang/novis/commit/55244b39d3f646f41ea925e227ba26688bcba83e))
- **mutate**: judging mutants lowers the program once per round, not once per mutant ([`5a40aa7f`](https://github.com/novis-lang/novis/commit/5a40aa7f2e98b18a798a955baff7d536404e5d8e))
- **nv**: a case batch is extracted while the next one runs, and recorded in one transaction ([`86b921d4`](https://github.com/novis-lang/novis/commit/86b921d42e6acd8ad4467cff89a96fe18aaab227))
- **nv**: a recorded atom's functions come from one profile process ([`4757d8b1`](https://github.com/novis-lang/novis/commit/4757d8b1c5781d804743cabf337c73aadc60f742))
- **nv**: a recorder makes its compile-cache parent private once ([`f9e8ce63`](https://github.com/novis-lang/novis/commit/f9e8ce63e1c3a5ac77cdab79ccfae48aa82f4d02))
- **nv**: a selection reads the tree once ([`bbaf5c4c`](https://github.com/novis-lang/novis/commit/bbaf5c4c35e00832c064779ecbb55cbe474a215c))
- **nv**: the pre-push checks run side by side and once per tree ([`0747c9ac`](https://github.com/novis-lang/novis/commit/0747c9ac1fdf8a599ccfd87634a5eefa5be2a0e0))
- **nvs-diagnostics**: the code test scans the test text once ([`6262e95c`](https://github.com/novis-lang/novis/commit/6262e95c3ae3aed965802d329ee02fc2573315b3))
- **nvs-ext**: the built-in component tests share compiled components on disk ([`2a3239bb`](https://github.com/novis-lang/novis/commit/2a3239bbd6c29c285db843cd171c1aba9bac6dae))
- **nvs-syntax**: the prefix sweeps share the corpus worker pool ([`a92fc321`](https://github.com/novis-lang/novis/commit/a92fc321cc79752ba0d88ec706f9d066b47fe4be))
- **proofs**: 243 members whose source changed in this gate are measured again ([`e02c9daf`](https://github.com/novis-lang/novis/commit/e02c9daff775ef931fd9a3c189c41ee955d182ba))
- **proofs**: the 129 records the chain made stale are measured from the release binary ([`cde8d52e`](https://github.com/novis-lang/novis/commit/cde8d52ef8f9aa4accdd095b5edcb54316da5b3b))
- **proofs**: the statements and iteration features are measured again, and llms-full.txt matches its sources ([`9507cbb3`](https://github.com/novis-lang/novis/commit/9507cbb39d0f554b8ca3a61f812a6197eb784a78))
- **proofs**: the twelve Session members are measured again on the lent-session source ([`f30c0ebb`](https://github.com/novis-lang/novis/commit/f30c0ebbfd8072a375bb36da45711a55e3d60bc4))
- **queue**: a busy worker claims again without asking the roster ([`4c25a1eb`](https://github.com/novis-lang/novis/commit/4c25a1eb19b4ad6d4facd60b2a27fef78d9ba88e))
- **ratelimit**: a shed entry lives as long as its window ([`18ecf7d7`](https://github.com/novis-lang/novis/commit/18ecf7d789aff0cc66109c18d604e155b6b5827a))
- **reference**: the testing chapter's features are measured again ([`6c26fa89`](https://github.com/novis-lang/novis/commit/6c26fa8963a497ac51ca794010a6abb90e430e03))
- **reflect**: every Core\Reflect member is measured after the parent change ([`ec0d2880`](https://github.com/novis-lang/novis/commit/ec0d28802d9037f668fd9ffe0de5564e9bd8134f))
- **router**: a route's repeated query keys are counted in one pass ([`7af70cb3`](https://github.com/novis-lang/novis/commit/7af70cb3b9c4677cd4c5d6ce2c05b99a3e0d6df0))
- **runtime**: a file's media type is looked up without an allocation ([`eb69110d`](https://github.com/novis-lang/novis/commit/eb69110d9c75558991bf3fae982a81b31f8a96ec))
- **runtime**: a freed graph keeps its first eight waiting containers on the stack ([`9e56258c`](https://github.com/novis-lang/novis/commit/9e56258c0372156ac99b003e4110c16f8f3b0221))
- **runtime**: a spawned task shares its request's tree, deadline and origin ([`b8916acf`](https://github.com/novis-lang/novis/commit/b8916acf8e7bc7e469be107899c4ed3d1ff7ecab))
- **runtime**: a typed array check reads the tags the array was written with ([`ec7eb683`](https://github.com/novis-lang/novis/commit/ec7eb683fb1851c38475f927d7b5bf607341569d))
- **runtime**: an integer key on a hashed array allocates only the key it keeps ([`d32f6b5e`](https://github.com/novis-lang/novis/commit/d32f6b5e657faf99224e4e6aa0675b5d1f19924c))
- **server**: a hello request makes 25 allocations, and each has a row that says why ([`831705be`](https://github.com/novis-lang/novis/commit/831705be88f42c40ea6b6d33afb2f31947a1d259))
- **server**: a small static file is one blocking job, and a 304 opens no file ([`e151bb2b`](https://github.com/novis-lang/novis/commit/e151bb2bc01313b507b8e61bd2105ba3dcfb71c9))
- **server**: steps 3 and 4 look up a remainder once between them ([`37155af1`](https://github.com/novis-lang/novis/commit/37155af12efeed4074150489f4b7d9ac5caca7e0))
- **server**: the door tests the snapshot's debug store flag before it builds a Request ([`ccaaa617`](https://github.com/novis-lang/novis/commit/ccaaa6173e35e739283c2aad22a17118b46c20c2))
- **server**: the head scanner steps byte by byte only through a line that can frame a body ([`83eff1af`](https://github.com/novis-lang/novis/commit/83eff1afb2a6140890bb507c0e0890397faf9e9b))
- **server**: the two stream cells are made only for a request that runs a program ([`4a83ceaf`](https://github.com/novis-lang/novis/commit/4a83ceafa4d6762189cd204a6bf8e8697215c3e9))
- **session**: the session benches are recorded again, and get and remove are constant ([`fd22d06c`](https://github.com/novis-lang/novis/commit/fd22d06c6e8a606a37ab7a4407a5637f0f6996b4))
- **start**: `nv start-cost --floor` measures the Linux start floor, and the size's share is recorded ([`cd1ab688`](https://github.com/novis-lang/novis/commit/cd1ab68876fa857f83c993918dcefdbe1895d6e5))
- **start**: the built-in manifests are shared as `Arc<Manifest>`, not copied per start ([`ad9622e1`](https://github.com/novis-lang/novis/commit/ad9622e1883bea12cb297f06750c144ec18f547b))
- **stdlib**: BigInt compareTo, neg and abs work on the stored magnitude ([`2be66fe2`](https://github.com/novis-lang/novis/commit/2be66fe2216952a3f546f99baf484abf5f47d0e6))
- **stdlib**: Compress reserves room for its output before it encodes or decodes ([`24381f90`](https://github.com/novis-lang/novis/commit/24381f90b05f84843d154454628ea3ceddf0f0c1))
- **stdlib**: Compress::compress starts its output empty again ([`f736ff90`](https://github.com/novis-lang/novis/commit/f736ff90c013efdb9a8cf29048480db9348d15fb))
- **stdlib**: Core\Json::isValid reads the document and builds nothing ([`fdf2fedc`](https://github.com/novis-lang/novis/commit/fdf2fedc1edd2a7142e767aa571f45aef85074dc))
- **stdlib**: File::readLine reads ahead into a buffer kept beside the open file ([`d8e98e00`](https://github.com/novis-lang/novis/commit/d8e98e00ec4ab676355dded9ae42c1f6e83b1b41))
- **stdlib**: File::readLine zeroes its read-ahead buffer only when it grows ([`d9908516`](https://github.com/novis-lang/novis/commit/d9908516e15295218bd4fe36ede282e8bbed7c43))
- **stdlib**: Net Stream::read and Datagram::receive read into a buffer per thread ([`880c03a2`](https://github.com/novis-lang/novis/commit/880c03a25bae124a914693cdc0f68b2c65105893))
- **stdlib**: Regex::replace and replaceWith reserve the subject and return it on no match ([`8206ea76`](https://github.com/novis-lang/novis/commit/8206ea761c9a5ba715597829317ea7b65194f62f))
- **stdlib**: Str::trim returns its argument when nothing is trimmed ([`b60d85d2`](https://github.com/novis-lang/novis/commit/b60d85d22365f6521b986de2142672efa705f95a))
- **stdlib**: Zip::extract decompresses every entry into one block ([`9aea54f0`](https://github.com/novis-lang/novis/commit/9aea54f0ede3d0142ec7181b7422527a6f742738))
- **stdlib**: a Core\Request body read allocates no error name unless it throws ([`04a65b6c`](https://github.com/novis-lang/novis/commit/04a65b6c2fffafa89f1d0b02cfee8d501e131a0e))
- **stdlib**: a collection loop's cursor allocates no empty pairs array, back to seven allocations a round ([`5fd64fc1`](https://github.com/novis-lang/novis/commit/5fd64fc18197116f97d5f8f19b88923a871c2b78))
- **stdlib**: a keyed CSV row retains its header names ([`5515ef90`](https://github.com/novis-lang/novis/commit/5515ef908900d8da39b26b6e61a733309e965dcb))
- **stdlib**: a loop over an ObjectMap or ObjectSet shares its store and copies it only on a write ([`5c44cd5d`](https://github.com/novis-lang/novis/commit/5c44cd5d63f8eff468edf36d61af0439f4a35839))
- **stdlib**: a reply body is read in 64 KiB steps straight onto its buffer ([`9c61b787`](https://github.com/novis-lang/novis/commit/9c61b787fbee7c07813a57fcfce211fd6a6ec034))
- **stdlib**: a split with a limit and a search that ignores case read only what they need ([`00aa18de`](https://github.com/novis-lang/novis/commit/00aa18de62d9132caa80df2ba1df1a59b5d3041f))
- **stdlib**: an ObjectMap or ObjectSet keyed by a class stores each object by its identity ([`d6dc7a2f`](https://github.com/novis-lang/novis/commit/d6dc7a2fb844fed7a9968cf4be4a52c36df2549e))
- **stdlib**: an ObjectMap or ObjectSet keyed by exactly string or int stores the key as it is ([`e82cfd3e`](https://github.com/novis-lang/novis/commit/e82cfd3e4780dd3b830e5de950c19016c03915bb))
- **stdlib**: append, prepend, replaceRange, slice and sort keep their old code ([`426919d9`](https://github.com/novis-lang/novis/commit/426919d92bca444394ccf314fdecdd6685f796f3))
- **stdlib**: appendAll, padStart, padEnd, overlay, overlayDeep and underlay change a sole array in place ([`742e3626`](https://github.com/novis-lang/novis/commit/742e3626f296a5a3444d4ed0339a59cb08172510))
- **stdlib**: every Core\Arr member and testing feature this goal changed is measured again ([`55d45ade`](https://github.com/novis-lang/novis/commit/55d45ade4e7155e484d27409b64f056bfe28d5a1))
- **stdlib**: replaceRange, filter, withoutNull, unique, diff and intersect change a sole array in place ([`a1982946`](https://github.com/novis-lang/novis/commit/a1982946d14eecfec646b35aa7d452e6a364da11))
- **stdlib**: reverse, sort, sortByKey, values, map, mapKeys and flip change a sole array in place ([`d05a2fea`](https://github.com/novis-lang/novis/commit/d05a2feabb8cff92808510cc57323afe0b864781))
- **stdlib**: the bucket, set and appending array members allocate per result ([`36972375`](https://github.com/novis-lang/novis/commit/3697237546ce28df48098e42c2d2678dc5298c2a))
- **stdlib**: the members this goal changed are measured again ([`37dc67f1`](https://github.com/novis-lang/novis/commit/37dc67f160ff2359d15bb287d4e3fda117400a21))
- **stdlib**: the members this goal changed are measured again ([`4858965e`](https://github.com/novis-lang/novis/commit/4858965ede718777c7f672de1e494da18db35261))
- …and 8 more, in the full changes below.

#### Formatter

- **fmt**: cargo fmt --check is clean over the workspace ([`7d0d7064`](https://github.com/novis-lang/novis/commit/7d0d70641ef3c9ddce813c9a11c0e9d98e3cd855))
- **fmt**: four corpus files and the Time-Date reference block are formatted ([`c49a3e16`](https://github.com/novis-lang/novis/commit/c49a3e165b964d3058272b219b786e7d7e1cbb10))
- **fmt**: http.rs is formatted as rustfmt writes it ([`42ae5e46`](https://github.com/novis-lang/novis/commit/42ae5e46ba9957de91530fdaf82cd9ebfa032512))
- **fmt**: the checker's borrowed walks and File::readLine are rustfmt clean ([`937ae854`](https://github.com/novis-lang/novis/commit/937ae854a51c55eba0feec8acdd638d414146b84))
- **fmt**: the goal's edits are formatted and the comparison page renders its recorded bench ([`bcfeec6a`](https://github.com/novis-lang/novis/commit/bcfeec6ac28dd63eee1fec68036129e835fe0547))
- **fmt**: the tree is formatted and its rendered pages match their sources ([`82a92afe`](https://github.com/novis-lang/novis/commit/82a92afe579ab6e188dfda512e4d2a2ced15d093))
- **fmt**: three files are rustfmt-clean and the types rule chapter matches its fragments ([`4904dff5`](https://github.com/novis-lang/novis/commit/4904dff5cb9902f4481b72f2fc74316c6a54e3b1))
- **fmt**: two test files are formatted as rustfmt writes them ([`cbe10627`](https://github.com/novis-lang/novis/commit/cbe10627b3fdb4c0a7404e6aa19a24c6988a33eb))

[Full changes](https://github.com/novis-lang/novis/compare/v0.0.4...v0.0.5) — 941 commits.

## [0.0.4] - 2026-11-15

### Features

- **autoload**: autoload links and a prefixed discover build the map ([`cf4f6ccc`](https://github.com/novis-lang/novis/commit/cf4f6cccb09f4ca1f5f0650e8c6e078806ee96a8))
- **codegen**: a class reference builds only a class new could make ([`d6b0e1f8`](https://github.com/novis-lang/novis/commit/d6b0e1f87705e03ed0b498e913a033a593a26737))
- **config**: a publish moves a generation and fills its snapshots' limits ([`63ac930f`](https://github.com/novis-lang/novis/commit/63ac930fbe2c0939189d291ba90c4e6be7d02eca))
- **config**: a schedule entry's on key is checked at boot ([`dc7f12bc`](https://github.com/novis-lang/novis/commit/dc7f12bc14e691b197e165b3d626b26675dca87b))
- **config**: config dump lists every default in force, and serve says where log records go ([`811a4c90`](https://github.com/novis-lang/novis/commit/811a4c909439427969db686814572126dc34492b))
- **config**: every [http] key a request may set reaches its response, and set refuses the rest ([`e2dd3a7c`](https://github.com/novis-lang/novis/commit/e2dd3a7c318513d4037e328a355754213e4308da))
- **config**: nvs config check resolves each link to another module, and --mounts prints the table ([`7f8fb3ab`](https://github.com/novis-lang/novis/commit/7f8fb3ab85f10cb922b7ad668b091c95dce056cf))
- **db**: SQLite allows a PRAGMA by name, and the reset clears the generated id ([`532b481a`](https://github.com/novis-lang/novis/commit/532b481a5ef7e88390c8eb3400c7efa9dc35abb2))
- **db**: a MySQL row keeps its packet, and a text cell is copied once ([`e5490ccb`](https://github.com/novis-lang/novis/commit/e5490ccbb0f38267c6bdb8fc646646d58940cc00))
- **db**: a database error's message holds no text the server chose ([`47fd4cc1`](https://github.com/novis-lang/novis/commit/47fd4cc17a2d75f1006d29a2e1115b98dea833fa))
- **db**: a failed connect logs the server's text and throws without it ([`85c7cd68`](https://github.com/novis-lang/novis/commit/85c7cd686d142855f2aeffbb91d2f088116e0877))
- **db**: a statement call that binds no secret keeps the server's text ([`be50013a`](https://github.com/novis-lang/novis/commit/be50013a8524a8a3cd91cde4d55d475700da5517))
- **db**: a table names each kind of session state per driver, and the MySQL and MariaDB reset restores the login roles ([`fbfe1223`](https://github.com/novis-lang/novis/commit/fbfe12236575bf1b0ccfbbf04f5d1604db606dfa))
- **db**: the MySQL and MariaDB reset selects the configured database and charset again ([`e098037e`](https://github.com/novis-lang/novis/commit/e098037edb2b98bb3d1efd60b18d477bfc756561))
- **db**: the PostgreSQL reset discards sequence state, and startup pins standard_conforming_strings ([`4dbc9032`](https://github.com/novis-lang/novis/commit/4dbc9032c4b4d9df9800d14eeb49114f727842c6))
- **docs**: every reader shows @param, @return and @throws as a Core card does ([`f3d3e0a0`](https://github.com/novis-lang/novis/commit/f3d3e0a07f56b49669161bc637151910b107b766))
- **ext**: a request's end is bounded, and a guest call stops at the wall deadline ([`6fee169b`](https://github.com/novis-lang/novis/commit/6fee169b145f8ac1c9c5ac9de7e71ea41d15cbcd))
- **graph**: every crossing lands in one receiving side, its program's classes then Core ([`a0b8df11`](https://github.com/novis-lang/novis/commit/a0b8df11e79c423bc4fee28e455a89a83dc37aa3))
- **hir**: the checker holds @param, @return and @throws to the method they document ([`594c784c`](https://github.com/novis-lang/novis/commit/594c784c2a88b2b4b1211da003946000ec0cb4a6))
- **host**: a connection's isolate is watched, and cpu_time holds for each turn ([`cd95e751`](https://github.com/novis-lang/novis/commit/cd95e751f44deca96c3bb199da0e313e19fdb3f1))
- **host**: a stream whose reads never park meets its bound every sixteenth read ([`47e41b6a`](https://github.com/novis-lang/novis/commit/47e41b6a22c3376f17f8ea5eb8f68a20bcc34e73))
- **host**: a wait that outlives wall_time names its reason ([`79c07bcf`](https://github.com/novis-lang/novis/commit/79c07bcfb3c909723357a545b67fdccb7761f013))
- **host**: the blocking pool skips a job its task abandoned in the queue ([`9c96bd6b`](https://github.com/novis-lang/novis/commit/9c96bd6bb215a9bbe744e14ebc1b05abe4afcae1))
- **host**: the park funnel ends every parked wait at its tree's wall bound ([`9116135d`](https://github.com/novis-lang/novis/commit/9116135d07615a1b09a800ea95b139e46e29c16e))
- **instance**: every Core class with instances says what its slots hold ([`a09f5d40`](https://github.com/novis-lang/novis/commit/a09f5d404fa80ae8b309c178c436706d7de76b1f))
- **json**: Core\Json has its own pull reader, which agrees with serde_json on every saved input ([`a36b94ef`](https://github.com/novis-lang/novis/commit/a36b94efbebe79c9658cda7ca408d6d75eef0066))
- **json**: Core\Json writes every byte itself, escaping each string in one pass ([`2ab1371f`](https://github.com/novis-lang/novis/commit/2ab1371f1660358f5c3c2b6224f37a3cc9851f6d))
- **json**: decode, isValid and Request::json build values from Core\Json's own reader ([`ce60cd45`](https://github.com/novis-lang/novis/commit/ce60cd45d83be35382ea72d5bf2b40150f5811d8))
- **json**: decodeAs fills classes in a loop over Core\Json's own reader, on the task's own stack ([`a59aca99`](https://github.com/novis-lang/novis/commit/a59aca994bbbb038e94ce9d0d64fae67e74cdd72))
- **json**: decodeEach reads through Core\Json's own reader, and the spare stack is gone ([`6c01b4bf`](https://github.com/novis-lang/novis/commit/6c01b4bfc18982f86b90714effdd4c67f9d15351))
- **json**: secret_spans finds secret values in one read of the reader's events ([`d9a98268`](https://github.com/novis-lang/novis/commit/d9a982689a7a2d62b480793162d1dc5ce1a45bf2))
- **links**: a link to an autoload links root is read, checked and lowered ([`2c0e2f3e`](https://github.com/novis-lang/novis/commit/2c0e2f3e2283ee589436ca4f29484a1a68b0dc32))
- **lsp**: a doc tag is offered only where its declaration takes it ([`4591bb30`](https://github.com/novis-lang/novis/commit/4591bb305b78cb88b64648f5e8f777c6d59a0eb9))
- **lsp**: a local variable answers its declaration and every site ([`eeb615d8`](https://github.com/novis-lang/novis/commit/eeb615d87fa3038eb78cca3018d6c9ec32df1a9a))
- **lsp**: a method's parameter is renamed with its @param name and its named arguments ([`77b5a911`](https://github.com/novis-lang/novis/commit/77b5a9119a37293c6d93baa4e1c5905c309bbab1))
- **lsp**: go to definition and hover reach a route of another module ([`6f3bc4e1`](https://github.com/novis-lang/novis/commit/6f3bc4e1e93bf1fb3acdb08420b3fc5c35c13105))
- **lsp**: hover on a method's parameter shows its @param sentence ([`08a22855`](https://github.com/novis-lang/novis/commit/08a228552c1ca793280bef08d0771f9e168fd412))
- **lsp**: the $name after @param is coloured as a parameter ([`924a7175`](https://github.com/novis-lang/novis/commit/924a7175e235b358d09840c0c286c0c29d4616e4))
- **meta**: a derived class carries its JSON schema in nvs meta --json ([`6db3f44c`](https://github.com/novis-lang/novis/commit/6db3f44c8397b8739274a994c0b8cbbecb40ec04))
- **mounts**: a mount block takes canonical, and one function picks the row a link to another module goes through ([`e5140bc6`](https://github.com/novis-lang/novis/commit/e5140bc6aa5b8530c02e85f88a275d589f4a2083))
- **nv**: bun nv ci --green says whether main's tip is pushed and green ([`e25c5d7b`](https://github.com/novis-lang/novis/commit/e25c5d7b008a5785dbe4ae3fb935eae5e4548279))
- **pkg**: every command but a locked fetch refuses a signed file older than seven days ([`e071882b`](https://github.com/novis-lang/novis/commit/e071882bbbb31ad663b1c82585e074e84106c752))
- **registry**: a retraction's log entry waits until the time it shows ([`9144046d`](https://github.com/novis-lang/novis/commit/9144046d5f6491016d29a18f9569119ef87cdfb7))
- **registry**: a token of a lapsed account or one with no verified channel does nothing ([`3c5c5af5`](https://github.com/novis-lang/novis/commit/3c5c5af5c048fa05c430047a8695ae93ed0f7b5a))
- **registry**: an owner sees and cancels a waiting retraction or advisory on the package page ([`49c58478`](https://github.com/novis-lang/novis/commit/49c5847856f55703ee782e1bbbb6e11bc63c34dd))
- **registry**: an owner with a passkey cancels a waiting retraction or advisory ([`d42f87a2`](https://github.com/novis-lang/novis/commit/d42f87a24653a75c0bf33f24ae8ef72ef843474d))
- **runtime**: a call by name checks each argument against its declared type ([`a2784d94`](https://github.com/novis-lang/novis/commit/a2784d94165ecc12524f4e7ce233aa387b0a5356))
- **runtime**: a failed conversion never shows a text operand ([`2beb6bfd`](https://github.com/novis-lang/novis/commit/2beb6bfd89aa5fab961ab42c4e28d8aee65a8953))
- **runtime**: a hooked property write and a bare callable call check the declared type ([`54c03db6`](https://github.com/novis-lang/novis/commit/54c03db685e7b03372e14bc13d13ca2082b71a31))
- **runtime**: a slot check tests what an is test tests ([`3d073952`](https://github.com/novis-lang/novis/commit/3d073952a99aba7e6186caebb5ca4e07db2efa33))
- **runtime**: an erased read or call never returns a secret ([`8bd7bc3a`](https://github.com/novis-lang/novis/commit/8bd7bc3abcde5b9b4e578844a8024c4755e45308))
- **runtime**: construct and forClass ask the one question the copy doors ask ([`00c81fc5`](https://github.com/novis-lang/novis/commit/00c81fc5c152b938cd71df3e6e238c15e4afcc91))
- **schedule**: reload and failed entries run from one process-wide Events ([`7b6cb3cb`](https://github.com/novis-lang/novis/commit/7b6cb3cb8de24b322585d7389458250ef86e231f))
- **schedule**: start and stop entries run, and every run reads one map ([`8823ff30`](https://github.com/novis-lang/novis/commit/8823ff308a55941f2a8b1309de4c664a59efc265))
- **security**: a package sets only limits and restores only what a package set ([`f3965187`](https://github.com/novis-lang/novis/commit/f39651871d53bff3e7c13141b5265e640d373bef))
- **security**: an extension call and a third-party extension's source ask the namespace ([`e91c5111`](https://github.com/novis-lang/novis/commit/e91c5111ed6f56b4b2cf00641e03003dd7fd3a51))
- **security**: every Core class has capability rows or a reason for none ([`cb877e01`](https://github.com/novis-lang/novis/commit/cb877e01725f9ae0121d584b2f1e705e7da00b81))
- **security**: reading the process environment is the capability env ([`7a6e8cce`](https://github.com/novis-lang/novis/commit/7a6e8cce23dcf664b38bcc6d19897f61735b9c2d))
- **security**: spawn script, a queued job and an upgrade ask the calling namespace ([`9c2cd549`](https://github.com/novis-lang/novis/commit/9c2cd549d01848b060ae49344999296279977448))
- **serialize**: decode checks every slot of a Core class it builds ([`a43a74f2`](https://github.com/novis-lang/novis/commit/a43a74f2a96d5204c7656457ee3861b6a29a8d5b))
- **serve**: a link to another module goes through the mount the server picks at boot and reload ([`d8adfe7e`](https://github.com/novis-lang/novis/commit/d8adfe7e456094afb0e853f673e3b912851fbb99))
- **serve**: a served program's built-in components are loaded before the server accepts ([`272945a6`](https://github.com/novis-lang/novis/commit/272945a625f90474962724b8471b6b3743cbc1b0))
- **server**: a request's own [http.headers] and [http.cors] settings shape its response ([`8a96e4d1`](https://github.com/novis-lang/novis/commit/8a96e4d120df1673e3c86e3507ef9563a4c034e1))
- **server**: a streamed response is answered under the request's own [http] settings ([`e72ccefd`](https://github.com/novis-lang/novis/commit/e72ccefd474570e7535914889d1c982b7e9392d1))
- **server**: an event stream takes a max_open place like a WebSocket ([`24aabe02`](https://github.com/novis-lang/novis/commit/24aabe02b85f259fdc891181981cc515779b2465))
- **syntax**: a doc comment parses @param, @return and @throws, and none carries a type ([`ab0916f9`](https://github.com/novis-lang/novis/commit/ab0916f9f74471e8a0ff4524558a79bb22f24d3b))
- **test**: Core\Test::request gives the link to another module that nvs serve gives ([`a2fcae75`](https://github.com/novis-lang/novis/commit/a2fcae75e15275cbb3c5ce3be87249130da91b96))
- **tooling**: a /// TODO: line is a todo and is left out of the docs ([`029c7f6c`](https://github.com/novis-lang/novis/commit/029c7f6c2d7848a7873dc9fb3fb43d41ea693c0b))
- **topic**: a publish encodes once and each subscriber decodes into its own classes ([`566c03a5`](https://github.com/novis-lang/novis/commit/566c03a5bc2ff0bd8b94b424c17d03a16014033f))
- **topic**: a topic name belongs to one application ([`2fccf7d3`](https://github.com/novis-lang/novis/commit/2fccf7d36aeec535c9cf13cef995688ad991dad4))
- **types**: a call or a write with no parameter type refuses a qualified value ([`4841abdd`](https://github.com/novis-lang/novis/commit/4841abdda98d76889b4d7a62ff89792d86925136))
- **types**: a derived codec reads a set of allowed values in both formats ([`64fe687b`](https://github.com/novis-lang/novis/commit/64fe687b9c2bf30c20b54523123758366f38d2d0))
- **types**: a derived codec's fields include every property its parents declare ([`26e0046d`](https://github.com/novis-lang/novis/commit/26e0046dfe218605efc4d1a0148ad3d567c3a263))
- **types**: a member that makes keys from values refuses a secret one ([`ad87963c`](https://github.com/novis-lang/novis/commit/ad87963cb69e416c9b12d5846a08ab8d8a565197))
- **types**: a member that makes keys refuses a tainted key its answer reads back plain ([`8018df2a`](https://github.com/novis-lang/novis/commit/8018df2ac8b22e8f906fcb3077b7de599b34ab4f))
- **types**: a secret value enters mixed only where secret mixed is written ([`c7f8a2e4`](https://github.com/novis-lang/novis/commit/c7f8a2e49723fbb33a6ea44ba59678b8c813244c))
- **types**: a type test and a conversion keep secret and tainted ([`f99e1efc`](https://github.com/novis-lang/novis/commit/f99e1efc0404189c126e0b6ebf9eade644261ca0))
- **types**: one check guards every key position, and a secret is never a key ([`42af288f`](https://github.com/novis-lang/novis/commit/42af288faa8332a5a141723b439d7113d232c423))
- **types**: one matrix tries secret and tainted at every position that erases a type ([`0287a079`](https://github.com/novis-lang/novis/commit/0287a079001e0737845de1daa04826dc8781c89b))
- **vscode**: the grammar colours the five doc comment tags ([`4754ded2`](https://github.com/novis-lang/novis/commit/4754ded25b2daa57c6d96c015c96e7a289d25464))

### Fixes

- **abi-probe**: the wasm probes build with wasmtime's anyhow feature again ([`6b86ab3a`](https://github.com/novis-lang/novis/commit/6b86ab3aca7443b5fbea178b9a7206cc9a22cb7e))
- **cli**: the --events publisher reads the configuration after the run has it ([`bd244308`](https://github.com/novis-lang/novis/commit/bd2443089997386ad25a933eb0534828c01ba093))
- **config**: a breach names a well-known group in English, and the gate's tests send the viewer token ([`c46235be`](https://github.com/novis-lang/novis/commit/c46235be84ece762ba69fb2a6bfefd40dd16b7b6))
- **config**: an inline secret directive leaves the merged table, and every dump prints it <secret> ([`7461bb58`](https://github.com/novis-lang/novis/commit/7461bb583ad56ebb43bbc7a1c67a480dcdec0165))
- **config**: every surface that lists the grants names env ([`1d2aa0f4`](https://github.com/novis-lang/novis/commit/1d2aa0f4ea837eff80c51d2fc24ed2936e713eb4))
- **config**: the Windows ownership check refuses every other account's write entry ([`9bfc5d4f`](https://github.com/novis-lang/novis/commit/9bfc5d4f72bdc18e976ddc4aa65335c37d355ea4))
- **crypto**: KeyPair::read names the key format it was given and the command that converts it ([`89ecacb8`](https://github.com/novis-lang/novis/commit/89ecacb8ac3eeacf0286d6763732241de8258ee1))
- **csrf**: a token writes the session only when it draws the identifier ([`8a93550e`](https://github.com/novis-lang/novis/commit/8a93550e1f0fe2977ab14e120d1941aaceefbb0b))
- **db**: a MySQL row with octets after its last cell is InvalidData ([`df0eb588`](https://github.com/novis-lang/novis/commit/df0eb5880333612107975498a971c163fc031fae))
- **db**: a SQL Server datetimeoffset decodes across 0001 to 9999 ([`c8ada7c1`](https://github.com/novis-lang/novis/commit/c8ada7c14c676027d27b48fad955f54e4a8f6bf8))
- **db**: a SQLite request cannot make the schema writable ([`64bb9542`](https://github.com/novis-lang/novis/commit/64bb95420d6cb21112ef77e59b0fabe76ea88bd7))
- **db**: a column default reaches DDL only as a typed literal ([`37205d79`](https://github.com/novis-lang/novis/commit/37205d79d93739eba348176d70edfa02e4c65db6))
- **debug**: the viewer needs its token on every bind, loopback too ([`17961ca2`](https://github.com/novis-lang/novis/commit/17961ca23d0c062e049c92b603472a2ace3e2f24))
- **diagnostics**: an @param that names no parameter is E0335 ([`d7f03202`](https://github.com/novis-lang/novis/commit/d7f032023650f894194614184151f24a246ecbe9))
- **ext**: a crossing value is counted once, and the pool has a place for every request ([`80edd4a1`](https://github.com/novis-lang/novis/commit/80edd4a16a526e07a68e00c33b2e929fe0cb82d2))
- **ext**: a guest stops at the tick after its request passes the memory ceiling ([`109d8e7b`](https://github.com/novis-lang/novis/commit/109d8e7b6d0bc062c697491618f66e8227ab5e20))
- **ext**: the instance pool has at most 4,096 places, because its table pool commits memory ([`7ec95769`](https://github.com/novis-lang/novis/commit/7ec957692e53284c27628774ba34e91c02446761))
- **gate**: links-between-modules passes verify, the doc links and the proofs word bound ([`ba0f22ef`](https://github.com/novis-lang/novis/commit/ba0f22eff7e674adfa1b24adc7002b286f15d03a))
- **gate**: request-path-per-core's rule chapters, fuzz lock and Csrf figures are current ([`0b6d01cf`](https://github.com/novis-lang/novis/commit/0b6d01cfe8e038ae56a0d221b513e78593b8aa7f))
- **gate**: the rule chapters are current, and two gate tests hold under load ([`78eebf4c`](https://github.com/novis-lang/novis/commit/78eebf4cd1757d611859a87899b8e6c6a1359c86))
- **host**: a rerooted connection task charges its CPU to its new request ([`b4b0bd32`](https://github.com/novis-lang/novis/commit/b4b0bd32894d6defa7671cb2551fd819db907a7a))
- **ir**: a `.` operand or string hole keeps its read while a later operand can free it ([`9df83490`](https://github.com/novis-lang/novis/commit/9df834909e9ea32ce5f62b5a2fa69dd498661a14))
- **ir**: a `Core` receiver and a keyed read's receiver stay alive while later code runs ([`e10b66ec`](https://github.com/novis-lang/novis/commit/e10b66ec8b360c2971d5532134ee0443dbbae868))
- **ir**: a binary operand and a `match` subject keep their read while a later operand can free it ([`cac70bfd`](https://github.com/novis-lang/novis/commit/cac70bfd32d2c6ffab760e2ba237b1ac2c695d6a))
- **ir**: a borrowed call argument keeps its read while a later argument can free it ([`d99711a9`](https://github.com/novis-lang/novis/commit/d99711a9556dd7da0fb8b09ca7750209c6f00fcb))
- **ir**: a rendered format and a guarded element store keep what they borrow ([`c619a1a3`](https://github.com/novis-lang/novis/commit/c619a1a392e1519dacf2b7a1b036858ec595393e))
- **ir**: a spawn option and a route link argument stay alive while later ones run ([`ea673cde`](https://github.com/novis-lang/novis/commit/ea673cdef7525890f5866fba6d540e7ee61a0223))
- **ir**: a stop at a function's entry releases the arguments the callee owns ([`02358b5c`](https://github.com/novis-lang/novis/commit/02358b5c1a073e9d72bbe4ec4cf8059ce9daacd0))
- **ir**: a store keeps its receiver and keys alive while its value runs ([`0d6ddfa4`](https://github.com/novis-lang/novis/commit/0d6ddfa4de1adc428898a9c7522d084668d7f316))
- **ir**: a toString, a property guard and a generator factory keep what they borrow or own ([`ab47e05b`](https://github.com/novis-lang/novis/commit/ab47e05b14cc245065d7585f3cf21aed1463909c))
- **ir**: an element read keeps its borrowed array while its key can free it ([`44642751`](https://github.com/novis-lang/novis/commit/44642751eec7fd58c2e02170fc126e3beb6f4a25))
- **ir**: an inout loop walks the array it began with and writes by key ([`12931269`](https://github.com/novis-lang/novis/commit/12931269e82e56a5a912fef8b68e206ee1efdb0f))
- **ir**: an object ordering keeps its left operand while the right one runs ([`45d528a4`](https://github.com/novis-lang/novis/commit/45d528a42ee0cd77736c570d34d51a4ee03baefd))
- **json**: a mixed field decodes a JSON null, in a document and in a row ([`1d742740`](https://github.com/novis-lang/novis/commit/1d7427408f84d1b1e11d697984fa0d9ff0fe9505))
- **log**: with no [log] target every record goes to stderr and never into a response ([`51994027`](https://github.com/novis-lang/novis/commit/51994027b27c08cef93abc90c5b734a197f0e414))
- **lsp**: a file in an ignored folder finds the program that loads it ([`ee6896ee`](https://github.com/novis-lang/novis/commit/ee6896ee40204172ac80ba0e390c98b1e068f904))
- **meta**: a component's promoted properties have a card, an index line and a place on the website ([`94f422dd`](https://github.com/novis-lang/novis/commit/94f422ddd1b606dec9fec74d1331422f46a71cbf))
- **nv**: a doc comment's tag lines are each one sentence in the comment bounds ([`87e5bf1f`](https://github.com/novis-lang/novis/commit/87e5bf1f007c7df5fb2084f7f91a736f1dc090b5))
- **nv**: the C-dependency ledger drops ittapi-sys, which left the graph ([`3e45f86b`](https://github.com/novis-lang/novis/commit/3e45f86b2749e7bd7603154d1628ebb54d35be38))
- **nv**: the comment bounds keep a doc comment's tags in a typed list ([`668d9ec6`](https://github.com/novis-lang/novis/commit/668d9ec6c717590cbc54d2add2292cc20d7981fa))
- **pkg**: a package carries one package.toml, its own ([`a136ab9d`](https://github.com/novis-lang/novis/commit/a136ab9df1decf3695597131d7deec449ec09a24))
- **pkg**: one function judges every package path, and a git tree entry is one name ([`9d83b478`](https://github.com/novis-lang/novis/commit/9d83b4787e910c21e4cc79af5af7d00f8119ee84))
- **pkg**: the outermost named manifest decides a file's package ([`7760ef38`](https://github.com/novis-lang/novis/commit/7760ef3839934beee358752b007fae9b39cbd40e))
- **proofs**: a promoted property owes no proofs, and the doc-tags pages and perf are current ([`bb03ced3`](https://github.com/novis-lang/novis/commit/bb03ced395b5a4705d1ee80b856224785d9f05fa))
- **proofs**: a with-stderr example's output leaves out the queue worker's warning ([`30ded135`](https://github.com/novis-lang/novis/commit/30ded135751bf2482361f53dac2b5d8069eb3331))
- **proofs**: the perf sweep skips a bench whose service is down and keeps what it measured ([`985570fa`](https://github.com/novis-lang/novis/commit/985570fab7e19cf42120841d062cda100074612b))
- **proofs**: the tree's programs pass typed values where a call has no parameter list ([`12374331`](https://github.com/novis-lang/novis/commit/123743313346ff582daa2db31d376c4edaa83250))
- **registry**: the gate of registry-waits-hold is green ([`904748b0`](https://github.com/novis-lang/novis/commit/904748b0a6bdfe002de5848fdc94795a3a6c326a))
- **run**: nvs run and nvs test resolve a link to another module from any directory ([`aa531b6f`](https://github.com/novis-lang/novis/commit/aa531b6f1fdf0fcbd8df457241e9a77254ea6119))
- **runtime**: `nvs_tasks_in_flight` counts a core's tasks from its first scrape ([`5b5ca564`](https://github.com/novis-lang/novis/commit/5b5ca564782858d43d9569cf1d7c858360a3b682))
- **runtime**: a fault's message is clipped to 8 KiB where it is made, and only abi.rs builds one ([`f4032b3e`](https://github.com/novis-lang/novis/commit/f4032b3eb2633e5ecad2c324db32fe1d2395e857))
- **runtime**: a private member is reached by name only from the class that declares it ([`6aea7973`](https://github.com/novis-lang/novis/commit/6aea7973821145cbccba0901d84971b30b303b46))
- **runtime**: a refused write by name says property and names no internal class ([`61feaf59`](https://github.com/novis-lang/novis/commit/61feaf59b75b6ee1f73394d202a16b035e0dce53))
- **runtime**: a spawned script keeps the ceilings its spawn decided after any Core\Config call ([`ece7e0b6`](https://github.com/novis-lang/novis/commit/ece7e0b6c59664931e1f8ce3b60a527b724718a7))
- **runtime**: a waiting connection leaves the tree its after-response work runs in ([`01e1e0d3`](https://github.com/novis-lang/novis/commit/01e1e0d3c3aba7d5dc22c6e9c4a8ed19c0e87ee4))
- **runtime**: an idle keep-alive connection frees its extension instances ([`24b116ec`](https://github.com/novis-lang/novis/commit/24b116eceea11b449e9cc768f32880336de2a6dc))
- **runtime**: decode refuses every class encode refuses, before it makes an object ([`2c4e7a95`](https://github.com/novis-lang/novis/commit/2c4e7a95110bc2f7b48934514755cccf7731e59a))
- **runtime**: every request gets a fresh tree, so nothing carries over from the last one ([`fece0eaa`](https://github.com/novis-lang/novis/commit/fece0eaa998c28ee3aa3c9f3d01e9c580830f809))
- **runtime**: no array primitive holds a panic a program reaches ([`a1559efe`](https://github.com/novis-lang/novis/commit/a1559efe942175d0ca4609a2fe19ab03bcb0ed21))
- **runtime**: no object primitive holds a panic a program reaches ([`96fb2696`](https://github.com/novis-lang/novis/commit/96fb2696147693ebaeaf4c8029f283b1cea8f992))
- **runtime**: no primitive in the runtime holds a panic a program reaches ([`0cf398a7`](https://github.com/novis-lang/novis/commit/0cf398a7d23e3821bdbfe91d4cc49a2be2e626fb))
- **runtime**: no string, value or arithmetic primitive holds a panic a program reaches ([`ef187958`](https://github.com/novis-lang/novis/commit/ef1879583437c8f120855c12b0b5933a3e172c84))
- **server**: a listener asks the kernel for its longest accept queue, and the flood case waits for late requests ([`a01d7b18`](https://github.com/novis-lang/novis/commit/a01d7b1867e3b7b24deed8c96a84c1cc412ca10e))
- **server**: a new connection waits for its first request under keepalive_timeout ([`022b1487`](https://github.com/novis-lang/novis/commit/022b148763bf7032636d02ca75fdab32d552ad3c))
- **server**: only a message restarts a WebSocket idle timeout ([`b5bcf8fc`](https://github.com/novis-lang/novis/commit/b5bcf8fc9c87a371ea71f28e8dc523fe8eb019b8))
- **session**: a destroyed session stays destroyed on every store ([`f75e0751`](https://github.com/novis-lang/novis/commit/f75e0751c89c28bda021f191d3635c9896f60897))
- **session**: a session row is keyed by its application on every store ([`1d385d85`](https://github.com/novis-lang/novis/commit/1d385d85fb220cbbd6e31dcc37018d59807c0a65))
- **site**: a viewer shot opens the debug viewer's token link first ([`4bd9a5d6`](https://github.com/novis-lang/novis/commit/4bd9a5d643c186440d7bc35601d69bbac3f9f317))
- **stdlib**: Core\Storage::get reads a whole object through the one bounded reader ([`b901a89a`](https://github.com/novis-lang/novis/commit/b901a89ab19acf4c8a11bd9aae5a2d44273d3fc2))
- **stdlib**: a backtracking regex search stops at the request's CPU limit ([`d7d5f970`](https://github.com/novis-lang/novis/commit/d7d5f970ef3bd25f61bc958023ef43131edcae00))
- **stdlib**: a repeated JSON key and a refused header, redirect or cookie quote the caller's text ([`0acc9670`](https://github.com/novis-lang/novis/commit/0acc9670868d1af85ab510682cb701e27f6501ca))
- **stdlib**: a wide Core\BigInt product polls the request's deadline between pieces ([`439f3c5c`](https://github.com/novis-lang/novis/commit/439f3c5cfaecf0251299bc2d97a4644bf9f5d0c7))
- **stdlib**: an HTTP body, a store reply, a file session and a cache entry ask the memory ceiling before they are held ([`aaed0fb5`](https://github.com/novis-lang/novis/commit/aaed0fb59e21287c9970530c4bbca632ea11d8e6))
- **stdlib**: every Core walk over caller-sized input polls the request's deadline ([`b8742450`](https://github.com/novis-lang/novis/commit/b87424507a33e1c0d6c8cc8bfa08174fe96ee43f))
- **stdlib**: every digest, every encoding and the native sort stop at an expired deadline ([`6fd15a5b`](https://github.com/novis-lang/novis/commit/6fd15a5b16076e56be321f24cae5e412574783fc))
- **stdlib**: every key derivation asks the request's CPU time before the first HMAC ([`8fd58f82`](https://github.com/novis-lang/novis/commit/8fd58f82556504b418cbd6186c77c38bedef6e9a))
- **stdlib**: every wide Core\BigInt call polls the request's deadline between pieces ([`f2dd5017`](https://github.com/novis-lang/novis/commit/f2dd5017e6fe201a56f3d9518d8577927b71860e))
- **stdlib**: the source scans read every shipped line of the cache and the compressor ([`49315fe8`](https://github.com/novis-lang/novis/commit/49315fe8eeaea42f5329b0a423034670ffe0a503))
- **test**: a folder walk goes into no package below the named folder ([`00a9c925`](https://github.com/novis-lang/novis/commit/00a9c925f8549bc1a61a834f42159475643aeed7))
- **test**: nvs test runs no test of a package the program loads ([`5a25d0e4`](https://github.com/novis-lang/novis/commit/5a25d0e4f5e4827d68f85e405740af80fe5f35a4))
- **types**: a secret in a literal written as an argument keeps its qualifier ([`c9e13b8b`](https://github.com/novis-lang/novis/commit/c9e13b8b40456d24d181161b7df355137c4473c2))
- **types**: a signature a property declares has its marker class ([`390182cc`](https://github.com/novis-lang/novis/commit/390182cc73ae00053d31b454e1b1c8cb646d538c))
- **types**: an argument's array element that does not fit names its declared type ([`8b11c0a9`](https://github.com/novis-lang/novis/commit/8b11c0a97905270de5ecfe354ca3da2ccf84e284))
- **types**: an extension's mixed parameter refuses a secret, and one mistake reports once ([`a631f0ba`](https://github.com/novis-lang/novis/commit/a631f0ba07e496a513ad8fca3c1cd6b044757421))
- **verify**: the goal's gate finds the rulebook current and two tests hold under load ([`0246cf0f`](https://github.com/novis-lang/novis/commit/0246cf0f3a52b84a57c357d96eb3993a6fda38ae))
- **vscode**: a /// doc comment line has no background, like every other comment ([`7ba67d44`](https://github.com/novis-lang/novis/commit/7ba67d44446d7ab3ed2e0a59242b5408b281d1d1))

### Performance

- **arrays**: a nested write is one write-through that copies only a shared level ([`cfea66d5`](https://github.com/novis-lang/novis/commit/cfea66d50986200c634b7f165114608426f31589))
- **arrays**: an append to a string element grows it where the array stores it ([`37850414`](https://github.com/novis-lang/novis/commit/37850414515380af75c84cee32ff6ca0d0a334df))
- **arrays**: an array or string passed at its last use is moved into the call ([`245594ba`](https://github.com/novis-lang/novis/commit/245594ba56eef1666b75c05e63227d625b57069d))
- **codegen**: a literal's retain and release are not emitted ([`0163636e`](https://github.com/novis-lang/novis/commit/0163636e10d44efc6a0f6e65a19e9769f671b0ff))
- **codegen**: a retain of a string, object or array makes no call ([`f6b8c63d`](https://github.com/novis-lang/novis/commit/f6b8c63df9d1d1822887447709a466682d973c7e))
- **config**: the door's settings are parsed once per snapshot, and a configured log file stays open ([`863dec7a`](https://github.com/novis-lang/novis/commit/863dec7a4cdc574997ade1b1e8aab53094ce5ffb))
- **core**: Core\Arr's perf figures are measured on the key-checking stdlib ([`7afa92ef`](https://github.com/novis-lang/novis/commit/7afa92ef94f1143ea77d9443983e5574d19e689b))
- **ext**: the built-in components are stored with Brotli and expanded only on a cache miss ([`daa31957`](https://github.com/novis-lang/novis/commit/daa319576b9fff2570565f2a125b58077db5e803))
- **json**: a decode keeps its first four open levels inline, and every Core\Json record is at or below its last ([`17b54694`](https://github.com/novis-lang/novis/commit/17b546941bcdb0f4fb244d98322f2646f87a1680))
- **json**: the JSON and attribute features carry figures measured on the new tree ([`8cb9cc61`](https://github.com/novis-lang/novis/commit/8cb9cc613a885ede1333c1c8ba129d6b27b3b4f9))
- **json**: the five Core\Json members that met their declarations are measured again ([`17f37e68`](https://github.com/novis-lang/novis/commit/17f37e68a2bd7c7ef50bdf06e9ef3a374e2b872c))
- **lang**: the type, error and attribute chapters' perf figures are measured again ([`46724419`](https://github.com/novis-lang/novis/commit/467244196b20e018f73ea7b95635e4aebb3a9750))
- **proofs**: Core\Router, Core\Test and lang:programs are measured again after links-between-modules ([`4baaf905`](https://github.com/novis-lang/novis/commit/4baaf9053c9e650cfa644fad5aa289c3feaf01bf))
- **proofs**: every member and chapter the goal changed has a current figure ([`edf72d1a`](https://github.com/novis-lang/novis/commit/edf72d1a3c0ef5786092549cb4dc0f0d009ec0aa))
- **proofs**: the Core\Session and Core\Cache records are measured again ([`1fa991ee`](https://github.com/novis-lang/novis/commit/1fa991eeed5f25bb0a3d8ca2836f95a69f52b35e))
- **proofs**: the benches this goal's files reach are measured again ([`add9c4d4`](https://github.com/novis-lang/novis/commit/add9c4d4006067fe39033ed6533c27bf5687d724))
- **proofs**: the nine Core\Session records are measured, and waits-are-bounded's gate is green ([`921feac5`](https://github.com/novis-lang/novis/commit/921feac5a87162a06442fb94695fb2c3ac39b85b))
- **proofs**: the reflection, test and two chapter groups are measured again ([`986cec9e`](https://github.com/novis-lang/novis/commit/986cec9efb533483fdd84c03ca5c3ea32025767c))
- **proofs**: the statements chapter and Core\Test figures are measured again ([`d27d2ffa`](https://github.com/novis-lang/novis/commit/d27d2ffa2e810d18b993707d875f2836743e41ff))
- **reflect**: ParameterInfo's perf figures are measured again ([`a6abcd8c`](https://github.com/novis-lang/novis/commit/a6abcd8c584338dfd110782893804b0b43c5d824))
- **runtime**: a request head is one buffer, and a rerun shares it ([`49ed6937`](https://github.com/novis-lang/novis/commit/49ed6937fedc4ee04a64ee816d230cf6a099c69e))
- **runtime**: a successful call by name allocates nothing ([`3845845a`](https://github.com/novis-lang/novis/commit/3845845ab3abdc5a335756c55e86df5f07101e02))
- **serve**: a mount's entry is read from its core's slot, with no lock ([`0c2f01a6`](https://github.com/novis-lang/novis/commit/0c2f01a6724c45e21ee1bc19baee55503cee755c))
- **server**: a keep-alive request's shared writes are counted, and held to 48 ([`ae8ae89a`](https://github.com/novis-lang/novis/commit/ae8ae89a509a2d78aa78d5311fa6bbd793dec2d0))
- **server**: a request holds one snapshot reference, and makes 34 shared writes ([`645e2397`](https://github.com/novis-lang/novis/commit/645e239751630d608868fb53ac9d3b7c68167c42))
- **server**: a static file's lookup, open and chunk reads run off the core ([`0c637546`](https://github.com/novis-lang/novis/commit/0c637546b47282868ee34e929e9b1243c063f969))
- **server**: the door reads its core's record of the publish and policy ([`a05a367e`](https://github.com/novis-lang/novis/commit/a05a367ebf3284f29bb0fffb444998d69efcee2f))
- **stdlib**: every member whose source the goal changed is measured again ([`9a2c2c22`](https://github.com/novis-lang/novis/commit/9a2c2c22e41cb2714548f9a28f17a544c7a8fb02))
- **stdlib**: the members whose doors changed are measured again ([`dc6c1b18`](https://github.com/novis-lang/novis/commit/dc6c1b183122b48e85eebff9acd834b19b35f250))
- **strings**: an append of an interpolation or a chain writes its parts into the target ([`104f784e`](https://github.com/novis-lang/novis/commit/104f784e81f17143849f327e9833cbb9dce043fc))
- **strings**: an append to a plain property or static property grows its string in place ([`434ba14d`](https://github.com/novis-lang/novis/commit/434ba14d4c277e96031ceee44333508d9a3d1abc))

### Formatter

- **fmt**: nvs fmt writes a doc comment's tags in one order, and a doc tag error never stops it ([`ed6b0ba3`](https://github.com/novis-lang/novis/commit/ed6b0ba31b9154d5a5fb4c47e60bf0f79aaf9135))
- **fmt**: the derive examples, fixtures and packages keep the formatter's layout ([`0ed6b5c0`](https://github.com/novis-lang/novis/commit/0ed6b5c034de5c6bfff280b893dc1813e1223505))
- **fmt**: the goal's hostile cases are formatted and the statements chapter is rendered ([`39c9949f`](https://github.com/novis-lang/novis/commit/39c9949fd65cb8c9f4a9bcf40f9888858200592e))
- **fmt**: the lowering and the database row module are rustfmt-clean ([`dfbab53a`](https://github.com/novis-lang/novis/commit/dfbab53aa4e6b62713ba04d40e214f41fc0922fd))
- **fmt**: the slot-type files are formatted as rustfmt writes them ([`f8389632`](https://github.com/novis-lang/novis/commit/f8389632f7dd3a2fb9338845137aae38f044808a))
- **fmt**: the tree is formatted by rustfmt again ([`5293f75f`](https://github.com/novis-lang/novis/commit/5293f75fa089dff4916c1a6e1dda0dcee56a14ea))

### Internal

321 further commits: docs (201), test (59), chore (56), refactor (2), build (2), wip (1).

[Full changes](https://github.com/novis-lang/novis/compare/v0.0.3...v0.0.4) — 524 commits.

## [0.0.3] - 2026-11-10

### Features

- **apps**: apps/chat shows WebSockets and apps/poll shows server-sent events, each with its website page ([`13159de2`](https://github.com/novis-lang/novis/commit/13159de2aca6234e650a6a17e420e3a07b2a664f))
- **apps**: chat deploys with nvs.toml and one production.toml, as poll does ([`c9797093`](https://github.com/novis-lang/novis/commit/c97970936dd12510217e10e3c3a3a6664aad44a4))
- **apps**: poll's security policy is set in nvs.toml, for every response ([`082b8053`](https://github.com/novis-lang/novis/commit/082b8053758492fbd4216bcd44c00ebc3574ea6e))
- **ci**: a real release needs a green full CI run of main's tip, not a dry run ([`9c43636c`](https://github.com/novis-lang/novis/commit/9c43636c5d7c4e821c33e0a09fa338b3f37fe694))
- **cli**: NOVIS_PARSE_ONLY makes nvs check its command line and exit 0, running nothing ([`8f8f4da5`](https://github.com/novis-lang/novis/commit/8f8f4da5412aeffdffe10c87fd21be2eb80fbd2f))
- **cli**: a request file's body after --BODY-- may hold any bytes ([`41677edd`](https://github.com/novis-lang/novis/commit/41677edd59ec58970e368a96c8c8623985aca67a))
- **config**: an owner in Administrators passes the ownership check on Windows ([`13c4cc74`](https://github.com/novis-lang/novis/commit/13c4cc743c30b36ad4d2db0fc81d305954c97ef3))
- **config**: nvs init sets up the data folder and the project file, and offers to restrict its folder ([`bfff166e`](https://github.com/novis-lang/novis/commit/bfff166e7f03f93ddfb05181869568d274c75189))
- **config**: the folder above a checked folder fails only when another account can replace it ([`d6752d1a`](https://github.com/novis-lang/novis/commit/d6752d1a6d12fb82679bb11e146aff83c1e44d65))
- **csv**: Core\Csv::rows decodes a charset one chunk at a time ([`3db03f8a`](https://github.com/novis-lang/novis/commit/3db03f8ad0f77cc2b88e4a3b07e248dd752a7fdf))
- **csv**: Csv::rows reads any byte source, and its cells follow the source's taint ([`3e227c34`](https://github.com/novis-lang/novis/commit/3e227c3409764e7a53eb0f9b722e2f6ef71c7263))
- **deep**: bun nv deep runs the build legs and the release's version step ([`74105c5c`](https://github.com/novis-lang/novis/commit/74105c5c0f305900db25b8f85d86f9dc2e2570ac))
- **diagnostics**: nvs check --count and the editor index count the file-system work they did ([`0ee67eeb`](https://github.com/novis-lang/novis/commit/0ee67eeb7d1fdbdcc8041a49c6e260917909b449))
- **ext**: Core\Pdf and Core\Image read every byte source ([`03b9889a`](https://github.com/novis-lang/novis/commit/03b9889ac12d564344b9b95a8f8c39abd4fcb1fb))
- **format**: a literal Core\Str::format writes each integer placeholder into the result ([`a55599c0`](https://github.com/novis-lang/novis/commit/a55599c0d52f5d7a9ba7ae2d8431b8df67965552))
- **fuzz**: each fuzz target's check is a function cargo test replays over its seeds ([`2398bf57`](https://github.com/novis-lang/novis/commit/2398bf57b95617d48e0abe3bc04391598543244b))
- **html**: sanitize keeps the five value-checked attributes Markdown writes ([`fc0a1813`](https://github.com/novis-lang/novis/commit/fc0a181318e46dd1120e97fcc96a173caa8f7d38))
- **ide**: a file with routes shows R in the Explorer, and its tooltip names each route ([`ff916f38`](https://github.com/novis-lang/novis/commit/ff916f38ed5712b1bea4673b7c545fbf78fcbe5a))
- **io**: Core\IO and Core\IO\File read bytes, and write and append take bytes ([`21640d81`](https://github.com/novis-lang/novis/commit/21640d812e4f43e488c3b4b8baab6f16705a5749))
- **io**: Core\IO::stdinStream gives standard input as tainted bytes chunks ([`b67c68ad`](https://github.com/novis-lang/novis/commit/b67c68adb34f3dcd0b0a581b01b4399f28b3ba06))
- **ir**: interpolated strings and html templates join int and escaped holes as typed parts ([`ec835148`](https://github.com/novis-lang/novis/commit/ec8351480a3648d5fce37818896496498af17b86))
- **json**: encodeEach's pieces carry encode's taint, and a response streams clean ones ([`745de7b7`](https://github.com/novis-lang/novis/commit/745de7b75b3b8a3590b47a65bf5cbe0bb2f3d109))
- **json**: every JSON string Novis writes escapes the characters that end HTML markup or a script line ([`93d8f777`](https://github.com/novis-lang/novis/commit/93d8f777cf1e8cd1e9a3a4327d53608713b34d3c))
- **kit**: a usage wall holds only the login that met it ([`82162fdd`](https://github.com/novis-lang/novis/commit/82162fdd4b3c1641ac57c25112ba9fda82b6641e))
- **lsp**: a Core alias has a stub, a hover card and a completion row ([`78e67e9b`](https://github.com/novis-lang/novis/commit/78e67e9b51ff44e1ea326891f4a6a7f4e089a0ed))
- **lsp**: bun nv lsp-start times the editor's start and prints its index's work ([`e8969ddb`](https://github.com/novis-lang/novis/commit/e8969ddba396ddea4567f74d22a4a8ff226f48f3))
- **lsp**: nvs.inlayHints.parameterNames turns the parameter-name hints off without a restart ([`64250f15`](https://github.com/novis-lang/novis/commit/64250f150f3bed9074eeae599527e68869611071))
- **lsp**: nvs/redactions answers a docComment range per /// line to a client that declared the kind ([`110f8e3e`](https://github.com/novis-lang/novis/commit/110f8e3e2401996f00b01f8275600cf439dd5e21))
- **lsp**: the index build at start shows as work-done progress ([`74e43801`](https://github.com/novis-lang/novis/commit/74e43801cee11d82209dac9d8fdae6fa75979908))
- **markdown**: Core\Markdown::fromHtml writes Markdown from HTML ([`37789209`](https://github.com/novis-lang/novis/commit/37789209facbc7cde4e746ceaa8104be00389695))
- **markdown**: Core\Markdown::toHtml renders CommonMark and GFM through the sanitizer's list ([`6463e25a`](https://github.com/novis-lang/novis/commit/6463e25ac4ace0b6ccf905d74acdd85b236eb28a))
- **markdown**: Core\Markdown::toHtmlEach walks a source one block group per advance ([`da2bcdd5`](https://github.com/novis-lang/novis/commit/da2bcdd5c359b5d0ca47ccd4f9b20ecb2cf978fe))
- **markdown**: Core\Markdown\Live renders text that arrives in pieces ([`e1dc9472`](https://github.com/novis-lang/novis/commit/e1dc94726cf76376aaeeb1fceedcbefc21c828d1))
- **markdown**: a document is rendered in segments, and each reads the whole document's definitions ([`08bbe0a7`](https://github.com/novis-lang/novis/commit/08bbe0a7e022877c9e847cd2fa6206365b40ee79))
- **markdown**: a document renders one block group at a time, and the groups join to the whole HTML ([`1999047f`](https://github.com/novis-lang/novis/commit/1999047f3c171d63ddf7315a33b7a70b9e404475))
- **markdown**: fromHtml holds the whole tree, and a parse fed in pieces gives the same Markdown ([`40a8df38`](https://github.com/novis-lang/novis/commit/40a8df38bbe18dda921df24a9a719c283fa4519b))
- **markdown**: toHtml's link, image, heading and code callbacks change what a document renders ([`23e26c5a`](https://github.com/novis-lang/novis/commit/23e26c5a1bee69bc37d7510c340078fa4830e812))
- **nv**: `bun nv ci --check` runs CI's `rustdoc links` step before it pushes ([`ac8243fd`](https://github.com/novis-lang/novis/commit/ac8243fd3c224d838ecdd0159b23c84ff7a8ed7e))
- **nv**: `bun nv deep` runs every CI job this machine can run, and `bun nv fuzz --all` goes on after a crash ([`a2773216`](https://github.com/novis-lang/novis/commit/a27732169d6d42cae778eb98178e122ea2e3fe4f))
- **nv**: bun nv walkthrough takes pictures of a website walkthrough ([`bb31414d`](https://github.com/novis-lang/novis/commit/bb31414d14ef707ee4ee4d1cccc54a521c75d764))
- **nv**: every WSL step runs in its own test distro, which bun nv wsl makes and checks ([`cf18d84b`](https://github.com/novis-lang/novis/commit/cf18d84bd984867d0410e50e71fab7afaa99c61a))
- **nv**: proxy-validate checks nginx, Caddyfile and Dockerfile files in their pinned images ([`1d84fbaa`](https://github.com/novis-lang/novis/commit/1d84fbaa0ee5273654a3de5d64ab466111a2c340))
- **nv**: the WSL copy's HEAD is the checkout's HEAD, and deep leaves the db services up ([`661a58af`](https://github.com/novis-lang/novis/commit/661a58af3c19f5b269d1ad6f12b5247367b9c242))
- **pdf**: an img and an inline svg are inline boxes in their line ([`3dacbb53`](https://github.com/novis-lang/novis/commit/3dacbb53154d0983eca87303880cb9bda4b6de60))
- **proofs**: a member is named in its class's introduction when the class has one ([`c711b8a2`](https://github.com/novis-lang/novis/commit/c711b8a2433bf4c4e3ec55a68b224ee0c45c5ff4))
- **proofs**: a member that reads through byte_source.rs has its perf staled by it ([`b72f4595`](https://github.com/novis-lang/novis/commit/b72f4595b2c9df78fa24ca6eb9846943a2fed26b))
- **proofs**: a time stop is judged by the sweeps its run measured, not by a fixed slack ([`f7028f34`](https://github.com/novis-lang/novis/commit/f7028f346f20028d78933ee246612805bf65f7dc))
- **queue**: a Core\Queue\Id has toString, parse and tryParse ([`d4122775`](https://github.com/novis-lang/novis/commit/d4122775fa8d85c4f7a0a70f4cecd3165e262c78))
- **queue**: a job row carries a random token, and status, cancel and delete match it ([`5f82a493`](https://github.com/novis-lang/novis/commit/5f82a493ec0d455130cf8ca5d3a6e368650baa90))
- **queue**: every hand-made queue table has the token column ([`7a0a2df3`](https://github.com/novis-lang/novis/commit/7a0a2df309bc88a9121cc7bda7205175a2407ca8))
- **registry**: the package page shows each version's readme rendered once from Markdown ([`59536c04`](https://github.com/novis-lang/novis/commit/59536c04ef71a8f6bd9a8f1ef24ec4288ff4dd43))
- **release**: publishing a release lists the VS Code extension on Open VSX too ([`d76a075a`](https://github.com/novis-lang/novis/commit/d76a075aa91be9a41f66657ad3a3f2232941deda))
- **request**: Core\Request\BodyStream::peek gives the start of the body and consumes nothing ([`18af9b19`](https://github.com/novis-lang/novis/commit/18af9b19abfdfc6abffc5d8114466222747912b4))
- **request**: an upload part and a body stream give their start without being consumed ([`1aaad843`](https://github.com/novis-lang/novis/commit/1aaad843fd4f20efeb6504913df3bda1cee42dfe))
- **runtime**: a body written from an owned buffer takes it whole when the body is empty ([`e5b086f8`](https://github.com/novis-lang/novis/commit/e5b086f8adc302732cc6c8a577fe06dc163b9269))
- **runtime**: nvs_str_concat_parts joins strings, integers and escaped html text in one allocation ([`d9dd8d24`](https://github.com/novis-lang/novis/commit/d9dd8d245ef281c62254eaf4076512e92777424e))
- **runtime**: the request body's hold is shared through an Rc handle ([`cc257a8d`](https://github.com/novis-lang/novis/commit/cc257a8d03eb7e6c41a323878b60bb7bf9e75095))
- **scaling**: a constant bench's sibling is judged by what one operation costs ([`384d11e0`](https://github.com/novis-lang/novis/commit/384d11e0920ad2f44e7980715676e251275a19d2))
- **service**: a command that needs admin rights asks for them itself ([`7b1af697`](https://github.com/novis-lang/novis/commit/7b1af6976a26a19e7c63a4c9b9517f6804ea4410))
- **site**: a walkthrough scene a page shows counts as showing its editor's snippets ([`c35bd20e`](https://github.com/novis-lang/novis/commit/c35bd20e557f85d4ec3275620b4115f80cfbd171))
- **site**: every page with examples shows the nvs.toml they run with, once ([`2163c6b6`](https://github.com/novis-lang/novis/commit/2163c6b64a4470e12b8c332f52219c524dab048e))
- **site**: site --check code names every page code block no check runs ([`eccf3367`](https://github.com/novis-lang/novis/commit/eccf3367c3b9bc39c49e77b6e57978749cdeb795))
- **site**: site --check code parses every nvs line of a shell block, and each other block says why it is not run ([`26e366fd`](https://github.com/novis-lang/novis/commit/26e366fd591398e3811f10ba2383052dd7f484f5))
- **site**: site --check code reads every toml block with nvs config check, and parses json and yaml ([`3492132f`](https://github.com/novis-lang/novis/commit/3492132ff4744cf0ff2596a0655a9ac7ea5a4969))
- **site**: site --check code runs the command each txt block shows the output of, and marks each other one ([`4330bbcb`](https://github.com/novis-lang/novis/commit/4330bbcb1b9dcaf83a00e63019fed30cdb385c3e))
- **site**: site --check code validates nginx, Caddy, Dockerfile and rust blocks, and apps checks each deploy/ folder ([`a102d68b`](https://github.com/novis-lang/novis/commit/a102d68b81abb668d4cec63d2aa8f11ad545fc7e))
- **spreadsheet**: Reader::open takes a Core\Bytes::Source, a request body included ([`6f2d9be3`](https://github.com/novis-lang/novis/commit/6f2d9be3961b5c944ca04f35f95ae817e64f0525))
- **spreadsheet**: the reader has one CSV path, through Core\Csv::rows ([`46cc37ec`](https://github.com/novis-lang/novis/commit/46cc37ec5ead652c4a2be07fd99cade1ce8a385e))
- **stdlib**: Core\Hash, Core\Compress and Core\Storage::put read from every byte source ([`01fca0df`](https://github.com/novis-lang/novis/commit/01fca0dfd9910fef69959453c88ca7df114f09be))
- **stdlib**: Core\Json registers decodeEach and encodeEach with their cards ([`990f2590`](https://github.com/novis-lang/novis/commit/990f2590e6cc967af5d33958bddb8f92f1e00b79))
- **stdlib**: Core\Json::decodeEach reads one element at a time ([`fa934963`](https://github.com/novis-lang/novis/commit/fa9349631948549a5ea962bcef01ae4ca36049fa))
- **stdlib**: Core\Json::encodeEach writes an array or JSON Lines in pieces ([`459cd5d6`](https://github.com/novis-lang/novis/commit/459cd5d67001647f4a2bcf10fab9b3ba16878c93))
- **stdlib**: Core\Xml::reader reads every byte source one window at a time ([`16b25a0f`](https://github.com/novis-lang/novis/commit/16b25a0f05e1001b9e3592b3beb45c0b67f99532))
- **stdlib**: Core\Zip reads an archive by ranges from every byte source ([`81f8ac2a`](https://github.com/novis-lang/novis/commit/81f8ac2abc0574bbbcc81e242045231dca7ea90d))
- **stdlib**: a byte source reads a part forward, and decodeEach takes the forward source ([`053846ec`](https://github.com/novis-lang/novis/commit/053846eca75bfe108466cb786d2e9cc9a3c84a62))
- **stdlib**: a typed decode builds an object of scalar fields at its closing brace ([`9c5168f9`](https://github.com/novis-lang/novis/commit/9c5168f920bf0958e29d3be27ad9a5b45032b03b))
- **stdlib**: a typed decode builds every object at its closing brace ([`bc26f223`](https://github.com/novis-lang/novis/commit/bc26f2235c1d015d0849e680a3b9b94ec98dbc71))
- **stdlib**: one byte source reads a file, chunks and bytes ([`24f65922`](https://github.com/novis-lang/novis/commit/24f659220b73230903f5e98f40798f20348ae2e8))
- **types**: Core declares type aliases, and Core\Bytes::Source and Chunks exist ([`e5707179`](https://github.com/novis-lang/novis/commit/e57071797598c9444168e43c5f9292a5ae8872bb))
- **types**: Core\Json::decodeEach<mixed> type-checks and walks plain values ([`a377dc17`](https://github.com/novis-lang/novis/commit/a377dc1703e04b0ff5b847c5e94b27cfb932258f))
- **types**: a foreach over a written array with a typed binding says how to type its items ([`83c1e91c`](https://github.com/novis-lang/novis/commit/83c1e91c1fa812cde2d67bad11180436cf86ba6f))
- **types**: a request part, a body stream and a walk of tainted chunks carry taint ([`b5cd1a5f`](https://github.com/novis-lang/novis/commit/b5cd1a5fcb4853e92792f2f851302d7f2cc4c945))
- **vscode**: each /// doc comment line has a faint background in the Novis themes ([`260aa38b`](https://github.com/novis-lang/novis/commit/260aa38bd2551ca8bed5327c1205c5806548b672))
- **vscode**: the Novis themes draw brackets plainly, PhpStorm's method and variable colours, and boxless hints ([`882a9058`](https://github.com/novis-lang/novis/commit/882a9058090fa070ccc2d7d975d09fef37473e3d))
- **watchdog**: each sweep records how late it woke, and a time stop writes it for the proofs runner ([`a2906803`](https://github.com/novis-lang/novis/commit/a2906803ee7092ee3db0e3aff9fde42b61f5527a))
- **website**: N opens a speaker-notes window for a walkthrough ([`bef87bb1`](https://github.com/novis-lang/novis/commit/bef87bb1d182cbac4a3fe5eedfc8216d4dc3bfa4))
- **website**: a class page with no members has an introduction, and every class page names what returns it ([`3f5bb5e4`](https://github.com/novis-lang/novis/commit/3f5bb5e4c5a92b9de56c4502e4a65691cf6540b7))
- **website**: a gallery shows every walkthrough part in each of its states ([`4d545825`](https://github.com/novis-lang/novis/commit/4d5458251fb1eaab77ccc63ce377ad64d03d4225))
- **website**: a walkthrough caption is a bubble beside what it is about, read while the stage slows and the rest blurs ([`db394721`](https://github.com/novis-lang/novis/commit/db3947217b9bc4896f439d81120d79f504c6c49c))
- **website**: an interactive walkthrough of Core\Task::runAfterResponse, built from reusable parts ([`31515d9e`](https://github.com/novis-lang/novis/commit/31515d9ef34d3d7dd09d63d52aeeb5bae6314f3e))
- **website**: code takes the VS Code themes' colours, and the dev server restarts when a file the config reads changes ([`9e67d6c1`](https://github.com/novis-lang/novis/commit/9e67d6c183125a32138c5b0abc75bad3ca6b8855))
- **website**: every walkthrough page links its feature's reference pages ([`eab91ed3`](https://github.com/novis-lang/novis/commit/eab91ed3e6a646e1c98a2fcef1ed78f1ae3f0f3d))
- **website**: on a phone the walkthrough camera frames one part at a time ([`2230ad68`](https://github.com/novis-lang/novis/commit/2230ad68f5b4de6ff895f6566f8b9fcdf05d0bda))
- **website**: the front page has a carousel of walkthroughs that load on play ([`6bf7fef9`](https://github.com/novis-lang/novis/commit/6bf7fef96f99e4850f6fcacc76390bd4b40e231f))
- **website**: the phone header shows as many links as fit ([`facf6bc4`](https://github.com/novis-lang/novis/commit/facf6bc45a3a6c2054ac86fc4f125dc7416c54b2))
- **website**: the walkthrough "Page now, slow parts later" shows later and slotted ([`04d9cea3`](https://github.com/novis-lang/novis/commit/04d9cea3ec6efe91c7e01c3cfd84b5a1bc6d13a5))
- **website**: walkthroughs are a Guides group, and the header says Guides, Examples & Walkthroughs ([`18c61ae2`](https://github.com/novis-lang/novis/commit/18c61ae216c0bab4991a3108ad21164b468ba04f))

### Fixes

- **byte-sources**: the gate's reds from the byte source are fixed at their causes ([`9625b979`](https://github.com/novis-lang/novis/commit/9625b9799df81886f07c1d75fac6b2521460d741))
- **byte-sources**: the goal-end gates are green but the website's questions ([`4c38fde0`](https://github.com/novis-lang/novis/commit/4c38fde044fe67e1cc1bc3ae74b88e8f4ee27a52))
- **ci**: nv ci passes over a Release run cancelled behind another on its commit ([`cc76e4ac`](https://github.com/novis-lang/novis/commit/cc76e4ac66767dcf060ec4d96efb1bf11fb334f7))
- **ci**: nv ci watches every workflow that starts runs, so no red run goes unseen ([`e7934827`](https://github.com/novis-lang/novis/commit/e79348275f4de32c02e4d26bb72497c6265345a8))
- **cli**: nvs init checks only the folder it writes into, as nvs serve does ([`4366e437`](https://github.com/novis-lang/novis/commit/4366e4370da84577e5f54017aacf453d3d1d9c2d))
- **docker**: the smoke test copies its app into a volume on Windows, where a bind mount is world-writable ([`3e8b643d`](https://github.com/novis-lang/novis/commit/3e8b643d082c10779fec106ce57a25a053e79bdd))
- **errors**: a message a program sees names no rule id, tag number or slot ([`c0562a1d`](https://github.com/novis-lang/novis/commit/c0562a1d5cbbc26f037498c46409b1ec85e34c84))
- **examples**: text from a body stream and from a PDF is declared tainted ([`bc74e264`](https://github.com/novis-lang/novis/commit/bc74e2642240edae6a193ba7781f79ec8daf2246))
- **host**: a refused connect on macOS reports the refusal, not BrokenPipe ([`081f5974`](https://github.com/novis-lang/novis/commit/081f597409902e748db1005d141b38697999f2b3))
- **host**: a suspendable parse's resume and unwind are bracketed by a sanitizer fiber switch ([`634f091c`](https://github.com/novis-lang/novis/commit/634f091cbe22f323e2384a7e5d0d075c862a2db4))
- **html**: a sanitized answer starts with no BOM, and an adoption-agency move is parsed again ([`eb4153d9`](https://github.com/novis-lang/novis/commit/eb4153d98be67608df7ebcdf5b6f95d031d44851))
- **html**: a sanitized document that fostered a node is a fixed point ([`62e63198`](https://github.com/novis-lang/novis/commit/62e63198ab65ef80d7c2fb1be0dce1bc66901e94))
- **html**: the sanitizer parses in no-quirks mode, so `<p>a<table></p>` rebuilds to a fixed point ([`21528eff`](https://github.com/novis-lang/novis/commit/21528effa0646c08183f5a1eb26121aa7fc43e28))
- **html**: the serializer writes a CR as &#13; ([`bbfa6626`](https://github.com/novis-lang/novis/commit/bbfa662688b1ae28de57f52539501892521f865c))
- **http**: a reply body's walk takes from the front without moving the rest ([`04640cfc`](https://github.com/novis-lang/novis/commit/04640cfc26e821f810cac4795ed36d807da1ecfe))
- **limits**: a CPU raise from before the handler's entry does not stop the handler ([`f7f40eb1`](https://github.com/novis-lang/novis/commit/f7f40eb1636397bbed162b9bcd333c7f525f81dc))
- **limits**: a wall-time limit handler's slice is counted from its entry ([`cf009474`](https://github.com/novis-lang/novis/commit/cf009474e6639a8862a7bcfbd60675a62cc825c5))
- **lsp**: a comment offers nothing, and a doc line offers its tags and their targets ([`d947df7f`](https://github.com/novis-lang/novis/commit/d947df7fa7ab45a2012db67f7682b964132ecac5))
- **lsp**: a request with a later edit to its document queued is answered ContentModified ([`0f393976`](https://github.com/novis-lang/novis/commit/0f3939764d5a31f6d6abfa0278ce47f03c071ea8))
- **lsp**: a start answers requests before the workspace is indexed ([`8bfd5e22`](https://github.com/novis-lang/novis/commit/8bfd5e22b3f5b8b918b364b1ba39fc69546049ce))
- **markdown**: `~` and `~~` keep one lower bound each, so a `~~` with no opener leaves a `~` opener matchable ([`9edd84b4`](https://github.com/novis-lang/novis/commit/9edd84b466d8899ca980720b14976bf45f4ad32e))
- **markdown**: a CDATA section in a paragraph ends at the first `]]>` ([`a30beca3`](https://github.com/novis-lang/novis/commit/a30beca3b6b514797f8d20adc20df556ee1f0148))
- **markdown**: a CR an entity decodes to is written as LF ([`7f30cd83`](https://github.com/novis-lang/novis/commit/7f30cd837cca339aa57fad13e69168b74437ea19))
- **markdown**: a Live frees its state with the object, and a callback re-entering it throws ([`dbe3ab8b`](https://github.com/novis-lang/novis/commit/dbe3ab8b0d75aa60bde9cd4f1b24199075089171))
- **markdown**: a `\` in a link title is an escape, and a list item is no delimiter row ([`9378a955`](https://github.com/novis-lang/novis/commit/9378a95596a03e6dd44a4248148a9b6a916dbce6))
- **markdown**: a `\` that ends a header row is the cell's text ([`ce78e532`](https://github.com/novis-lang/novis/commit/ce78e532326cec2b63135901a85f67a9a383b8ec))
- **markdown**: a blank line that ends an HTML block loosens its list ([`ec06db20`](https://github.com/novis-lang/novis/commit/ec06db20ea0f1171ca965cb5e46bb30dd070b958))
- **markdown**: a closing tag may end on the next line ([`2cdcab4b`](https://github.com/novis-lang/novis/commit/2cdcab4be23ae02e74f4e31766536ff7271ec99a))
- **markdown**: a code block's language is read after its info string is decoded ([`a1f0c764`](https://github.com/novis-lang/novis/commit/a1f0c7646b057b6c9dae699fb22a44c228cb0d75))
- **markdown**: a delimiter row after spaces may be a list item ([`2d32fd04`](https://github.com/novis-lang/novis/commit/2d32fd049d5e46abea97af261e14c033a2da8237))
- **markdown**: a group after one the rebuild leaves empty starts the page ([`58a8254d`](https://github.com/novis-lang/novis/commit/58a8254d5e14c80e0b20c6afd7f0baf894983071))
- **markdown**: a header row indented four columns or more still starts a table ([`15d06a47`](https://github.com/novis-lang/novis/commit/15d06a474865a8bed60da6672decdc88f230fd7a))
- **markdown**: a header row that would be an empty list item starts a table ([`7c0b220b`](https://github.com/novis-lang/novis/commit/7c0b220b2f3386369e7f652f57c3312898377158))
- **markdown**: a lazy `===` line is part of a reference definition ([`734ccb35`](https://github.com/novis-lang/novis/commit/734ccb35a98b1a9df6d51ded4650f46d7d8bd433))
- **markdown**: a lazy line before a delimiter row is a header row ([`dffbfb86`](https://github.com/novis-lang/novis/commit/dffbfb869a53e6592b5737390f0359e3200d0043))
- **markdown**: a line indented four columns ends a table ([`ba21ac41`](https://github.com/novis-lang/novis/commit/ba21ac41a7cc187e4a04eb5ca42268b24249c7b1))
- **markdown**: a line that starts an HTML block of type 7 ends a table ([`0b3ca4b9`](https://github.com/novis-lang/novis/commit/0b3ca4b9962914408cc2894636b806b6e7997eef))
- **markdown**: a link label after an escaped `[` is text ([`1f62353b`](https://github.com/novis-lang/novis/commit/1f62353b8c6566293bd1011bf48b85742f244d09))
- **markdown**: a link title needs a space after its destination ([`ed3bd7f7`](https://github.com/novis-lang/novis/commit/ed3bd7f7b4697a7bb6bf7dfa4bd5fc8e4c624ff6))
- **markdown**: a list ends where a reference definition after it starts ([`740e0abe`](https://github.com/novis-lang/novis/commit/740e0abea5fdc58eb39f2a0d2f87b0b9286bc96d))
- **markdown**: a loose task item with nothing after its box has no paragraph ([`5429af7d`](https://github.com/novis-lang/novis/commit/5429af7d11b11c534d1c894628cdcab08ae91eec))
- **markdown**: a loose task item's box is written in front of its first paragraph ([`820d49fb`](https://github.com/novis-lang/novis/commit/820d49fbd47f11a84238add9b0125ab877114bec))
- **markdown**: a paragraph a table interrupts defines no reference ([`97224a30`](https://github.com/novis-lang/novis/commit/97224a30cbbfa8efc0d504bd5284058d16d882f6))
- **markdown**: a paragraph whose delimiter row made no table never becomes one ([`354bfb85`](https://github.com/novis-lang/novis/commit/354bfb85c9e8a014bb0c1340a50c7b94b17615cb))
- **markdown**: a parse of uncut text asks the memory limit first ([`e576e370`](https://github.com/novis-lang/novis/commit/e576e37007c366b4909e26f01a3c1a294b455549))
- **markdown**: a run of raw tags that never close costs linear time ([`4bec7d46`](https://github.com/novis-lang/novis/commit/4bec7d46032369a74142425fc2b0cd9d86fa3bec))
- **markdown**: a second blank line ends an empty list item ([`ddee5853`](https://github.com/novis-lang/novis/commit/ddee5853a4e42b46467778425b25075c2236a777))
- **markdown**: a segment is cut only at a line start ([`b8aa1191`](https://github.com/novis-lang/novis/commit/b8aa1191c56c99b0341cdfe5f2551451545717b9))
- **markdown**: a segment is cut where the text after it reads alone as in the whole ([`a5b2b137`](https://github.com/novis-lang/novis/commit/a5b2b137dbab12d9484c982f44acc73e92efe7e4))
- **markdown**: a setext underline under a lone task marker does not panic ([`05fc53b2`](https://github.com/novis-lang/novis/commit/05fc53b25f87b271cec3298558760e9598c85b2a))
- **markdown**: a tab between a task marker's brackets is text ([`d963408a`](https://github.com/novis-lang/novis/commit/d963408a37a0a2c987716cfcab5671f2d6c78f59))
- **markdown**: a tab ends an unquoted attribute value in raw HTML ([`23abbde8`](https://github.com/novis-lang/novis/commit/23abbde8b04d33b19e512cdbf69a8242c7c1709e))
- **markdown**: a tab-indented `>` does not continue a block quote ([`137fc24a`](https://github.com/novis-lang/novis/commit/137fc24a9571c897601e527f89ad5e21306e95e5))
- **markdown**: a task marker after four columns of tabs is code ([`c57f5d1c`](https://github.com/novis-lang/novis/commit/c57f5d1c3ca2a44d52b0810e1c67bccfd00cdbc6))
- **markdown**: an HTML block's end tag is matched in any case ([`6804edf7`](https://github.com/novis-lang/novis/commit/6804edf74b3e5c99a6d07d41b8d0a43bb22b0141))
- **markdown**: an angle autolink inside a link's text is written as text ([`b4f9ff87`](https://github.com/novis-lang/novis/commit/b4f9ff878b1a34ff26aef83b4205237e36c74d9e))
- **markdown**: an autolink's entities resolve, as cmark resolves them ([`b3f755d4`](https://github.com/novis-lang/novis/commit/b3f755d4710a488a1f6945794d59cd7c22254338))
- **markdown**: an escape inside `www.` or a scheme stops an autolink ([`2192f1d4`](https://github.com/novis-lang/novis/commit/2192f1d47e44bd276e286a293e57733ee8454039))
- **markdown**: an indented blank line keeps an empty list item open ([`a6585eda`](https://github.com/novis-lang/novis/commit/a6585edada31c27011556a80874e7c30f691d928))
- **markdown**: an inline link's title is the longest cmark reads ([`13740596`](https://github.com/novis-lang/novis/commit/13740596623a4206e2dbfb43c8d7137bb0455eb8))
- **markdown**: fromHtml drops a lone last break and cuts more emails ([`d76fd0a5`](https://github.com/novis-lang/novis/commit/d76fd0a5f96e5844debba9c040eb904cd533a5b8))
- **markdown**: fromHtml escapes a `!` right before a link ([`2d0fee0e`](https://github.com/novis-lang/novis/commit/2d0fee0ec86a65f74bfc475894159a601353dd51))
- **markdown**: fromHtml escapes a colon at the start of a line ([`9ba5124c`](https://github.com/novis-lang/novis/commit/9ba5124ccb437653af1a57bd4e229b3bd41863ee))
- **markdown**: fromHtml keeps a form feed as text ([`b4262c7d`](https://github.com/novis-lang/novis/commit/b4262c7d059c26e7db86aede202db508182a5b0e))
- **markdown**: fromHtml keeps text after a closed nested list tight ([`76fbb1fe`](https://github.com/novis-lang/novis/commit/76fbb1fe409a99b312784a01d47fc18c33e593c1))
- **markdown**: fromHtml writes a URL percent-encoded as the renderer writes it ([`9d7def29`](https://github.com/novis-lang/novis/commit/9d7def29b23ca008211268f36be693c2653035f2))
- **markdown**: fromHtml writes a code span of one space as ` ` ([`8a4cb7a2`](https://github.com/novis-lang/novis/commit/8a4cb7a280acc4bf1faf142cbc36a12ea87896a7))
- **markdown**: fromHtml writes a leading U+FEFF as an entity ([`9429a35c`](https://github.com/novis-lang/novis/commit/9429a35c0a4cb8a92ea059c8638ad5e0d1661885))
- **markdown**: fromHtml writes every line break a browser shows ([`a85a174a`](https://github.com/novis-lang/novis/commit/a85a174a93dee372a3328a8c2fe11a0ee47c4c55))
- **markdown**: fromHtml writes nested emphasis that parses back to the same nesting ([`9f43defc`](https://github.com/novis-lang/novis/commit/9f43defc6029f3482bb01d6d9adb795ed6461ac5))
- **markdown**: fromHtml writes nested lists on one line without a thematic break ([`9f233a7e`](https://github.com/novis-lang/novis/commit/9f233a7e9a30d6649e976677d80f8ff89964227e))
- **markdown**: fromHtml writes nothing for a list with no items ([`fb0a8053`](https://github.com/novis-lang/novis/commit/fb0a8053c72b9705dd4c2e27f780386366bafed0))
- **markdown**: fromHtml's Markdown reads back as the HTML it was written from ([`b4535b18`](https://github.com/novis-lang/novis/commit/b4535b18927118c65123d7a6a17a84ac9132ec93))
- **markdown**: fromHtml's www cut reads bytes, not a slice of the text ([`e6005b35`](https://github.com/novis-lang/novis/commit/e6005b35d37624caae8666a247bd259ed55370f0))
- **markdown**: inline raw HTML across lines drops each container marker ([`6aa0f087`](https://github.com/novis-lang/novis/commit/6aa0f0872845181e795d053c97ad526e4b08db03))
- **markdown**: raw HTML in an image's description is its source text in the `alt` ([`0a46b9b9`](https://github.com/novis-lang/novis/commit/0a46b9b91f6c0c3bf8c7c557217216606eafddfd))
- **markdown**: tables, task items and strikethrough read as GFM and cmark-gfm read them ([`912072ec`](https://github.com/novis-lang/novis/commit/912072ec0b0628007998bfdbcf92ecb466ee9389))
- **markdown**: text before the rebuild writes `"` as `&quot;` ([`6d7545e1`](https://github.com/novis-lang/novis/commit/6d7545e1b509c84c2ad1079b6c3f4921d0946882))
- **markdown**: the goal's gate is green ([`c56acc42`](https://github.com/novis-lang/novis/commit/c56acc428e595b998eeed2603a2a9ef7dfa8600f))
- **markdown**: toHtmlEach asks the memory limit before it reads each piece ([`73a87a8d`](https://github.com/novis-lang/novis/commit/73a87a8d8af27996905e8de041f8ab19ae5431e4))
- **markdown**: toHtmlEach keeps a double-escaped script open across group cuts ([`e31845c5`](https://github.com/novis-lang/novis/commit/e31845c58e1c35928fd13b432c6ae48cb86ac6bc))
- **metrics**: a gather never reads a core whose thread has ended ([`290a5237`](https://github.com/novis-lang/novis/commit/290a523784f9930ed30895d85d93fc40864837fd))
- **nv**: CONTRIBUTING.md lists nvs-fuzz-checks, and ci --check runs the layout check ([`3cec78f6`](https://github.com/novis-lang/novis/commit/3cec78f67dcf53815a30b4a7e2b29b9449604cc6))
- **nv**: `bun nv ci --watch --past` covers a fuzz leg that names several targets after a comma ([`aa90ad79`](https://github.com/novis-lang/novis/commit/aa90ad79103733ac2d068140a3d867ff43ea2cd4))
- **nv**: `bun nv deep`'s `fuzz-smoke` step is not killed while libFuzzer replays a grown corpus in silence ([`10a93a08`](https://github.com/novis-lang/novis/commit/10a93a0828604693f919ced8220ceace080f1595))
- **nv**: `bun nv deep`'s docker step gives the build a writable `target` inside the read-only source ([`a0d1aec6`](https://github.com/novis-lang/novis/commit/a0d1aec6ca6232fc7ffdd4d9fb5c9640f9d40c21))
- **nv**: links reads a source file's comments only, and skips vendor/ ([`3a409d8d`](https://github.com/novis-lang/novis/commit/3a409d8d0ead0cd7cc0bf74d28892b8da2c81698))
- **nv**: scaling --budget --write stores a new bench's share while another is over ([`65d01e61`](https://github.com/novis-lang/novis/commit/65d01e61e381718ea78f1eefb2cec342de8e4240))
- **pdf**: a fixed-height box taller than the room left continues on the next pages ([`8a45e985`](https://github.com/novis-lang/novis/commit/8a45e985dce9e2fe56440792eecaa4bda61d897b))
- **pdf**: a flex band taller than a page splits between its items' lines ([`3713e55c`](https://github.com/novis-lang/novis/commit/3713e55c87066a118061d906405688d3416975a7))
- **pdf**: a heading kept with a block taller than a page moves with that block's first lines ([`e833c6a6`](https://github.com/novis-lang/novis/commit/e833c6a63e9d2424c76ca8bd6f2982a7b7758f97))
- **pdf**: a line that floats leave no room starts below them ([`0f5f9fbd`](https://github.com/novis-lang/novis/commit/0f5f9fbd9023b340866d8472a7a1cdbb8ac5838f))
- **pdf**: a right-to-left table and flex row run from the right ([`8af3bc7e`](https://github.com/novis-lang/novis/commit/8af3bc7ea007516c6af73ec167fd3a19c3eacbf8))
- **pdf**: a table copies only its own cells, so tables nested 2000 deep end at the depth error ([`72252177`](https://github.com/novis-lang/novis/commit/7225217765b3f3d4c42d916c0ec1a38a53bab5fd))
- **pdf**: a table's header and footer are left off when no row fits with them ([`20cc89a9`](https://github.com/novis-lang/novis/commit/20cc89a9da2ca09d4417898cd81056c1942a3b8f))
- **pdf**: collapsed table borders resolve one border per edge by CSS 2.1 ([`98ea4028`](https://github.com/novis-lang/novis/commit/98ea40282df4b1f1d081176e9623e60ed7c23e2e))
- **pdf**: non-finite values are dropped and reported, var() resolves each property once, size-0 text is skipped ([`ed73be4d`](https://github.com/novis-lang/novis/commit/ed73be4d7d75edf7d75b2817df4f8dc7fcbed297))
- **pdf**: overflow clips, visibility hides, margins go negative, and list types are read ([`a253a7f1`](https://github.com/novis-lang/novis/commit/a253a7f1e6f36e3551042aa5ed14309fca368835))
- **pdf**: page counters restart, page selectors and media features are read or reported ([`42fa083b`](https://github.com/novis-lang/novis/commit/42fa083b1db22c8e46e5534d1cd95d70f15157e8))
- **pdf**: page sides follow the document's direction, and a forced break keeps the next margin ([`e1eb5d41`](https://github.com/novis-lang/novis/commit/e1eb5d41aaa67da5a09ecb9ee9f2f424d967093f))
- …and 58 more, in the full changes below.

### Performance

- **bench**: Core\Test's members are measured again on the completion's file read ([`9ae59789`](https://github.com/novis-lang/novis/commit/9ae5978922e473e64a68b54cf12e2b1092120cb5))
- **bench**: the comparison is recorded on the tree with the faster request path ([`972da656`](https://github.com/novis-lang/novis/commit/972da656aa131e2a1e947c578935157eef177d37))
- **bench**: the comparison's cases are recorded on the tree with one read per request ([`4ccbe86a`](https://github.com/novis-lang/novis/commit/4ccbe86a79038f024ca5aa9b277a2de28a9cf3e1))
- **bench**: the literals bench fills an html template, and three string benches declare their allocations ([`05a7f44d`](https://github.com/novis-lang/novis/commit/05a7f44db95aeced8721d98a484ebb98267a24fa))
- **byte-source**: a file is read into a buffer of its size, at most one read chunk ([`e73a6fcb`](https://github.com/novis-lang/novis/commit/e73a6fcb8e3f07e30e1fc341d9ed2a636ee1c6fa))
- **byte-sources**: every figure the byte source made stale is measured again ([`6bf83c94`](https://github.com/novis-lang/novis/commit/6bf83c94cbfa04a1389ace9ba55d67f877ceb974))
- **byte-sources**: the figures joined to byte_source.rs are measured again ([`c3719ad9`](https://github.com/novis-lang/novis/commit/c3719ad959d7bb0682f0cfa840ae68a261d8ae80))
- **check**: a class file's lender search walks its own project and reads each file once ([`62a91776`](https://github.com/novis-lang/novis/commit/62a917765aa2f8f54d3563de35d7371e4aac36a3))
- **check**: an autoloaded file's path is resolved once per compile and once per lender question ([`c8d86440`](https://github.com/novis-lang/novis/commit/c8d86440089be70e61ff31f2ceaf1c45439cb1ec))
- **config**: an uncached run resolves each settings path and its entry once per form ([`28aba995`](https://github.com/novis-lang/novis/commit/28aba995c8ff2481e57f46ce8d75c10df7735ff6))
- **host**: a request that keeps its default CPU limit never wakes the watchdog ([`f964813a`](https://github.com/novis-lang/novis/commit/f964813a5233a4ab8b5dbab83f9b3e2bea663856))
- **host**: a short read on Unix parks the next read without a recv ([`64976035`](https://github.com/novis-lang/novis/commit/649760357ac8bef60404622b8a09088a84839cd5))
- **host**: an outbound connect asks the zero-length write before SO_ERROR ([`5b2037df`](https://github.com/novis-lang/novis/commit/5b2037dfac90d226b3e1ca80eb8028bcca75421c))
- **html**: Core\Html's six figures are measured on the BOM and adoption-agency fix ([`0dead72b`](https://github.com/novis-lang/novis/commit/0dead72b3a6a9f45cf1d923fa9aef3f68cbbdc7d))
- **ir**: a bare integer placeholder joins as an integer part ([`670506f2`](https://github.com/novis-lang/novis/commit/670506f22d4bd1f8bb9f831dc9b5742fd6de5784))
- **lsp**: a cold workspace index costs a sixth of what it did ([`92d208d2`](https://github.com/novis-lang/novis/commit/92d208d21ced717dcb217e9f4b6af3d1df2f24ea))
- **lsp**: an index entry is analysed apart from the merge, and resolves its path once ([`09e9b51d`](https://github.com/novis-lang/novis/commit/09e9b51d102a267f7b273baa23bfea1a5523e33d))
- **lsp**: the editor parses each settings file once until it changes ([`9c8e2e99`](https://github.com/novis-lang/novis/commit/9c8e2e993b97843bd3ad9c73ae52adb38dd36e5f))
- **lsp**: the index settles on worker threads, and files each result in path order ([`cd620e16`](https://github.com/novis-lang/novis/commit/cd620e164b01c4565ed63c4ff9d2577f8abd6829))
- **nv**: bun nv fuzz runs a target on half the cores in fork mode ([`31d1ab81`](https://github.com/novis-lang/novis/commit/31d1ab814882887a0a65aefba54da531e51d452c))
- **pdf**: a table's rows are paged with suffix sums, so a longer table costs work in proportion ([`e4634c50`](https://github.com/novis-lang/novis/commit/e4634c50385183dbd450a757c864ecc71996903a))
- **pdf**: an image used many times in a document is resolved and decoded once ([`f8c95dc2`](https://github.com/novis-lang/novis/commit/f8c95dc242214ee907d2c6e756071efd5161956c))
- **pdf**: margin boxes read named strings, counters and running elements from a page index ([`4c581b3e`](https://github.com/novis-lang/novis/commit/4c581b3edbe8c5125fc82f4cb6734c678d4cbb25))
- **pdf**: the cascade tests an element only against selectors its id, classes or tag can match ([`89081b3a`](https://github.com/novis-lang/novis/commit/89081b3aef94b4783ec30410c5a0b9f50ab2cd3e))
- **proofs**: every stale stdlib perf figure is measured again on this tree ([`c7517507`](https://github.com/novis-lang/novis/commit/c7517507c310c550f9a3ff23bada82f0dda5bff9))
- **proofs**: the IO, Xml, Compress, Request and Mime classes are measured again ([`2ffccd6a`](https://github.com/novis-lang/novis/commit/2ffccd6a1dfa781e1a0786b602ea1e75e78c3c27))
- **proofs**: the members stage 4 touched and the escape and format members carry fresh records ([`cde06a13`](https://github.com/novis-lang/novis/commit/cde06a13e812f51bc923c6c4632e3ae7e7714e3d))
- **proofs**: the stream, flags and reflect members carry records taken after rustfmt ([`a603b742`](https://github.com/novis-lang/novis/commit/a603b742465851b3345e144cd7054a67e683631e))
- **queue**: the queue and JSON members carry fresh perf records ([`b84382ec`](https://github.com/novis-lang/novis/commit/b84382ec2f83f35f0854343c8c997c9e67269a85))
- **render**: an escaped text is scanned once to measure and once to write ([`755bf4df`](https://github.com/novis-lang/novis/commit/755bf4df62418d4927eb473d4298b4a68d41912c))
- **runtime**: a byte writer's write that fits its buffer asks the budget nothing ([`7b04f310`](https://github.com/novis-lang/novis/commit/7b04f310528dcfb1b3b5b18d16483a0bc0467996))
- **runtime**: an integer placeholder writes its digits without Rust's formatter ([`f837844b`](https://github.com/novis-lang/novis/commit/f837844b20e977472c19eae2228ef9077963343b))
- **server**: Core\Response::sendJson's body is the encoder's chunks and peaks at the document plus one chunk ([`f4dc58dd`](https://github.com/novis-lang/novis/commit/f4dc58dd760fce207667d525e6e87afebb548dee))
- **spreadsheet**: custom formats, long format codes and whole-column references cost linear work ([`4e38a9cc`](https://github.com/novis-lang/novis/commit/4e38a9cc181f1e05a1b915dc77537f634d161aec))
- **stdlib**: 56 Core\Json, Core\Request, Core\Response, Core\Script and Core\Test records are measured again ([`a29c3d4c`](https://github.com/novis-lang/novis/commit/a29c3d4c1e3492f6ec4ee92b2e2ad2a0a7052a13))
- **stdlib**: Core\Json's encoder allocates nothing per element and peaks at twice the document ([`486b65e4`](https://github.com/novis-lang/novis/commit/486b65e4c87dba5863c575b291fa2bccc1514871))
- **stdlib**: Core\Response::sendJson hands the encoder's buffer to the body ([`7f05c071`](https://github.com/novis-lang/novis/commit/7f05c071e587beb1e16bd769d15b7cec80129f72))
- **stdlib**: Core\Str is measured again after find_counted ([`8df6864d`](https://github.com/novis-lang/novis/commit/8df6864d1fe64f880e60b1983df175e3fc17d559))
- **stdlib**: Core\Test's assertCount, assertEquals and assertNull ramp to a batch where their counts agree ([`ac16c795`](https://github.com/novis-lang/novis/commit/ac16c79512cb68d073b223d1b85e452bd54cc310))
- **stdlib**: Core\Test's perf records are measured on the test.rs that answers a sendFile response ([`bcd795c5`](https://github.com/novis-lang/novis/commit/bcd795c5fa68d450ba4b293f4d39133a621d08d2))
- **stdlib**: a Core instance finds its descriptor by the class's address ([`35745036`](https://github.com/novis-lang/novis/commit/3574503695d1a8f4e0015ee91cfbd33ec6926e90))
- **stdlib**: a request body with a declared length is read into one buffer of that length ([`9bcf85a1`](https://github.com/novis-lang/novis/commit/9bcf85a1ac8bd11d251f02aed0965a739031c7fa))
- **stdlib**: decodeEach and encodeEach each have a bench whose ladder holds memory flat to 256 MiB ([`81ca7312`](https://github.com/novis-lang/novis/commit/81ca73122d929df1543adc2a8c2096650d5bf74b))
- **stdlib**: every JSON member states its peak per document byte, and its size ladder is recorded flat ([`793ef0da`](https://github.com/novis-lang/novis/commit/793ef0daea3ce3fd1169c89f27ce9595624f7233))
- **stdlib**: the JSON members' size ladders grow the document in bytes from 1 KiB ([`59f53faa`](https://github.com/novis-lang/novis/commit/59f53faa1ca7af29c2e270237494f292e526f1a9))
- **stdlib**: the JSON reference ladder times the free apart and prints serde_json's own encode ([`e3edb8a4`](https://github.com/novis-lang/novis/commit/e3edb8a44a87c76ce0cd208326537f3b3cdddd13))
- **stdlib**: the typed JSON readers are measured again and declare their allocation counts ([`c60ce6a4`](https://github.com/novis-lang/novis/commit/c60ce6a4d6cdb4d2d248b03eb99a1672b7e5b202))
- **test**: a directory of programs reads each file once ([`6c13c339`](https://github.com/novis-lang/novis/commit/6c13c3390800d3caed715c2e70a93a70922eb771))
- **test**: a directory of programs resolves only the files another one requires ([`5e53c0ad`](https://github.com/novis-lang/novis/commit/5e53c0ada37a68c95f601d9927238dd42f304759))

### Formatter

- **fmt**: io, otlp and the limits tests are formatted by rustfmt ([`c0ff7183`](https://github.com/novis-lang/novis/commit/c0ff718328a47c6c081f6e73f4d0336e4d19a5cf))
- **fmt**: the goal's earlier slices are in rustfmt's layout ([`ebc4ac9f`](https://github.com/novis-lang/novis/commit/ebc4ac9fd1941fc5dd8137266bd63318159ef4fa))
- **fmt**: the parallel index's files are in rustfmt's layout ([`cc9bfb2b`](https://github.com/novis-lang/novis/commit/cc9bfb2bae2b01649b45a6fa6588114bc7bfc936))
- **fmt**: the typed-parts join and its lowering are formatted as rustfmt writes them ([`5c238d3c`](https://github.com/novis-lang/novis/commit/5c238d3c02f2259e7f31b885f09dad176d6661c8))
- **fmt**: the watchdog policy case is formatted by rustfmt ([`6560a9b5`](https://github.com/novis-lang/novis/commit/6560a9b5ea6170ac847574404401e2094860776f))

### Internal

463 further commits: docs (268), test (101), chore (72), build (12), refactor (6), wip (4).

[Full changes](https://github.com/novis-lang/novis/compare/v0.0.2...v0.0.3) — 772 commits.

## [0.0.2] - 2026-11-03

### Features

- **lsp**: a body-only edit of an open file re-queues no reader ([`84172184`](https://github.com/novis-lang/novis/commit/84172184433eeea9ba51f4fac6495dd294a4dda4))
- **lsp**: a file is parsed again only when its text changed ([`207a0cd4`](https://github.com/novis-lang/novis/commit/207a0cd4674b468c6ce2b814c176108e9db15a4a))
- **lsp**: the editor checks only the bodies whose inputs changed, against one session interner ([`4e0674e8`](https://github.com/novis-lang/novis/commit/4e0674e8cce327d17c3459ba314885c6e3f1f4b4))
- **lsp**: the symbol index is kept on disk, and a start analyses only its stale entries ([`59eb4b7f`](https://github.com/novis-lang/novis/commit/59eb4b7f4cd60f171137a0d99c0be431644039c2))
- **site**: the website counts page views with a cookieless Matomo on stats.novis-lang.org ([`da19c63e`](https://github.com/novis-lang/novis/commit/da19c63e1091ffe261a879a791b9905ee189af8a))
- **tools**: NVS_PROOF_TIMES writes each proof program's wall time, slowest first ([`f62d0639`](https://github.com/novis-lang/novis/commit/f62d063984e5a5c7e32209606169a6b65c2581d3))
- **types**: a check walks the bodies of a chosen set of files and folds every other file's cached output ([`297f9356`](https://github.com/novis-lang/novis/commit/297f935629565bc0a81299be015467c5845f5cf2))
- **vscode**: the nvs.path setting links the download and the release page ([`1e6737b7`](https://github.com/novis-lang/novis/commit/1e6737b7c6240011c979b7b0403061f194176421))

### Fixes

- **cli**: nvs run's stop cancels a queue worker still opening its connection ([`63a6861d`](https://github.com/novis-lang/novis/commit/63a6861d44bb8588bf152a90baf48dc67468f310))
- **cli**: nvs run's stop cancels a queue worker still opening its connection again ([`45e4ac4c`](https://github.com/novis-lang/novis/commit/45e4ac4c7c3db5e521e098171b3359f877455f2a))
- **cli**: nvs run's stop no longer cancels a queue worker still opening its connection ([`cb931f1e`](https://github.com/novis-lang/novis/commit/cb931f1efaded760478f1e2e75b98f081f5cabde))
- **goals**: incremental-index's named checks run the tests they name ([`fa8b07a3`](https://github.com/novis-lang/novis/commit/fa8b07a3b628d191997883063d477b132dc9e4d0))
- **host**: a cancelled task unwinds without a crash in Windows' optimized builds ([`6b0bc591`](https://github.com/novis-lang/novis/commit/6b0bc591ce0f793bf528233e8496b34fd34fffae))
- **host**: the slice clock wakes on time on macOS, so a group's deadline is read before the CPU stop ([`48879136`](https://github.com/novis-lang/novis/commit/488791366020cd1f43d05e9a4b11898cceb2674b))
- **lsp**: the editor types a program against the built-in components ([`35d3753f`](https://github.com/novis-lang/novis/commit/35d3753fe6f2e0c5235303120366468eac5096d2))
- **nv**: a release judge reads build.yml's legs at the run's commit ([`60bf5449`](https://github.com/novis-lang/novis/commit/60bf5449fbce929d6f196fe8ea6b5484ade31214))
- **nv**: embedded text runs no platform matrix ([`0d9eda6a`](https://github.com/novis-lang/novis/commit/0d9eda6a7ee33fa769c90fb421b8fabe82854f4d))
- **nv**: the code lane covers every input the binary is built from ([`b5a12569`](https://github.com/novis-lang/novis/commit/b5a12569b6185c7afe4e5a8d3a89bafcdaeb8f55))
- **release**: a release binary names the commit it was built from, without -dirty ([`4ce8a689`](https://github.com/novis-lang/novis/commit/4ce8a6893fc6951470870f022d7f63d757600a98))
- **release**: publish fetches the committed lock before --apply bumps the version ([`53c7b504`](https://github.com/novis-lang/novis/commit/53c7b504e968ddaaac27b108149747d5db811e21))
- **runtime**: a Serialize payload refused partway frees every object decode built ([`4b8503ca`](https://github.com/novis-lang/novis/commit/4b8503ca80d7fb8ee67c02f4768bf0dfebcb3ee8))
- **site**: the front page names the site Novis-Lang to search engines ([`1684a158`](https://github.com/novis-lang/novis/commit/1684a1580b6c9026d58ed5237ff4ee83fce9e5fa))
- **site**: the front page's search questions are stamped against its new head ([`383af5ab`](https://github.com/novis-lang/novis/commit/383af5abc45d4659f0e35cdd722c338960196ae8))
- **site**: the hero slogan says SQL injection does not compile ([`d5294d15`](https://github.com/novis-lang/novis/commit/d5294d15a7339ff03c827c29e37b5ecf680cf074))
- **site**: the hero slogan says SQL injection is blocked, so it never reads as NoSQL injection ([`72b8c974`](https://github.com/novis-lang/novis/commit/72b8c9749abe6095f90bb4765f11c93c23c87f59))
- **spreadsheet**: the tests read back through calamine 0.38, clear of RUSTSEC-2026-0436 ([`56f215bb`](https://github.com/novis-lang/novis/commit/56f215bb0a1fe129b0e47fdf4bf1d887726f63bc))
- **types**: a per-file table part inherits the provenance switch and never turns it on ([`a35771e7`](https://github.com/novis-lang/novis/commit/a35771e7ca124c25619eb2b5c88d7c29dded36d3))
- **website**: the sidebar opens with the current page's entry in view ([`2dfacf60`](https://github.com/novis-lang/novis/commit/2dfacf6099c8f4c6068b1be838b5ced38f1f31df))

### Performance

- **lsp**: an answer waits for the open document, never for the index ([`e5b77942`](https://github.com/novis-lang/novis/commit/e5b7794256c67ea741d1f4ffac795ed65fc8b757))

### Internal

55 further commits: docs (24), build (10), test (10), chore (8), wip (2), refactor (1).

[Full changes](https://github.com/novis-lang/novis/compare/v0.0.1...v0.0.2) — 84 commits.

## [0.0.1] - 2026-11-02

This is the first release of Novis.

Novis is the programming language for web applications and the command line. You can build a
business app, a website, an API or any other web service with it. The tools that a modern web
application needs are built in.

- **Input is tainted.** Text from a request must be checked or escaped before it reaches a
  database, a shell or a page.
- **Every request on its own.** Each request has its own memory. When one request crashes, the
  others keep running.
- **One program.** `nvs` runs your code, serves web requests, runs your tests and formats your files.
- **Compiled to machine code.** Each file is compiled the first time it is needed. You never run a
  build step.

### Pre-Alpha

Novis is Pre-Alpha. Any release can change the language, and Novis is not ready for production use.

### Install

Download the archive for your system from the files below. Each archive contains the `nvs` program.
Put `nvs` on your `PATH`, then run `nvs --version` to check it. `SHA256SUMS` lists the checksum of
every file.

You can also use the Docker image:

```sh
docker pull ghcr.io/novis-lang/novis:0.0.1
```

The editor extension for VS Code is `nvs-0.0.1.vsix`, in the files below. In VS Code, open the
Extensions view and choose **Install from VSIX...**.

### Read more

- [Install Novis](https://novis-lang.org/install/)
- [The roadmap](https://novis-lang.org/in-depth/roadmap/): the work that is planned next.
- [What changed](https://novis-lang.org/in-depth/what-changed/): what is new in Novis.
