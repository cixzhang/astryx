// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file Template-transform parser — the load-boundary validator for the module
 * an integration points its manifest `templateTransform` at. Zod is sealed
 * here; consumers call `parseTemplateTransform` or import the
 * {@link AstryxTemplateTransform} type.
 *
 * The public type is generic (typed `wrap.props`) and generics erase at
 * runtime, so this schema validates the concrete runtime shape — the hand-
 * written type carries the compile-time (author-facing) safety.
 */

import {z} from 'zod';
import {formatZodError} from '../_shared/errors.mjs';

/** @typedef {import('./type').AstryxTemplateTransform} AstryxTemplateTransform */

/**
 * A statically-renderable prop value: primitives, or JSON-shaped
 * objects/arrays. Recursive via z.lazy.
 * @type {import('zod').ZodType<import('./type').TemplateWrapPropValue>}
 */
const propValueSchema = z.lazy(() =>
  z.union([
    z.string(),
    z.number(),
    z.boolean(),
    z.null(),
    z.array(propValueSchema),
    z.record(z.string(), propValueSchema),
  ]),
);

const wrapSchema = z
  .object({
    component: z
      .string()
      .regex(
        /^[A-Za-z_$][A-Za-z0-9_$]*$/,
        'component must be a valid identifier (e.g. "AppFrame")',
      ),
    from: z.string().min(1, 'from must be a non-empty module specifier'),
    importKind: z.enum(['named', 'default']).optional(),
    props: z
      .record(
        z
          .string()
          .regex(
            /^[A-Za-z_$][A-Za-z0-9_$-]*$/,
            'prop name must be a valid JSX attribute name',
          ),
        propValueSchema,
      )
      .optional(),
  })
  .strict();

const scopeSchema = z
  .object({
    types: z.array(z.enum(['page', 'block'])).optional(),
    include: z.array(z.string()).optional(),
    exclude: z.array(z.string()).optional(),
    packages: z.array(z.string()).optional(),
  })
  .strict();

const templateTransformSchema = z
  .object({
    description: z.string().optional(),
    appliesTo: scopeSchema.optional(),
    // A single wrapper or a non-empty stack (outermost first). Normalize a
    // single wrapper to a one-element array up front so validation has a single
    // branch — this keeps error paths precise (`wrap.0.component` /
    // `wrap.0.props`) instead of the generic "invalid union" a `z.union` gives.
    wrap: z.preprocess(
      value => (value == null || Array.isArray(value) ? value : [value]),
      z
        .array(wrapSchema)
        .min(1, 'wrap must declare at least one component'),
    ),
  })
  .strict();

/**
 * Validate an unknown value as an Astryx template transform, or throw.
 *
 * @param {unknown} input
 * @param {string} [label]
 * @returns {AstryxTemplateTransform}
 */
export function parseTemplateTransform(input, label = 'astryx template transform') {
  const result = templateTransformSchema.safeParse(input);
  if (!result.success) {
    throw new Error(formatZodError(label, result.error));
  }
  return result.data;
}
