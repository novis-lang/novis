`#[Route]` is repeatable (`rule:attributes/repeatable`), and repeating it is how one method serves
several verbs. Repeated `#[Route]` attributes on **one method** may carry the same `name`, provided
every one of them carries the same `path`:

```php
#[Route(path: "/webhook", method: Http\Method::Post, name: "webhook")]
#[Route(path: "/webhook", method: Http\Method::Put,  name: "webhook")]
#[Access(allow: Audience::Public)]
public function receive(): Response { … }
```

One path per name is the whole of the safety argument: it keeps `Core\Router::url` a function, so
no rule is needed to choose between two declarations. Every other duplicate `name` is `E0749`,
naming both sites — the same name on two different methods, which is the copy-paste that gives two
endpoints one identity; and the same name on one method whose repetitions carry different paths,
where `url()` would have two answers and no ground to prefer one.

**The duplicate-*route* rule is untouched.** Two attributes sharing both `path` and `method` remain
an error however they are grouped, so nothing enters through the door a shared name opens. The name
is written on each line rather than inherited from the first: the attributes are independent
payload objects, and an inheritance rule would make their order matter, which the route table's precedence
rules exist to avoid.
