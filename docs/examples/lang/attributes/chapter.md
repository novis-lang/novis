---
order: 90
title: Attributes, routes, commands and derived codecs
summary: `#[...]` metadata as anonymous objects, how it is read back, and the names the compiler acts on — JSON codecs, the route table, the command table, and program enumeration
keywords: attribute, #[...], Attribute, reflection, read attributes, Core\Attributes, get, all, #[Route], route attribute, Route, Access, Query, Api, OpenAPI, Router::url, urlAbsolute, Command, Option, console command, Json\Derive, Json\Field, JSON encoding, decodeAs, Program::implementing, autoload
---

# An attribute is an anonymous object attached to a declaration
# Reading attributes back: `Core\Attributes::get` and `all`
# The names the compiler acts on
<!-- generated: attributes -->
# `#[Core\Json\Derive]` and `#[Core\Json\Field]`: a class with a JSON codec
# `#[Core\Route]` and `#[Core\Access]`: the route table
# `#[Core\Api]` and the OpenAPI document
# `#[Core\Command]` and `#[Core\Option]`: the command table
# `Core\Program::implementing<I>()`: every class that is an `I`
# `Core\Program::implementingWith<I, T>($member)`: every implementor with one attribute
