# tfold

A repository map that fits a token budget.

Point a coding agent at `tfold` instead of letting it explore. It prints the structure of a
directory, spending its budget where the code actually is, and it never guesses what anything means.

```
$ tfold . --budget 400

portable-agent-layer/  [code · git · 460 files · 159 tests hidden]
├── .agents/  (18 files)
├── .claude/  (7 files)
├── .codex/  (1 file)
├── .cursor/  (6 files)
├── .github/  (3 files)
├── .husky/  (4 files)
├── .opencode/  (1 file)
├── assets/  (177 files)
├── docs/  (1 file)
│   └── plans/  (1 file)
├── eval/  (27 files)
├── scripts/  (1 file)
├── src/  (192 files)
│   ├── cli/  (12 files)
│   ├── hooks/  (93 files)
│   ├── targets/  (12 files)
│   │   ├── claude/  (2 files)
│   │   ├── codex/  (2 files)
│   │   ├── copilot/  (2 files)
│   │   ├── cursor/  (2 files)
│   │   ├── opencode/  (3 files)
│   │   └── lib.ts
│   └── tools/  (75 files)
├── .gitattributes
├── .gitignore
├── .jscpd.json
├── .releaserc.json
├── .secretlintignore
├── .secretlintrc.json
├── AGENTS.md
├── biome.json
├── bun.lock
├── bunfig.toml
├── CHANGELOG.md
├── CLAUDE.md
├── commitlint.config.js
├── components.json
├── klint.rules.ts
├── klint.yaml
├── knip.json
├── LICENSE
├── package.json
├── README.md
├── stryker.config.mjs
└── tsconfig.json

~399 tokens
```

Raise the budget and the same call walks deeper into whichever subtree earns it.

