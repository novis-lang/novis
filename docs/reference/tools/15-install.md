---
id: install
title: "Installing on a host: folders and permissions"
summary: the three steps that set up a server; which commands ask for admin rights; the data folder where Novis keeps its own files; where to put the `nvs` binary, the configuration, the compile cache and the logs on a server; which folder permissions Novis checks and when; the commands that set them on Windows and on Linux; and what each refusal message means
keywords: install, installation, setup, deploy, deployment, server, host, folder, directory, permissions, ACL, DACL, icacls, chmod, chown, owner, ownership, Authenticated Users, Users, Everyone, SID, S-1-5-11, S-1-5-32-545, inheritance, E0607, access denied, os error 5, nvs init, --yes, nvs serve, nvs service install, --config, --data, data folder, .nvsdata, file_cache_dir, opcache, cache, log, logs, log target, Windows, Linux, PATH, service account, elevated prompt, administrator, admin rights, User Account Control, UAC, sudo, root, --no-elevate
---

# Set up a server

Three steps set up a server:

1. Put `nvs` in a folder, for example `D:\srv\novis\nvs.exe` or `/opt/novis/nvs`. Put that folder
   on the `PATH`.
2. In the project folder, run `nvs init`.
3. Run `nvs service install <name> -- serve <entry file>`. Give the entry file as a full path. The
   command asks for admin rights itself.

On Windows:

```text
cd D:\srv\shop
nvs init
nvs service install shop -- serve D:\srv\shop\public\index.nvs
```

On Linux and macOS:

```text
cd /srv/shop
nvs init
nvs service install shop -- serve /srv/shop/public/index.nvs
```

The commands work in any order. `nvs init`, `nvs service install` and `nvs run` all use the same
data folder.

A service does not start in the project folder. Without `--config`, it reads the `nvs.toml` in the
data folder. To use the project file instead, add `--config` with its full path after `--`.

You need `icacls` or `chmod` only for folders you choose yourself, and for a folder above one of
them. The section *Folders you choose yourself* shows that layout.

## What `nvs init` does

`nvs init` creates the data folder with its subfolders `cache`, `tmp`, `lsp` and `logs`. The data
folder is `.nvsdata` next to `nvs`, or the folder that `--data` names. `nvs run` creates it in the
same way. Then `nvs init` writes the project file `nvs.toml` into the current folder.

It prints one line for each path:

    created the data folder `D:\srv\novis\.nvsdata`
    created the project file `D:\srv\shop\nvs.toml`

You can run it again at any time. It then creates nothing and overwrites nothing. It prints the
same lines with `kept` instead of `created`, and the exit status is `0`.

`nvs init` checks the folder it writes into. `nvs serve` makes the same check later. If another
account can change that folder, `nvs init` asks:

    Restrict `D:\srv\shop` so only you and administrators can change it? [Y/n]

On Linux and macOS, the question says `root` instead of `administrators`. An empty answer is yes.
`nvs init --yes` answers yes without asking. After the change, it prints:

    restricted `D:\srv\shop` so only you and administrators can change it

On Windows, the change is two `icacls` commands: `/inheritance:d`, then `/remove:g <SID>`. On
Linux and macOS, it is `chmod go-w`. These are the same commands that the `help:` line of the
error prints.

`nvs init` asks only when standard input and standard error are a terminal. Without a terminal, or
when you answer no, it prints the commands as help. Then it prints an `error:` line, and the exit
status is `1`.

`nvs init` never changes a folder above the one it writes into. When the folder belongs to another
account, the system may not allow the change. Then `nvs init` asks for admin rights and runs again
with `--yes`.

## Admin rights

These commands ask for admin rights when they need them:

- `nvs service install`, `nvs service uninstall`, `nvs service start` and `nvs service stop`
- `nvs init`, when it must change a folder that your account cannot change

A command with `--dry-run` never asks. `nvs run`, `nvs serve`, `nvs test` and every other command
never ask. They run your program, and your program should not run as administrator.

On Windows, the command shows the Windows prompt for admin rights (User Account Control). After
you accept it, the output appears in the same window. The exit status is the status of the command
that ran with admin rights. On Linux and macOS, the command runs again with `sudo` in the same
terminal.

Sometimes no prompt can show. Examples are an SSH session, a CI job, a program without a console,
or the option `--no-elevate`. Then the command asks nothing. On Windows it prints
`run this in a terminal started as administrator:`, then the command. On Linux and macOS it prints
`run this as root:`, then the command with `sudo` in front. The exit status is `1`.

