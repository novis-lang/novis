Puts a setting back to the value the configuration file gives it, or to its built-in default.

`Core\Config::set` changes a setting for this request only. `restore` deletes that change, so the
setting is again what the configuration file says. A program that changed a setting for one step can
undo it afterwards. The name is read the way `Core\Config::get` reads it, so a limit can be written
with its short name, `memory`, or with its full name, `limits.memory`. One call puts back one setting.

**Good to know:** a name your program never changed is allowed. There is nothing to undo, so nothing
happens and there is no error. When the configuration file does not write the setting, it gets its
built-in default again. A setting with no default, such as `wall_time`, is `null` again.
