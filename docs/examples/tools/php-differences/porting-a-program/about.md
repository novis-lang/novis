To port a PHP program, you read it to learn what it does, and then you write it again in Novis.

PHP code does not run in Novis, and no tool converts it. There is also no list that gives the Novis
method for each PHP function. You choose the new design yourself.

Start with the types: the classes, their typed properties and the enums. Then write each part again
with the `Core` classes. `nvs agent find` finds the method for a task. Run `nvs check` after each
part, and write tests that `nvs test` runs.

For a large program, an AI coding agent can do this work. `nvs agent init` tells the agent about
`nvs agent`, and the agent looks up each method with `nvs agent find` and `nvs agent show`.