# The layout

## The data folder

Novis keeps its own files in one folder, the **data folder**. By default it is the folder `.nvsdata`
next to the `nvs` program. The option `--data <folder>` names another folder. Every command accepts
it, and a relative path starts at the current folder.

| In the data folder | Contains | The setting that moves it |
|---|---|---|
| `nvs.toml` | the configuration file that `nvs` reads when the current folder has no `nvs.toml` | `--config` |
| `cache` | compiled programs | `[opcache] file_cache_dir` |
| `tmp` | the folders that `Core\IO::temporaryDir` creates | `[io] temp_root` |
| `lsp` | files for the editor | the editor's `nvs.stubs.dir` setting |
| `logs` | `nvs.log`, the log of a service | `[log] target` |

`nvs init`, `nvs run`, `nvs serve`, `nvs test`, `nvs build`, `nvs check` and `nvs lsp` create the
data folder and its four subfolders when they do not exist. When `nvs` creates the folder, only the account
that runs `nvs` can open it. On Windows, `Administrators` and `SYSTEM` can open it too. Other
commands, such as `nvs config check`, never create it.

If `nvs` cannot create or use the data folder, it prints one warning and runs without it. Programs
then run without the compile cache, and `Core\IO::temporaryDir` throws an error. This happens when
the `nvs` program is in a folder that your account cannot write to, for example `/usr/local/bin`.
Pass `--data` with a folder that only your account can change.

`nvs test` and `nvs lsp-test` need a folder for temporary files. Without a usable data folder and
without `[io] temp_root`, they stop with an error.

## Folders you choose yourself

On a server you can also choose each folder yourself. `nvs init` does not restrict these folders.
You set their permissions with the commands in the sections *Windows* and *Linux and macOS*. A host then needs four folders. They can have any
names and be anywhere.

| Folder | Contains | Who writes to it | Who reads it |
|---|---|---|---|
| binary | `nvs` (`nvs.exe` on Windows) | administrators | everyone, so that every account can run `nvs` |
| config | `nvs.toml`, included files, secret files | administrators | the account that runs `nvs`, and nobody else if the files are private |
| cache | compiled programs | the account that runs `nvs` | not important |
| logs | log files | the account that runs `nvs` | your choice |

The simplest layout puts `config`, `cache` and `logs` inside the binary folder:

    D:\srv\novis\nvs.exe            /opt/novis/nvs
    D:\srv\novis\config\nvs.toml    /opt/novis/config/nvs.toml
    D:\srv\novis\cache\             /opt/novis/cache/
    D:\srv\novis\logs\              /opt/novis/logs/

Put the binary folder on the `PATH`. Then `nvs` runs from any directory, and `--config` tells it
where the configuration is:

    nvs --config D:\srv\novis\config\nvs.toml init
    nvs --config D:\srv\novis\config\nvs.toml serve D:\srv\app\index.nvs

The file after `serve` is optional when the configuration has `[[server.mount]]` blocks. With no
file, `nvs serve` serves every file that those blocks mount:

    nvs --config D:\srv\novis\config\nvs.toml serve

If the configuration has no `[[server.mount]]` block, `nvs serve` needs the file.

A configuration with `[[server.mount]]` blocks must set `root` in the `[server]` block. Every
mounted file is found under that folder. Without `root`, the server does not start:

```toml
[server]
root = "D:/srv/www"

[[server.mount]]
scan = "*/public/index.nvs"
prefix = "/{1:lower}"
```

With `--config`, `nvs init` writes `nvs.toml` to that path, and every key in it is present and
commented out. The folder for the file must already exist. `nvs init` checks and restricts that
folder as the section *What `nvs init` does* shows. `nvs run` and the other commands never
write into the current directory. When they find no configuration file, they write the same file
into the data folder.

Each key's line ends with a short note. `# default` means the value shown is what Novis uses when
the key is not set, so removing the `#` changes nothing. `# default: no cap` (or `off`, or another
short phrase) means Novis uses that when the key is not set, and the value shown is one you could
write instead. `# example` means there is no default: nothing is granted, mounted or opened until
you write the key.

Two keys in that file point at the other two folders. Write paths with `/`, on Windows too:

```toml
[opcache]
file_cache_dir = "D:/srv/novis/cache"

[log]
target = "file:D:/srv/novis/logs/nvs.log"
```

