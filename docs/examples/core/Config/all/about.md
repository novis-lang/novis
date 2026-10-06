Gives every setting in force for this request, as one array.

The keys are the dotted names a configuration file uses, such as `limits.memory` or `log.level`, and
they are sorted by name. Every value is text, written the way the configuration file writes it. A
setting your program changed with `Core\Config::set` is in the array with the new value. A name that
holds a group of settings is not an entry of its own, and a name that holds a list is not in the
array at all.

**Good to know:** the array is a copy. Changing it changes no setting. It also holds passwords that
were read from a secret file, so do not write the whole array into a log.
