`package.toml` describes your application or a package: what it needs, and what it may do.

Your application's file lists the packages your code needs in `[dependencies]`, with the lowest
version that works. `[registries]` says where each vendor's packages come from. `[grants]` says which
capabilities the code of each package may use, such as `fs.read`. A package's file has a `[package]`
table with its `name` and `version`. It lists the capabilities it needs in `[capabilities]`, and its
data files in `files`.

When your program compiles, Novis reads only `[grants]` from your application's file. From a
package's file, it reads only the name and `[capabilities]`. Your program never reads the file when
it runs.

**Good to know:** a package may read the files it lists with no grant. It may not write them.

**The examples below** show an application using a package, a package reading its own data file, and
a package with one required and one optional capability.