# What Novis checks

Novis checks **who owns a folder and who can write to it**. It never checks who can read or run
something. You decide that yourself.

A configuration file can allow a program to read files, open network connections and start other
programs. An account that can change the file can allow itself all of that. The same is true for
the compile cache, because Novis runs the compiled programs it finds there.

A file or folder passes the check when both of these are true:

| | Windows | Linux and macOS |
|---|---|---|
| The owner is | the account that runs `nvs`, `Administrators`, a member of `Administrators`, or `SYSTEM` | the account that runs `nvs`, or `root` |
| Nobody else can write | none of `Everyone`, `Authenticated Users`, `Users`, `Guests` and `ANONYMOUS LOGON` can write, delete, or change the permissions or the owner | the mode has no write bit for the group or for others |

An entry for one named account, for example your own, never fails the check. Only the groups in
the table do.

This is what each command checks:

| Command | Checks | If the check fails |
|---|---|---|
| `nvs serve`, at the start and at each reload | every configuration file, and the folder that contains it | the server does not start, or it keeps its configuration and logs `E0607` |
| `nvs init` | the folder it writes into | it asks to restrict the folder. Without a terminal, or after the answer no, nothing is written and the exit status is `1` |
| `nvs run`, `nvs serve`, `nvs test`, `nvs build`, `nvs check`, `nvs lsp` | the data folder. For a folder that `--data` names, **also the folder that contains it** | one `warning:` line. The command runs without the data folder |
| every command that compiles a program | the cache folder, **and the folder that contains it** | a `warning:` line when `file_cache_dir` is set. The program runs, and it is compiled again on every start |

`nvs run`, `nvs check`, `nvs test` and `nvs config check` do not check the configuration files.

The check on the cache folder, and on a data folder that `--data` names, also looks at the folder
one level up. That folder fails only when
another account can delete or rename the folders in it. Other accounts may still create files and
folders there.

| | Windows | Linux and macOS |
|---|---|---|
| The folder one level up fails when | one of the groups above has the right to delete subfolders and files, to change the permissions, or to change the owner | the mode has a write bit for the group or for others, and the sticky bit is not set |

The right to change, which Windows gives `Authenticated Users` on a drive that is not the system
drive, does not fail the folder one level up. The cache folder itself still fails if it gets that
right from the folder above it. `/tmp` passes on Linux and macOS, because it has the
sticky bit. If `D:\srv\novis\cache` has the correct permissions and `D:\srv\novis` does not, the
warning names `D:\srv\novis`. That is the folder to change.

`nvs init` checks the same folder that `nvs serve` checks later. It does not check the folder
above it, so it never asks you to change a folder like `D:\srv` that other projects share.

The default data folder, `.nvsdata` next to `nvs`, is the one exception. The folder that contains it
is the folder of the `nvs` program, and Novis does not check that folder. An account that can change
that folder can already replace `nvs` itself.

The log folder is not checked.

# Windows

On every drive except the system drive, Windows gives the group `Authenticated Users` the right to
change every new folder. That means every account on the computer can replace `nvs.exe` and edit
`nvs.toml`. Novis refuses such a folder.

`nvs init` restricts the folder it writes into, so the three steps at the top need none of the
commands below. Novis does not check the binary folder itself. On a drive other than the system
drive, protect it with step 2. The other steps are for folders you choose yourself, and for a
folder above one of them.

Use the SID of a group in `icacls`, not its name. Windows translates group names: a German Windows
calls `Authenticated Users` "Authentifizierte Benutzer", and `icacls` does not find the English
name there. A SID starts with `*` and is the same on every Windows.

| Group | SID |
|---|---|
| `Authenticated Users` | `*S-1-5-11` |
| `Users` | `*S-1-5-32-545` |
| `Everyone` | `*S-1-1-0` |
| `Administrators` | `*S-1-5-32-544` |

Run these commands in a Command Prompt that was started with **Run as administrator**. In
PowerShell, put quotes around an argument that contains parentheses: `"svc-novis:(OI)(CI)RX"`.

1. Create the folders.

       mkdir D:\srv\novis\config D:\srv\novis\cache D:\srv\novis\logs

