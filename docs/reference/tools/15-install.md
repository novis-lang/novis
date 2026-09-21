---
id: install
title: "Installing on a host: folders and permissions"
summary: where to put the `nvs` binary, the configuration, the compile cache and the logs on a server; which folder permissions Novis checks and when; the commands that set them on Windows and on Linux; and what each refusal message means
keywords: install, installation, setup, deploy, deployment, server, host, folder, directory, permissions, ACL, DACL, icacls, chmod, chown, owner, ownership, Authenticated Users, Users, Everyone, SID, S-1-5-11, S-1-5-32-545, inheritance, E0607, access denied, os error 5, nvs init, nvs serve, --config, file_cache_dir, opcache, cache, log, logs, log target, Windows, Linux, PATH, service account, elevated prompt, administrator
---

# The layout

A host needs four folders. They can have any names and be anywhere.

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

`nvs init` writes a `nvs.toml` in which every key is present and commented out. Without `--config`
it writes `nvs.toml` into the current directory. The folder must already exist, and an existing
file is never overwritten.

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
| The owner is | the account that runs `nvs`, `Administrators` or `SYSTEM` | the account that runs `nvs`, or `root` |
| Nobody else can write | none of `Everyone`, `Authenticated Users`, `Users`, `Guests` and `ANONYMOUS LOGON` can write, delete, or change the permissions or the owner | the mode has no write bit for the group or for others |

An entry for one named account, for example your own, never fails the check. Only the groups in
the table do.

This is what each command checks:

| Command | Checks | If the check fails |
|---|---|---|
| `nvs serve`, `nvs ctl reload` | every configuration file, and the folder that contains it | the server does not start, or the reload is refused, with `E0607` |
| `nvs init` | the folder it writes into, **and the folder that contains that folder** | nothing is written and the exit status is `1` |
| every command that compiles a program | the cache folder, **and the folder that contains it** | a `warning:` line when `file_cache_dir` is set. The program runs, and it is compiled again on every start |

`nvs run`, `nvs check`, `nvs test` and `nvs config check` do not check the configuration files.

The check on the cache folder and in `nvs init` also looks at the folder one level up. If
`D:\srv\novis\config` has the correct permissions and `D:\srv\novis` does not, the message names
`D:\srv\novis`. That is the folder to change.

The log folder is not checked.

# Windows

On every drive except the system drive, Windows gives the group `Authenticated Users` the right to
change every new folder. That means every account on the computer can replace `nvs.exe` and edit
`nvs.toml`. Novis refuses such a folder.

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

Two things about accounts:

- An administrator account has administrator rights only in a prompt started with **Run as
  administrator**. In a normal prompt it is an ordinary account. If `nvs init` reports
  `Access is denied` (`os error 5`) in a normal prompt, either use an administrator prompt or give
  your own account the right by name, as in step 4.
- A folder that you created is owned by your account. That passes while you run `nvs` yourself.
  When `nvs serve` runs as a service under another account, the owner must be that account,
  `Administrators` or `SYSTEM`.

# Linux and macOS

Here the check reads the owner and the mode. Replace `novis` with the account that runs `nvs`.

    sudo install -d -o root  -g root  -m 0755 /opt/novis
    sudo install    -o root  -g root  -m 0755 nvs /opt/novis/nvs
    sudo install -d -o root  -g novis -m 0750 /opt/novis/config
    sudo install -d -o novis -g novis -m 0755 /opt/novis/cache
    sudo install -d -o novis -g novis -m 0755 /opt/novis/logs

`/opt/novis/config` is owned by `root`. The group `novis` can read it and cannot write to it, and
other accounts cannot open it. Give `nvs.toml` the mode `0640` and the same owner and group.

A mode with a write bit for the group or for others fails the check: `0775`, `0777`, `0664`. Fix
it with `chmod go-w <path>`.

# When something is refused

Every message names a path. **Change the permissions of that path**, also when it is not the
folder you were working in.

| Message | Meaning | What to do |
|---|---|---|
| `... grants write access to ...` (Windows), `... is group-writable` or `... is world-writable` (Linux) | a group of ordinary accounts can write to the named path | remove that right from the named path. On Windows the message prints the SID to use |
| `... is owned by ..., which is neither this account nor ...` | the owner of the named path is another ordinary account | make the owner the account that runs `nvs`, or an administrator |
| `Access is denied` or `Permission denied` (`os error 5`, `os error 13`) | the permissions are strict enough, and **your** account cannot write there | use an administrator prompt or `sudo`, or give your account the right |
| `... it already exists, and it is never overwritten` | `nvs init` found a file at that path | edit the file, or delete it and run `nvs init` again |
| `warning: [opcache] file_cache_dir ... is not used` | the cache folder, or the folder that contains it, failed the check | fix the named path. Until then the program works and starts more slowly |
