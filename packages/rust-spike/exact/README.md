# Astryx on Exact (Contract)

The Exact lane of the Rust spike: Astryx components written as an Exact
Contract library, plus apps that show them.

| Path | What it is |
| --- | --- |
| `astryx-layers/` | `@astryx-spike/astryx-layers`: components that hide their content until opened (Popover, Collapsible, and later Tooltip, Dialog, menus). Exact holds a library to always showing its slot, so these are a plain package beside the library. |
| `astryx/` | The library package (`@astryx-spike/astryx`): `.contract` components and `astryx.tamber`, Astryx's default theme as Tamber tokens. |
| `tokens/` | Astryx themes resolved to light and dark values (one JSON per theme), the importer's report, and `themes.tamber`: the `theme` axis an app pastes into its `app.tamber`. |
| `tools/astryx-tamber.mjs` | Writes `astryx/astryx.tamber` and `tokens/themes.tamber` from `tokens/*.json` through `contract tamber import astryx`. |
| `apps/astryx-gallery/` | Every ported family's visual cases, one per page, for QA. |
| `apps/astryx-shell-nav/` | The `shell-nav` template, rebuilt. |
| `apps/astryx-settings-dialog/` | The `settings-dialog` template, rebuilt. |
| `STATUS.md` | Each ledger row: ported, workaround or blocked, with evidence. |

## Setup

Exact is not vendored. Check it out at
`94b8671f70fe2e25b307a40e7fce66dc27c9f802` and link it here:

```sh
ln -s /path/to/exact .exact-sdk
cargo build -q -p contract --manifest-path .exact-sdk/Cargo.toml
bun install                      # installs the library where astryx-layers resolves it
cd apps/astryx-gallery && bun install
```

The gallery's cases are data in `apps/astryx-gallery/cases.contract`, written by
`bun tools/gallery-cases.mjs` from QA's case files (or `tools/families.mjs` smoke cases).
Icons come from `bun tools/astryx-icons.mjs`; `STATUS.md` from `bun tools/status.mjs`.

## Run

```sh
cd apps/astryx-gallery
bun exact.mjs contract build app.contract --json --errors-only   # compile
bun exact.mjs web-build                                          # the web build
bun exact.mjs agent web --url "/?case=<id>&scheme=dark&theme=butter" "clock data" "clock +1" "screenshot out.png"
bun exact.mjs test web                                           # app.test.contract
```

Linux host (headless). Exact's generated Linux shell bakes through its TypeScript path,
which needs a Hermes compiler built for glibc 2.36 or later; `linux-host/` bakes with an empty
Rust data source instead:

```sh
BIN=$(apps/astryx-gallery/linux-host/build.sh)
cd .exact-sdk && EXACT_APP_DIR=$PWD/../apps/astryx-gallery EXACT_LINUX_BIN=$BIN \
  bun scripts/agent.mjs linux --url "/?case=<id>" "clock data" "clock +1" "screenshot out.png"
```

A gallery page is `/?case=<id>&scheme=light|dark&theme=<astryx theme>`. The
component sits in a box with `testId="case"` and 16 px padding on the theme's
page background. With no `case`, the page lists every case.

## Theme tokens

`tokens/*.json` come from the published `@astryxdesign/theme-*@0.7.11`
packages and `@astryxdesign/core@0.7.11` (the newest published; the spike's
Astryx source is one patch later), resolved with Exact's
`scripts/astryx-tokens.mjs`. Regenerate with:

```sh
EXACT_SDK=.exact-sdk bun tools/astryx-tamber.mjs
```