2. Protect the binary folder. The first command stops the folder from taking permissions from
   `D:\`. The second removes the right to change it. `Users` keeps the right to read and run, so
   every account can still run `nvs`. `config`, `cache` and `logs` take the same permissions.

       icacls D:\srv\novis /inheritance:d
       icacls D:\srv\novis /remove:g *S-1-5-11

3. Make the configuration private. After this, only administrators and the accounts you add can
   read it.

       icacls D:\srv\novis\config /inheritance:d
       icacls D:\srv\novis\config /remove:g *S-1-5-32-545

4. Give the account that runs `nvs` its rights: read in `config`, change in `cache` and `logs`.
   Replace `svc-novis` with the account name.

       icacls D:\srv\novis\config /grant svc-novis:(OI)(CI)RX
       icacls D:\srv\novis\cache /grant svc-novis:(OI)(CI)M
       icacls D:\srv\novis\logs /grant svc-novis:(OI)(CI)M

5. Check the result. `icacls D:\srv\novis` must not show `Authenticated Users` or `Everyone`, and
   `Users` must show `(RX)` only. `dir /q D:\srv` shows the owner of each folder.

`icacls <folder> /remove:g <SID>` removes every right of that group, also the right to read. To
give reading and running back to all accounts, run
`icacls <folder> /grant *S-1-5-32-545:(OI)(CI)RX`.

Three things about accounts:

- An administrator account has administrator rights only in a prompt started with **Run as
  administrator**, or after you accept the Windows prompt for admin rights. In a normal prompt it
  is an ordinary account. If a command reports `Access is denied` (`os error 5`) in a normal
  prompt, either use an administrator prompt or give your own account the right by name, as in
  step 4.
- A folder that you created is owned by your account. That passes while you run `nvs` yourself.
  When `nvs serve` runs as a service under another account, the owner must be that account,
  `Administrators`, a member of `Administrators`, or `SYSTEM`. So a folder that an administrator
  created passes. In a prompt started with **Run as administrator**,
  `icacls <path> /setowner *S-1-5-32-544` makes `Administrators` the owner.
- After step 4, the account that runs `nvs` can change `cache` and `logs`. If you run `nvs` with
  this configuration under a different account, `nvs` does not use that cache. It prints a warning
  and compiles each program again. Another account could have written the files in that cache.

# Linux and macOS

Here the check reads the owner and the mode. `nvs init` restricts the folder it writes into, so
the three steps at the top need none of the commands below. They are for folders you choose
yourself, and for a folder above one of them. Replace `novis` with the account that runs `nvs`.

```text
sudo install -d -o root  -g root  -m 0755 /opt/novis
sudo install    -o root  -g root  -m 0755 nvs /opt/novis/nvs
sudo install -d -o root  -g novis -m 0750 /opt/novis/config
sudo install -d -o novis -g novis -m 0755 /opt/novis/cache
sudo install -d -o novis -g novis -m 0755 /opt/novis/logs
```

`/opt/novis/config` is owned by `root`. The group `novis` can read it and cannot write to it, and
other accounts cannot open it. Give `nvs.toml` the mode `0640` and the same owner and group.

A mode with a write bit for the group or for others fails the check: `0775`, `0777`, `0664`. Fix
it with `chmod go-w <path>`.

# When something is refused

Every message names a path. **Change the permissions of that path**, also when it is not the
folder you were working in. The `help:` line under the message prints the command to run, with
the path filled in.

| Message | Meaning | What to do |
|---|---|---|
| `... grants write access to ...` (Windows), `... is group-writable`, `... is world-writable` or `... is group- and world-writable` (Linux and macOS) | a group of ordinary accounts can write to the named path | remove that right from the named path. On Windows the message prints the SID to use |
| `... is owned by ..., which is neither this account ...` | the owner of the named path is another ordinary account | make the owner the account that runs `nvs`, or an administrator. On Windows, run `icacls <path> /setowner *S-1-5-32-544` in a prompt started with **Run as administrator** |
| `Access is denied` or `Permission denied` (`os error 5`, `os error 13`) | the permissions are strict enough, and **your** account cannot write there | use an administrator prompt or `sudo`, or give your account the right. The section *Admin rights* lists the commands that ask for admin rights themselves |
| `warning: [opcache] file_cache_dir ... is not used` | the cache folder, or the folder that contains it, failed the check | fix the named path. Until then the program works and starts more slowly |
| `warning: Novis cannot use its data folder ...` | the data folder could not be created, or it failed the check | fix the named folder, or pass `--data` with a folder that only your account can change. Until then programs run without the compile cache |
