// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file isImeKeyEvent.ts
 * @input Nothing — a pure predicate over a keyboard-event shape
 * @output Exports isImeKeyEvent
 * @position Leaf helper shared by every overlay's Escape handling. Lives on its
 *   own so the Layer dismissal stack can use it without importing the focus
 *   trap, which imports the stack back.
 *
 * SYNC: When modified, update:
 * - /packages/core/src/hooks/index.ts
 */

/**
 * Whether an Escape keydown should be ignored because it is cancelling an
 * in-progress IME composition. CJK/IME users press Escape to cancel
 * composition; that must not close the surrounding overlay. `keyCode === 229`
 * covers browsers that fire keydown before `isComposing` is set.
 */
export function isImeKeyEvent(event: {
  isComposing?: boolean;
  keyCode?: number;
}): boolean {
  return event.isComposing === true || event.keyCode === 229;
}
