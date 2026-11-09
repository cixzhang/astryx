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

Setup only. No component code is in this folder yet.

## Rules for this folder

- This is a public repository. Do not add internal links, hostnames, user
  names or other private identifiers.
- Do not copy Exact's source into this repository. Refer to it by commit.
- Each lane works on its own branch and merges into `rust-spike/main`.
