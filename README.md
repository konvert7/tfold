# treefold

A repository map that fits a token budget.

Point a coding agent at `treefold` instead of letting it explore. It prints the structure of a
directory, spending its budget where the code actually is, and it never guesses what anything means.

```
$ treefold . --budget 400

portable-agent-layer/  [code · git · 460 files · 160 tests hidden]
├── .agents/  (18 files)
├── .claude/  (7 files)
├── .codex/  (1 file)
├── .cursor/  (6 files)
├── .github/  (3 files)
├── .husky/  (4 files)
├── .opencode/  (1 file)
├── assets/  (177 files)
├── docs/
│   └── plans/  (1 file)
├── eval/  (27 files)
├── scripts/
│   └── build-skill-tools.ts
├── src/
│   ├── cli/  (12 files)
│   ├── hooks/  (93 files)
│   ├── targets/
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

~398 tokens
```

Raise the budget and the same call walks deeper into whichever subtree earns it.

## Install

```bash
cargo install treefold
```

## Use

Start wide, then drill into whatever the first call points at:

```bash
treefold .            # 789 tokens, 6 ms
treefold src/hooks    # 534 tokens, 2 ms
```

Two calls, 1,323 tokens, 8 ms. That replaces the `ls`, `find` and `cat` loop an agent otherwise
runs to answer the same question.

Any subdirectory works as a root. Pointed inside a git repository, `treefold` still applies the
repository's ignore rules, so `treefold src` hides what `git` hides.

## Why not `tree -L 2`

Depth is the wrong knob. On the repository above, depth 2 costs 1,537 tokens and spends most of
them printing 150 individual test filenames, because `test/` is flat. The directory that matters,
`src/`, is four levels deep and shows up as four bare names.

`treefold` takes a budget instead and decides depth per subtree. Flat directories collapse to a
count. Deep ones get walked.

## It does not guess

Every line is a fact about the filesystem. `treefold` never labels a directory with what it thinks
the code does, because that is where similar tools go wrong: a directory called `hooks` is not
necessarily React, and a confident wrong label costs more than no label.

The one rule that keeps it honest: a directory is expanded completely or collapsed to a count.
Breadth is never truncated. Showing 12 of 160 files reads like the whole directory and is a lie by
omission. `test/  (160 tests hidden)` cannot mislead anyone.

## Flags

| flag | default | what it does |
| --- | --- | --- |
| `--budget <n>` | `800` | Token budget for the output. |
| `--include-tests` | off | Show test files instead of collapsing them to a count. |

Tests are excluded by default and always counted in the header, so you can see that they exist
without paying for their names.

## What it expands first

Directories are ranked, and the ranking only decides expansion order. It never changes a label, so
a bad guess costs relevance, not accuracy. Shallow beats deep, more files beats fewer, an entry
point (`index`, `main`, `mod`, `lib`, `cli`) or a manifest (`package.json`, `Cargo.toml`, `go.mod`)
earns a bonus, and dotfile directories, `vendor`, `dist` and `examples` are pushed down.

## Directories without git

`treefold` works on any directory: a documentation folder, a synced drive, an extracted archive.
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

6 ms on a 460 file repository, measured over 20 runs. Comparable tools that build a symbol or
call graph take 4 to 6 seconds on the same repository, because they parse every file.
`treefold` reads no file contents at all.
