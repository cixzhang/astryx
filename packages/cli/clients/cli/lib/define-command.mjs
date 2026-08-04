// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file The CommandDoc -> Commander converter. One place turns a typed
 * `CommandDoc` (+ the FunctionDoc it wraps, for inherited param descriptions)
 * into a configured Commander command, so `--help` and the manifest can be
 * sourced from the same colocated docs that feed `astryx docs` and the docsite.
 *
 * The CommandDoc type lives in `@astryxdesign/cli/authoring` with no Commander
 * dependency; this converter is the CLI-side adapter. Only the metadata is
 * generic — each command still supplies its own action (the thin wrapper that
 * calls the api function and renders).
 *
 * @input a CommandDoc (+ optional FunctionDoc) + an action
 * @output a configured commander Command added to the given parent
 * @position packages/cli/clients/cli/lib — CLI command converter
 */

/**
 * Build a Commander command from a CommandDoc and attach it to `parent`.
 *
 * @param {import('commander').Command} parent - program or a group command.
 * @param {import('@astryxdesign/cli/authoring').CommandDoc} doc
 * @param {{fn?: import('@astryxdesign/cli/authoring').FunctionDoc, action?: (...args: any[]) => unknown}} [impl]
 * @returns {import('commander').Command} the created command.
 */
export function defineCommand(parent, doc, {fn, action} = {}) {
  // The command token is the last path segment ("theme build" -> "build"),
  // since subcommands are added to their group command, not the program.
  const token = doc.name.split(' ').pop();
  const argSpec = (doc.args ?? [])
    .map(a => {
      const inner = a.variadic ? `${a.name}...` : a.name;
      return a.required ? `<${inner}>` : `[${inner}]`;
    })
    .join(' ');

  const cmd = parent.command(argSpec ? `${token} ${argSpec}` : token);
  if (doc.summary) cmd.description(doc.summary);

  const paramDesc = (/** @type {string | undefined} */ name) =>
    (fn?.params ?? []).find(p => p.name === name)?.description ?? '';

  for (const arg of doc.args ?? []) {
    const argument = cmd.registeredArguments?.find(a => a.name() === arg.name);
    const desc = arg.description ?? paramDesc(arg.param);
    if (argument && desc) argument.description = desc;
  }

  for (const o of doc.options ?? []) {
    const desc = o.description ?? (o.param ? paramDesc(o.param) : '');
    const option = cmd.createOption(o.flag, desc);
    if (o.choices) option.choices(o.choices);
    if (o.default != null) option.default(o.default);
    cmd.addOption(option);
  }

  if (doc.examples?.length) {
    const lines = doc.examples.map(e => `  ${e.cli}`).join('\n');
    cmd.addHelpText('after', `\nExamples:\n${lines}`);
  }

  if (action) cmd.action(action);
  return cmd;
}
