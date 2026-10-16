`rule:core-api/written-visibility` makes an omitted visibility keyword a compile error, and `nvs fmt`
never inserts the keyword. The formatter orders modifiers and does not supply a missing one. A
formatter that inserted `public` would make a file's *meaning* depend on whether a tool had been run
over it, and would restore an implicit default for anyone who formats on save. A file that does not
compile still does not compile after `nvs fmt`, and its author writes the level they mean.
