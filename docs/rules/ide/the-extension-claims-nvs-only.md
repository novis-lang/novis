The extension registers `.nvs` and does not claim `.php`, even though `nvs-syntax` parses it. Claiming it
would fight every PHP extension a user already has, and losing that fight silently looks like Novis being
broken. An opt-in setting is M10's if anyone converting a codebase asks for it.

The extension-host run proves activation on `.nvs` and its absence on `.php`.
