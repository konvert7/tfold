# tfold

A repository map that fits a token budget.

Point a coding agent at `tfold` instead of letting it explore. It prints the structure of a
directory, spending its budget where the code actually is, and it never guesses what anything means.

```
$ tfold . --budget 400

portable-agent-layer/  [code · git · 460 files · 160 tests hidden]
  .agents/  (18 files)
  .claude/  (7 files)
  .codex/  (1 file)
  .cursor/  (6 files)
  .github/  (3 files)
  .husky/  (4 files)
  .opencode/  (1 file)
  assets/  (177 files)
  docs/  (1 file)
  eval/  (27 files)
  scripts/  (1 file)
  src/  (192 files)
    cli/  (12 files)
    hooks/  (93 files)
      handlers/  (21 files)
      lib/  (61 files)
      CompactRecover.ts
      LedgerCommit.ts
      LedgerSnapshot.ts
      LedgerUnapplied.ts
      LoadContext.ts
      PreCompactPersist.ts
      RtkWrap.ts
      SecurityValidator.ts
      SkillGuard.ts
      StopOrchestrator.ts
      UserPromptOrchestrator.ts
    targets/  (12 files)
      claude/  (2 files)
      codex/  (2 files)
      copilot/  (2 files)
      cursor/  (2 files)
      opencode/  (3 files)
      lib.ts
    tools/  (75 files)
  .gitattributes
  .gitignore
  .jscpd.json
  .releaserc.json
  .secretlintignore
  .secretlintrc.json
  AGENTS.md
  biome.json
  bun.lock
  bunfig.toml
  CHANGELOG.md
  CLAUDE.md
  commitlint.config.js
  components.json
  klint.rules.ts
  klint.yaml
  knip.json
  LICENSE
  package.json
  README.md
  stryker.config.mjs
  tsconfig.json

~397 tokens
```

Raise the budget and the same call walks deeper into whichever subtree earns it.

That example, and the timings below, are
[Portable Agent Layer](https://github.com/kovrichard/portable-agent-layer) — a 460 file
TypeScript repository. It is one of the six repositories the token estimator was fitted
against, so treat the numbers as one repository's shape rather than a universal claim.

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
tfold .            # 800 tokens, 6 ms
tfold src/hooks    # 591 tokens, 2 ms
```

Two calls, 1,391 tokens, 8 ms. That replaces the `ls`, `find` and `cat` loop an agent otherwise
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

It also draws no tree. `├── ` costs three tokens, `└── ` four, `│   ` two, and two spaces cost
one however many you stack. Indentation already says everything the glyphs said, so the glyphs
were 16% of a shallow map and 44% of a deep one, spent on nothing. Dropping them fits around 45%
more of the tree into the same budget.

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
| `--grep <pattern>` | none | Count literal matches per file and spend the budget where they are. |
| `-i`, `--ignore-case` | off | Match `--grep` regardless of case. |

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

tfold/  [code · git · 53 files · 1 tests hidden · 12 changed since HEAD~2]
  .agents/  (9 files)
    hooks/  (8 files)
    scripts/  (1 file)
      check-lf.ts
  .claude/  (1 file)
    settings.json
  .codex/  (1 file)
  .cursor/  (1 file)
  .github/  (1 file)
  .husky/  (3 files)
  .opencode/  (1 file)
  npm/  (10 files, 1 changed)
    native/  (4 files)
    tfold/  (2 files)
      bin/  (1 file)
        tfold.js
      package.json
    bootstrap-natives.mjs  *
    native-packages.mjs
    prepare-release.mjs
    stage.mjs
  src/  (8 files, 6 changed)
    classify.rs
    collect.rs
    estimate.rs  *
    lib.rs  *
    main.rs  *
    render.rs  *
    scan.rs  *
    tree.rs  *
  tools/  (4 files)
  .gitattributes
  .gitignore
  .jscpd.json
  .npmrc
  .releaserc.json
  biome.json
  bun.lock
  Cargo.lock  *
  Cargo.toml  *
  CHANGELOG.md  *
  commitlint.config.js
  knip.json
  package.json
  README.md  *

~296 tokens
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

## Finding where something lives

`grep -rn` answers the question at the wrong resolution. On a 700 file repository, `grep -rn prisma`
is 1,427 lines of output; `grep -rl` is 185 unordered paths. Neither tells you that it is
concentrated in one directory.

`--grep` counts literal matches per file and spends the budget where they are. Directories report
how many of their files matched, files report their matching line count.

```
$ tfold src --grep prisma

src/  [code · git · 464 files · 403 matching lines in 69 files]
  app/  (98 files, 1 matched)
  components/  (169 files, 3 matched)
  emails/  (5 files)
  hooks/  (7 files)
  lib/  (176 files, 63 matched)
    actions/  (14 files, 7 matched)
      credentials/  (6 files, 6 matched)
...
```

Then drill, the same as always:

```
$ tfold src/lib/dao --grep prisma --budget 300

dao/  [code · git · 20 files · 156 matching lines in 20 files]
  credentials/  (5 files, 5 matched)
    facebook.ts  (5)
    linkedIn.ts  (5)
    pinterest.ts  (6)
    threads.ts  (5)
    x.ts  (5)
  admins.ts  (2)
  audience.ts  (10)
  billing.ts  (5)
  content-performance.ts  (2)
  disconnect-account.ts  (7)
  invitations.ts  (16)
  mcp.ts  (6)
  media.ts  (8)
  notifications.ts  (3)
  orgs.ts  (2)
  post-flow.ts  (4)
  post-retry.ts  (9)
  posts.ts  (41)
  socials.ts  (7)
  team.ts  (8)

~166 tokens
```

A line holding the pattern twice counts once, so the number is matching lines, not occurrences.
Files over 2 MB and files with a NUL byte in the first 8 KB are skipped rather than scanned, which
is the same binary test `git` uses. Add `-i` to ignore case. It needs no git repository, unlike
`--since`, and the two compose: a file that changed *and* matches reads `db.ts  *  (24)`.

## Directories without git

`tfold` works on any directory: a documentation folder, a synced drive, an extracted archive.
It reads `.gitignore` files wherever it finds them, including nested ones, with or without a `.git`
directory present. The header says which mode it used, `git` or `walk`.

The map is also classified as `code`, `docs` or `mixed` from the file extensions, which is the only
inference in the tool and it is reported rather than acted on silently.

## Token counting

The footer estimate comes from a two constant linear model:

```
tokens_per_line = 1.03 + 0.357 × visible chars
```

Indentation is left out of the char count on purpose. A run of spaces collapses into a single
token however deep it goes, so depth is close to free and only the label and its summary get
billed. Fitted against `o200k_base` over 18 maps — six repositories at three budgets each — the
estimate lands within 3% on 15 of them, 8% under at the tightest budget and 9% over on a twelve
line map. It is deliberately not a real BPE tokenizer: two floats beat a 1.6 MB table for a number
that only has to be close enough to allocate against. Re-fit it if you change the line format.

## Speed

6 ms on the 460 file repository above, measured over 20 runs. Comparable tools that build a
symbol or call graph take 4 to 6 seconds on the same repository, because they parse every file.
`tfold` reads no file contents unless you ask it to with `--grep`, and that scan is a literal
substring search across threads, not a parse.


