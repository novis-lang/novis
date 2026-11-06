Three steps set up a server. First, put `nvs` in a folder and put that folder on the `PATH`.
Second, run `nvs init` in the project folder. It creates the data folder and writes the project
file `nvs.toml`. Third, run `nvs service install <name> -- serve <entry file>`.

The steps work in any order. `nvs init`, `nvs service install` and `nvs run` all use the same data
folder. You can run `nvs init` again at any time. It creates nothing and overwrites nothing.

A service does not start in the project folder, so give the entry file as a full path.
`nvs service install` asks for admin rights itself.

**The example below** prints the commands of steps two and three for each service in its input. It skips a project
folder that is not a full path.
