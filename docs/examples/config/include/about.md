Reads more settings from another file, or from every `.toml` file in a folder.

Use it to keep the settings of one machine apart from the settings you share. For example, the
shared `nvs.toml` is in version control, and each server adds its database address in its own file.
A folder is read in filename order, so a server can add a file without changing a shared one.

Novis reads a file's own settings first, and then the files it includes. When two files set the same
setting, the file read later wins. So an included file changes the settings of the file that
includes it, even if the `[[include]]` line is at the top. `nvs config dump --origin` shows which
file set each value.

**Good to know:** `optional = true` allows a missing file. A file that cannot be read or parsed stops
the server. So does a file that an account other than the server's could change.