That example, and every measurement in this README, is
[Portable Agent Layer](https://github.com/kovrichard/portable-agent-layer) — a 460 file
TypeScript repository. It is the corpus the token estimator was fitted against, so treat
the numbers as one repository's shape rather than a universal claim.

## Install

Run it without installing anything:

```bash
bunx tfold .
npx tfold .
```

Add it to a project so every agent working in that repo can rely on it:

```bash
bun add -d tfold
```

Or build from source:

```bash
cargo install tfold
```

The npm package is a launcher, not a reimplementation. The binary ships as a
per-platform optional dependency, so there is no build step and no `postinstall`
download — which is what makes it work under `bunx`, where lifecycle scripts are
blocked by default. Prebuilt for `linux-x64`, `darwin-x64`, `darwin-arm64` and
`win32-x64`; anywhere else, use `cargo install tfold`.

## Tell your agent about it

Once it is a dev dependency, two sentences in `AGENTS.md` (or `CLAUDE.md`) replace
the structure section of your README:

```markdown
## Orienting in this repo

Run `tfold .` for the structure, then `tfold <dir>` to drill into whatever
it points at. Do not use `ls` or `find` to map the repo — tfold is faster and
applies the repo's ignore rules.
```

The second sentence is the one that changes behaviour. Without it an agent makes
one call and falls back to habit.

## Use

Start wide, then drill into whatever the first call points at:

```bash
tfold .            # 795 tokens, 6 ms
tfold src/hooks    # 649 tokens, 2 ms
```

Two calls, 1,444 tokens, 8 ms. That replaces the `ls`, `find` and `cat` loop an agent otherwise
runs to answer the same question.

Any subdirectory works as a root. Pointed inside a git repository, `tfold` still applies the
repository's ignore rules, so `tfold src` hides what `git` hides.

## Why not `tree -L 2`

Depth is the wrong knob. On that repository, depth 2 costs 1,537 tokens and spends most of
them printing 150 individual test filenames, because `test/` is flat. The directory that matters,
`src/`, is four levels deep and shows up as four bare names.

`tfold` takes a budget instead and decides depth per subtree. Flat directories collapse to a
count. Deep ones get walked, and still carry the total, so `src/  (192 files)` tells you the
weight of a subtree whether or not you can see inside it.

## It does not guess

Every line is a fact about the filesystem. `tfold` never labels a directory with what it thinks
the code does, because that is where similar tools go wrong: a directory called `hooks` is not
necessarily React, and a confident wrong label costs more than no label.

The one rule that keeps it honest: a directory is expanded completely or collapsed, and either
way it reports how many files are under it.
Breadth is never truncated. Showing 12 of 160 files reads like the whole directory and is a lie by
omission. `test/  (160 tests hidden)` cannot mislead anyone.

## Flags

| flag | default | what it does |
| --- | --- | --- |
| `--budget <n>` | `800` | Token budget for the output. |
| `--include-tests` | off | Show test files instead of collapsing them to a count. |
| `--exclude <glob>` | none | Drop paths matching a glob. Repeatable. |
| `--since <ref>` | none | Mark what changed since a git revision and spend the budget there. |

Tests are excluded by default and always counted in the header, so you can see that they exist
without paying for their names.

`--exclude` layers on top of the ignore rules rather than replacing them, so a subtree that
`git` already hides stays hidden:

```bash
tfold . --exclude 'docs/**' --exclude vendor
```

An unparseable glob is rejected before any work happens, so a typo costs you an error rather
than a map that silently excludes nothing.

## Picking up a branch

`--since` compares against the merge base with a revision and reshapes the map around the result.
Changed files are marked, directories carry a count, and the ranking pulls the changed subtrees
into the budget ahead of larger untouched ones.

```
$ tfold . --since HEAD~2 --budget 300

tfold/  [code · git · 52 files · 1 tests hidden · 11 changed since HEAD~2]
├── .agents/  (9 files)
│   ├── hooks/  (8 files)
│   └── scripts/  (1 file)
│       └── check-lf.ts
├── .claude/  (1 file)
├── .codex/  (1 file)
├── .cursor/  (1 file)
├── .github/  (1 file)
├── .husky/  (3 files)
├── .opencode/  (1 file)
├── npm/  (10 files, 1 changed)
├── src/  (7 files, 5 changed)
│   ├── classify.rs
│   ├── collect.rs  *
│   ├── estimate.rs
│   ├── lib.rs  *
│   ├── main.rs  *
│   ├── render.rs  *
│   └── tree.rs  *
├── tools/  (4 files)
├── .gitattributes
├── .gitignore
├── .jscpd.json
├── .npmrc
├── .releaserc.json
├── biome.json
├── bun.lock
├── Cargo.lock  *
├── Cargo.toml  *
├── CHANGELOG.md  *
├── commitlint.config.js
├── knip.json
├── package.json
└── README.md  *

~294 tokens
```

It counts uncommitted edits and untracked files too, so it answers "where am I" rather than only
"what did I commit".

An unknown revision is an error. A directory that is not a git repository is not: the map comes out
in full and the header says `--since needs git, ignored`, because a map with no marks would
otherwise read as a branch that changed nothing.

## What it expands first

Directories are ranked, and the ranking only decides expansion order. It never changes a label, so
a bad guess costs relevance, not accuracy. Shallow beats deep, more files beats fewer, an entry
point (`index`, `main`, `mod`, `lib`, `cli`) or a manifest (`package.json`, `Cargo.toml`, `go.mod`)
earns a bonus, and dotfile directories, `vendor`, `dist` and `examples` are pushed down.

## Directories without git

`tfold` works on any directory: a documentation folder, a synced drive, an extracted archive.
It reads `.gitignore` files wherever it finds them, including nested ones, with or without a `.git`
directory present. The header says which mode it used, `git` or `walk`.

The map is also classified as `code`, `docs` or `mixed` from the file extensions, which is the only
inference in the tool and it is reported rather than acted on silently.

## Token counting

The footer estimate comes from a two constant linear model fitted against `cl100k_base`:

```
tokens_per_line = 2.82 + 0.2741 × chars
```

Measured error on this tool's own output is within 2% at budgets of 800 and 2000, and about 9% over
at 300. It is deliberately not a real BPE tokenizer: two floats beat a 1.6 MB table for a number
that only has to be close enough to allocate against. Re-fit it if you change the line format.

## Speed

6 ms on the 460 file repository above, measured over 20 runs. Comparable tools that build a
symbol or call graph take 4 to 6 seconds on the same repository, because they parse every file.
`tfold` reads no file contents at all.


