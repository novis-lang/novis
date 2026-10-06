Puts a setting back to the value the configuration file gives it.

`Core\Config::set` changes a setting for this request only. `restore` drops that change, so the
setting is again what the configuration file said, and a program that changed a setting for one step
can undo it afterwards. The name is read the way `Core\Config::get` reads it, so a limit can be
written with its short name, `memory`, or with its full name, `limits.memory`. One call puts back one
setting.

**Good to know:** a name your program never changed is allowed. There is nothing to undo, so nothing
happens and there is no error. A setting the configuration file does not give a value has no value
again after this.
