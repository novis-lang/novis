Gives you the entries of an array in the opposite order.

The result is a new array, and the original is not changed. By default the keys of the original are
gone: the entries are numbered from 0, string keys included. Set `preserveKeys` to keep every entry
under its own key. The values come back in the same order either way, and only the keys differ.

**Good to know:** a list you reverse twice is the list you started with. An array with keys you
reverse twice has its values in the original order again, and the keys are gone unless you asked to
keep them.
