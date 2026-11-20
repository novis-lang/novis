---
order: 90
title: Attributes, routes, commands and derived codecs
summary: Declared attributes written as `#[...]`, how they are read back, and the names the compiler acts on. These give JSON codecs, the route table, the command table and the list of a program's classes.
keywords: attribute, #[...], Attribute, attribute declaration, repeatable, reflection, read attributes, Core\Attributes, get, all, #[Route], route attribute, Route, Access, Query, Api, OpenAPI, Router::url, urlAbsolute, Router\Target, Router\Params, Target, Params, Command, Option, console command, Json\Derive, Json\Field, JSON encoding, decodeAs, Program::implementing, autoload
---

# An attribute is declared with the places it may be used
# Reading attributes back: `Core\Attributes::get` and `all`
# The names the compiler acts on
<!-- generated: attributes -->
# `#[Core\Json\Derive]` and `#[Core\Json\Field]`: a class with a JSON codec
# `#[Core\Route]` and `#[Core\Access]`: the route table
# `#[Core\Router\Target]` and `#[Core\Router\Params]`: a method that takes a route
# `#[Core\Api]` and the OpenAPI document
# `#[Core\Command]` and `#[Core\Option]`: the command table
# `Core\Program::implementing<I>()`: every class that is an `I`
# `Core\Program::implementingWith<I, T>($member)`: every implementor with one attribute
