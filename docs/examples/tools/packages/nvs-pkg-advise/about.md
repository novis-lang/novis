`nvs pkg advise <package> <versions> <summary>` sends a security advisory for a range of versions
of your package. `nvs pkg audit` then reports every locked version in the range, with the summary.
You can write the package as `acme/http` or as `Acme\Http`. The summary has at most 500 characters.

The range has two forms. `1.2.0..1.2.4` is the versions from 1.2.0 up to 1.2.4, without 1.2.4.
1.2.4 is the version that fixes the problem. `1.2.0..` is every version from 1.2.0. Use it when no
version fixes the problem yet. The first version must be lower than the second.

With a token, the advisory is in the registry's feed seven days later, and every owner gets a
message. An owner can send it again with a passkey on the website. Then it is in the feed at once.

**Good to know:** an advisory does not change which version `nvs pkg add` chooses. To stop a
version from being chosen, use `nvs pkg retract`.
