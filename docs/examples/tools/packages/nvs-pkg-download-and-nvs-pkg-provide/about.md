`nvs pkg download` adds a large file to your package, such as a font or a data set, without putting
it in the package's archive.

Run the command in the folder of your package. It downloads the file and adds a `[[download]]` entry to `package.toml`. The entry has the URLs,
the SHA-256 hash and the size of the file. Every URL must start with `https://`. The path may not be
under `src` and may not end in `.nvs`.

When somebody fetches your package, `nvs pkg fetch` downloads the file and checks its size and its
hash. When they do not match, the file is not used and the next URL is tried. Nothing in the file runs.
The package's own code may read the file with no grant.

When no URL works, get the file in another way. Then run `nvs pkg provide` with it, and fetch again.

**Good to know:** each file is kept once in a cache in the Novis data folder. Every project that needs it uses
that copy.

**The examples below** show a package reading a downloaded file, a file with two URLs, and the hosts
that `package.lock` lists.
