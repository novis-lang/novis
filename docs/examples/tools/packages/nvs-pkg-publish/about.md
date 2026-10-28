`nvs pkg publish` publishes your package at the last commit of its git repository. The command
sends no files. It sends the repository's URL, the commit, the version and the package's folder. The
registry downloads that commit and builds the archive itself, so push the commit first. The
repository must be public, with an `https://` URL.

The command builds the same archive on your computer first and prints its digest. `--dry-run` stops
there and lists every file.

A release sent with a token is pending for seven days, and `nvs pkg add` does not choose it. An owner
who signs in with a passkey on the website can release it at once.

The registry does not publish a version twice. Every class must be inside the namespace of the
package's name. A minor or patch version must not break the API.

**Good to know:** using a token does not count as activity. After 60 days with no passkey sign-in,
sign in on the website to publish again.
