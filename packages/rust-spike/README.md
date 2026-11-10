# Rust spike: two stacks, three components

This folder holds a time-boxed spike. It builds the same three Astryx
components, Button, Switch and TextInput, on two Rust-based stacks. Then it
compares the results against the same metrics.

## Why

Astryx needs to reach screens that the web stack does not reach well: desktop
apps, handheld and embedded devices, and native mobile shells. Before we
commit to one approach, we want evidence for two questions:

- Can an existing cross-platform UI framework carry Astryx's real component
  behavior, look and accessibility?
- What does our own Rust stack (`packages/rust`) cost for the same work?

## The two stacks

| Folder | Stack | What it is |
| --- | --- | --- |
| `exact/` | [Exact](https://github.com/expo/exact) (Contract language), pinned at commit `94b8671f70fe2e25b307a40e7fce66dc27c9f802` | Apps written in Contract. By default each host draws its own platform controls. The spike also tests an Astryx-drawn variant. |
| `native/` | Our stack in `packages/rust` | Astryx components drawn by the app itself, on desktop, web (WebAssembly) and headless targets. |

Exact is MIT-licensed. This repository does not vendor it. Code under `exact/`
uses Exact by path from a separate checkout at the pinned commit.

## Scope

- Components: Button, Switch and TextInput. Each follows Astryx's current
  component specs and docs.
- Surfaces: web, Linux, and the macOS and iOS hosts where they can be built.
- Both stacks are measured on one scorecard. It covers visual parity,
  behavior parity, accessibility, input, motion, bundle size, start time,
  build time, and authoring effort.

The component ledger and the metric definitions are fixed before
implementation starts. A change to them is a scope change.

## Status

Build phase. Component code lands on the lane branches first.

## Branches and how changes land

| Branch | Who writes it | Paths it may change |
| --- | --- | --- |
| `rust-spike/main` | QA lane: merges of the lane branches, the README, `qa/` | `packages/rust-spike/**` |
| `rust-spike/exact` | Exact lane | `packages/rust-spike/exact/**` |
| `rust-spike/native` | Native lanes (infrastructure and UI share it) | `packages/rust-spike/native/**`, and `packages/rust/**` only when no other way exists |

Every push goes through one maintainer-run push script. It refuses a push when:

- the push is not a fast-forward of the remote branch (nothing is ever force-pushed);
- a new commit changes a path outside the branch's allowlist;
- a new commit touches `pr-assets`;
- a new commit's added lines, messages or file names match the leak check
  (internal domains, host names, user names, internal ids).

Lanes that share a branch fetch it and merge it before they push. The QA lane
merges the lane branches into `rust-spike/main` with merge commits after each
wave, then runs both stacks' tests.

## Rules for this folder

- This is a public repository. Do not add internal links, hostnames, user
  names or other private identifiers.
- Do not copy Exact's source into this repository. Refer to it by commit.
- Each lane works on its own branch. The QA lane merges the lane branches into
  `rust-spike/main`.
- Measurement code lives in `qa/`.
