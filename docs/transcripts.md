# Transcripts: `cargo xtask transcript`

The utilities (spec §31, plan/utilities.md) are proved by transcripts: one case of a command with its argv,
environment, stdin and files, and the outcome the reference gave. The harness runs Liella's command and the reference
in private roots of their own and compares exit status, stdout, stderr and the files written (UTIL-1, UTIL-HARNESS).

```
cargo xtask transcript check  <file.transcript | dir>... [--reference <command>=<path>]... [--liella <command>=<path>]...
cargo xtask transcript record <file.transcript | dir>... [--reference <command>=<path>]...
```

* `check` runs Liella's side and, where the reference is installed (`/usr/sbin/<command>`, then `/usr/bin/<command>`),
  the reference. It reports every difference between the reference and Liella, and every difference between the
  reference and the record (a stale record fails too: re-record only after finding why). Without the reference it
  compares Liella with the record and says so on the `ok` line. Exit 0 when all hold, 1 on any `MISMATCH` or `ERROR`,
  2 on a usage error. A transcript that needs namespaces on a host without unprivileged user namespaces prints
  `SKIPPED <file>: needs user and mount namespaces: <why>`.
* `record` runs the reference and replaces the outcome at the end of the file; everything before it stays as written.
* Liella's side: `ldconfig` is a stub inside the xtask binary (`-p [-C CACHE]` only, over the loader's cache reader)
  until UTIL-3; other commands need `--liella` until their story adds them.
* The harness is part of xtask (std only, no third-party code; it already links `liella-ld`, whose writer makes
  `cache:` items and whose reader the stub uses). Its comparator and runner have unit tests (`cargo test -p xtask --bin
  xtask transcript::`), proved by the plants in `handoff/UTIL-HARNESS-plants.txt`; `xtask/tests/transcripts.rs` runs
  the committed transcripts.

## The file

UTF-8 text, one `key: value` directive per line; `#` lines and blank lines are skipped. Values are escaped: `\\`,
`\t`, `\r`, `\n`, `\xHH`; a value split on spaces (paths, `entry:` fields) writes a space as `\x20`. A **block** is
the `|` lines after its directive: `| text` is one line ending in a newline, `|` an empty line, `|~ text` a last line
without a newline; no lines means empty.

| Directive | Meaning |
|---|---|
| `name: <text>` | what the case shows (required) |
| `clause: <id>` | the spec clause it covers, one or more; `none (<why>)` if none (required) |
| `command: <name>` | the bare utility name; argv[0] on both sides (required) |
| `arg: <value>` | the next argument |
| `env: NAME=<value>` | one variable; the command gets exactly these, nothing inherited |
| `stdin:` + block | the standard input (default: empty) |
| `namespace: none \| user \| root` | `user`/`root`: run in an unprivileged user and mount namespace, as the caller's own uid and gid or as uid 0 inside it |
| `bind: <path in root> <absolute path>` | bind-mount a file or directory of the private root over a system path (needs a namespace) |
| `dir: <path>`, `file: <path>` + block, `link: <path> -> <target>` | create an item in the private root before the run |
| `cache: <path>`, `generator: <text>`, `entry: 0x<flags> <key> <path> [hwcaps=<name>]` | a loader cache written by Liella's writer (`liella_ld::cache::write`, D-27.43), entries in configuration order |
| `normalise: <literal> => <replacement>` | one more literal replacement in both outcomes (say why in a comment) |

The outcome comes last and is written by `record`: `status: exit <n>` or `status: signal <n>`, `stdout:` and
`stderr:` with blocks, then one `written-*` line per path that was created, changed or removed, sorted by path:
`written-file: <path>` + block (text files up to 64 KiB) or `written-file: <path> sha256 <hex> size <n>`,
`written-dir:`, `written-link: <path> -> <target>`, `written-other: <path> mode <octal>`, `written-removed:`.

## What is normalised

Only this, by default: the private root's path is replaced by `@ROOT@` in stdout, stderr, written files and link
targets (each side has its own root). `@ROOT@` in arguments, environment values, stdin and `link:` targets stands for
that path. Each `normalise:` line adds one literal replacement, applied to both sides after the root. Nothing else
(no whitespace folding, no sorting of output, no masking of numbers).

## Not covered yet

File modes and owners of written files, a hang limit for a command that never ends (the job's own limit applies),
and identity strings (Q3 of plan/utilities.md: a `normalise:` line per transcript until the stories settle them).
