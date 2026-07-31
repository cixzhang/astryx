// Copyright (c) Meta Platforms, Inc. and affiliates.

'use client';

/**
 * @file TourStep.tsx
 * @input Uses React, Popover/Button/Text/Heading/Stack from @astryxdesign/core, TourContext
 * @output Exports TourStep component and TourStepProps
 * @position Lab experiment (facebook/astryx#4239); a single spotlight step in a Tour
 *
 * A TourStep highlights a target element and renders a callout anchored to it,
 * with a heading, body, optional step progress ("2 of 5"), and back / next /
 * close controls. It registers with the parent `<Tour>` on mount (so step
 * order follows the children) and only renders its callout while it is the
 * active step. Anchoring + the callout surface reuse the core `Popover`
 * (`anchorRef` → the step's target), so positioning, top-layer rendering, and
 * dismiss semantics come from the existing layer system rather than a bespoke
 * implementation.
 *
 * SYNC: When modified, update these files to stay in sync:
 * - /packages/lab/src/Tour/Tour.tsx
 * - /packages/lab/src/Tour/TourContext.ts
 * - /packages/lab/src/Tour/Tour.doc.mjs (props table, features)
 * - /packages/lab/src/Tour/Tour.test.tsx (tests for new/changed behavior)
 * - /packages/lab/src/Tour/index.ts (exports if types change)
 */

import {useContext, useEffect, useId, type ReactNode} from 'react';
import * as stylex from '@stylexjs/stylex';
import {Popover} from '@astryxdesign/core/Popover';
import type {LayerPlacement} from '@astryxdesign/core/Layer';
import {Button} from '@astryxdesign/core/Button';
import {Text} from '@astryxdesign/core/Text';
import {Heading} from '@astryxdesign/core/Heading';
import {VStack, HStack} from '@astryxdesign/core/Layout';
import {spacingVars} from '@astryxdesign/core/theme/tokens.stylex';
import {TourContext} from './TourContext';

const styles = stylex.create({
  content: {
    display: 'flex',
    flexDirection: 'column',
    gap: spacingVars['--spacing-3'],
    // Keep the callout from growing arbitrarily wide with long body copy.
    maxWidth: '320px',
  },
  footer: {
    alignItems: 'center',
    justifyContent: 'space-between',
  },
});

export interface TourStepProps {
  /**
   * Ref to the element this step points at. The callout anchors to it (like a
   * Popover trigger); it must be a `<button>` or `[role="button"]` element,
   * matching Popover's `anchorRef` contract. Accepts a ref to any HTMLElement
   * subtype (e.g. `useRef<HTMLButtonElement>(null)`).
   */
  targetRef: React.RefObject<HTMLElement | null>;
  /** Step heading. */
  heading: ReactNode;
  /** Step body content. */
  children?: ReactNode;
  /**
   * Where the callout sits relative to the target.
   * @default 'below'
   */
  placement?: LayerPlacement;
  /** Test id applied to the callout content. */
  'data-testid'?: string;
}

/**
 * A single spotlight step within a `<Tour>`. Renders its callout only while
 * active.
 *
 * @example
 * ```
 * <TourStep targetRef={saveRef} heading="Save your work">
 *   Changes save automatically.
 * </TourStep>
 * ```
 */
export function TourStep({
  targetRef,
  heading,
  children,
  placement = 'below',
  'data-testid': testId,
}: TourStepProps) {
  const tour = useContext(TourContext);
  const id = useId();

  // Register with the controller on mount so the tour learns this step (and
  // its position among siblings). Unregister on unmount.
  useEffect(() => {
    if (tour == null) {
      return;
    }
    return tour.registerStep(id);
  }, [tour, id]);

  // Outside a <Tour>, or when this isn't the active step, render nothing.
  if (tour == null || tour.activeStepId !== id) {
    return null;
  }

  const {
    activeStepIndex,
    stepCount,
    isStepCountShown,
    onNext,
    onPrevious,
    onDismiss,
  } = tour;

  const isFirstStep = activeStepIndex <= 0;
  const isLastStep = stepCount > 0 && activeStepIndex === stepCount - 1;

  const content = (
    <div {...stylex.props(styles.content)} data-testid={testId}>
      <VStack gap={1}>
        <Heading level={4}>{heading}</Heading>
        {children != null && <Text type="body">{children}</Text>}
      </VStack>
      <HStack gap={2} xstyle={styles.footer}>
        {isStepCountShown && stepCount > 0 ? (
          <Text type="supporting" color="secondary">
            {`${activeStepIndex + 1} of ${stepCount}`}
          </Text>
        ) : (
          <span />
        )}
        <HStack gap={2}>
          {!isFirstStep && (
            <Button
              variant="ghost"
              size="sm"
              label="Back"
              onClick={onPrevious}
            />
          )}
          <Button
            variant="primary"
            size="sm"
            label={isLastStep ? 'Done' : 'Next'}
            onClick={onNext}
          />
        </HStack>
      </HStack>
    </div>
  );

  return (
    <Popover
      // Popover types anchorRef as RefObject<HTMLElement>; TourStep accepts a
      // nullable ref for ergonomics (useRef<HTMLButtonElement>(null)). Popover
      // guards a null `.current` internally, so this widening is safe.
      anchorRef={targetRef as React.RefObject<HTMLElement>}
      isOpen
      onOpenChange={open => {
        // Popover reports close from light-dismiss (backdrop) or Escape. Route
        // it to the tour as a dismissal so the whole tour ends, not just this
        // step's popover. Escape and outside-click both surface here.
        if (!open) {
          onDismiss('close');
        }
      }}
      placement={placement}
      label={typeof heading === 'string' ? heading : 'Tour step'}
      hasCloseButton
      closeButtonLabel="Close tour"
      content={content}
    />
  );
}

TourStep.displayName = 'TourStep';
