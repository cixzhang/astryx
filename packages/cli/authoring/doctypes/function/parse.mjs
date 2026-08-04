// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file Function doc parser (stamped `type: 'function'`). Hooks and CLI/API
 * functions share the discriminant and the sealed schema, so this is the same
 * validator as `parseHook`, exported under the general name. Consumers call
 * `parseFunction` or use `parseDoc`.
 */

export {parseHook as parseFunction} from '../hook/parse.mjs';
