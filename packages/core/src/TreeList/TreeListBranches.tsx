// Copyright (c) Meta Platforms, Inc. and affiliates.

'use client';

/**
 * @file TreeListBranches.tsx
 * @input Uses React, StyleX, theme tokens
 * @output Exports TreeListBranches component (internal)
 * @position Presentational component for tree connector lines; consumed by TreeListItem.tsx
 *
 * SYNC: When modified, update these files to stay in sync:
 * - /packages/core/src/TreeList/TreeListItem.tsx
 * - /packages/cli/assets/templates/blocks/components/TreeList/ (showcase blocks)
 */

import * as stylex from '@stylexjs/stylex';
import {colorVars, spacingVars} from '../theme/tokens.stylex';
import {mergeProps} from '../utils';
import {themeProps} from '../utils/themeProps';

const LINE_WIDTH = 1;

/**
 * Branch margin from the left edge. No exact spacing token for 10px,
 * so we use calc(--spacing-2 + --spacing-0-5) = 8 + 2 = 10.
 */
const BRANCH_MARGIN = `calc(${spacingVars['--spacing-2']} + ${spacingVars['--spacing-0-5']})`;

/** Per-level indent width. Reads the public, themeable `--tree-list-indent`
 * lever set on the tree-list root (default `--spacing-4`, 16px), so the guide
 * lines stay aligned with the row indent when a theme retunes the step. */
const LEVEL_INDENT = 'var(--tree-list-indent)';

const styles = stylex.create({
  container: {
    height: '100%',
    position: 'absolute',
    width: spacingVars['--spacing-5'],
  },
  verticalLine: {
    borderRadius: 1,
    insetInlineStart: 0,
    margin: 'auto',
    position: 'absolute',
    insetInlineEnd: 0,
    width: LINE_WIDTH,
    backgroundColor: colorVars['--color-border-emphasized'],
  },
  // Guide segment spanning the full <li>. The extra `1px` (plus the inter-row
  // gap) lets the segment bridge into the next sibling so the connector reads
  // as one continuous line across the gap. With no row gap this is just the
  // original `100% + 1px`; with a themed `--tree-list-row-gap` the segment
  // grows to cover the gap the row box opened up.
  verticalFull: {
    height: 'calc(100% + 1px + var(--tree-list-row-gap, 0px))',
  },
  // Last-in-group connector: nothing sits below, so the bridging extension
  // would overhang into empty space once a row gap is opened. Pull the segment
  // back by half the gap so it ends at the row box's bottom edge (the row box
  // carries `--tree-list-row-gap` / 2 as its bottom margin inside the <li>).
  // At the default `0px` gap this is exactly `100% + 1px` — identical to a
  // non-last connector — so the default look is unchanged.
  verticalLast: {
    height: 'calc(100% + 1px - var(--tree-list-row-gap, 0px) / 2)',
  },
});

// =============================================================================
// Types
// =============================================================================

interface TreeListBranchesProps {
  ancestorsIsLast: ReadonlyArray<boolean>;
  isLast: boolean;
  nestedLevel: number;
}

// =============================================================================
// Components
// =============================================================================

/**
 * Renders vertical lines showing parent-child relationships in the tree.
 * Positioned in a full-height container to span the entire item including children.
 *
 * The line element carries the stable `astryx-tree-list-guide` theme target so a
 * theme can recolor or hide the connectors via `defineTheme` (e.g.
 * `backgroundColor` or `display: 'none'`) instead of hiding the built-in guides
 * and reimplementing them.
 */
export function TreeListBranches({
  ancestorsIsLast,
  isLast,
  nestedLevel,
}: TreeListBranchesProps) {
  return (
    <>
      {ancestorsIsLast.map((ancestorIsLast, level) => {
        const branchOffset = `calc(${BRANCH_MARGIN} + ${level} * ${LEVEL_INDENT})`;
        return (
          // Skip the level that the current-item connector occupies
          // (nestedLevel - 1), since that position is rendered below
          // with the correct terminus/continuation style.
          !ancestorIsLast &&
          level !== nestedLevel - 1 && (
            <div
              // eslint-disable-next-line @eslint-react/no-array-index-key -- tree branch levels are fixed positional connectors
              key={level}
              {...mergeProps(stylex.props(styles.container), {
                style: {
                  insetInlineStart: branchOffset,
                },
              })}>
              <div
                {...mergeProps(
                  themeProps('tree-list-guide'),
                  stylex.props(styles.verticalLine, styles.verticalFull),
                )}
              />
            </div>
          )
        );
      })}
      {nestedLevel > 0 && (
        <div
          {...mergeProps(stylex.props(styles.container), {
            style: {
              insetInlineStart: `calc(${BRANCH_MARGIN} + ${nestedLevel - 1} * ${LEVEL_INDENT})`,
            },
          })}>
          <div
            {...mergeProps(
              themeProps('tree-list-guide'),
              // The last item in a group has no sibling below, so its connector
              // is clamped to the row box's bottom edge (verticalLast) instead
              // of bridging into the inter-row gap — no overhang into empty
              // space. Every other row bridges the gap (verticalFull) so the
              // line stays continuous down to the next sibling.
              stylex.props(
                styles.verticalLine,
                isLast ? styles.verticalLast : styles.verticalFull,
              ),
            )}
          />
        </div>
      )}
    </>
  );
}

TreeListBranches.displayName = 'TreeListBranches';
