// Copyright (c) Meta Platforms, Inc. and affiliates.

'use client';

/**
 * @file Tour.tsx
 * @input Uses React state/refs, TourContext, OverlayScrim tokens
 * @output Exports Tour controller component, TourProps, TourHandle
 * @position Lab experiment (facebook/astryx#4239); controller consumed by index.ts
 *
 * Tour is the controller for a product-tour / NUX walkthrough. It renders no
 * visible chrome of its own — it owns the tour state (active step, advance /
 * retreat / complete / dismiss) and shares it with declaratively-nested
 * `<TourStep>` children through context. Steps register on mount, so the step
 * ORDER is taken from the children in document order (no step array to keep in
 * sync with the markup).
 *
 * The behavior (a controller plus spotlight feature steps, an `isActive`
 * switch, dismiss-with-reason, and step progress) is derived from prior art in
 * an internal design system; the composition here is Astryx-native — steps
 * anchor via Popover and dim via an OverlayScrim-style backdrop.
 *
 * SYNC: When modified, update these files to stay in sync:
 * - /packages/lab/src/Tour/TourStep.tsx
 * - /packages/lab/src/Tour/TourContext.ts
 * - /packages/lab/src/Tour/useTour.ts
 * - /packages/lab/src/Tour/Tour.doc.mjs (props table, features)
 * - /packages/lab/src/Tour/Tour.test.tsx (tests for new/changed behavior)
 * - /packages/lab/src/Tour/index.ts (exports if types change)
 */

import {
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
  useState,
  type ReactNode,
} from 'react';
import * as stylex from '@stylexjs/stylex';
import {
  colorVars,
  durationVars,
  easeVars,
} from '@astryxdesign/core/theme/tokens.stylex';
import {
  TourContext,
  type TourDismissSource,
  type TourContextValue,
} from './TourContext';

const styles = stylex.create({
  // Dimmed background behind the active step. Fixed to the viewport and below
  // the step callout (Popover renders in the top layer above this). Non-
  // interactive except to catch a backdrop click; the active step's target
  // stays visible because the callout points at it — a true cutout/spotlight
  // is a follow-up (see doc "Deferred").
  backdrop: {
    position: 'fixed',
    inset: 0,
    backgroundColor: colorVars['--color-overlay'],
    // Below the top-layer Popover; above page content.
    zIndex: 1,
    transitionProperty: 'opacity',
    transitionDuration: durationVars['--duration-fast'],
    transitionTimingFunction: easeVars['--ease-standard'],
  },
});

/**
 * Imperative control surface for a Tour, mirroring the next/previous the steps
 * expose — for driving the tour from outside the step UI (e.g. a debug panel).
 */
export interface TourHandle {
  next: () => void;
  previous: () => void;
}

export interface TourProps {
  /**
   * Whether the tour is running. When false, nothing renders and step state
   * resets — the tour restarts from the first step next time it becomes active.
   * (Controlled: the consumer owns "has this user seen the tour?".)
   */
  isActive: boolean;
  /**
   * The tour's steps — `<TourStep>` elements. Step order is taken from their
   * order here; only the active step renders its callout.
   */
  children?: ReactNode;
  /**
   * Called when the tour is dismissed, with the reason. Fires for every exit,
   * including completing the last step (`'complete'`). The consumer flips
   * `isActive` to false in response.
   */
  onDismiss: (source: TourDismissSource) => void;
  /**
   * Called when the tour completes (advancing past the final step), after
   * `onDismiss('complete')`.
   */
  onComplete?: () => void;
  /**
   * Show a dimmed background behind the active step.
   * @default false
   */
  hasBackdrop?: boolean;
  /**
   * Show the step count ("2 of 5") in each step.
   * @default false
   */
  isStepCountShown?: boolean;
  /** Imperative handle exposing next/previous. */
  handleRef?: React.Ref<TourHandle>;
  /** Test id applied to the backdrop element when present. */
  'data-testid'?: string;
}

/**
 * Controller for a guided product tour. Renders no chrome; coordinates the
 * active step among its `<TourStep>` children.
 *
 * @example
 * ```
 * const [isActive, setIsActive] = useState(true);
 * const saveRef = useRef(null);
 * const shareRef = useRef(null);
 * <Tour isActive={isActive} hasBackdrop isStepCountShown onDismiss={() => setIsActive(false)}>
 *   <TourStep targetRef={saveRef} heading="Save your work">
 *     Changes save automatically to the cloud.
 *   </TourStep>
 *   <TourStep targetRef={shareRef} heading="Share it">
 *     Invite teammates from here.
 *   </TourStep>
 * </Tour>
 * ```
 */
export function Tour({
  isActive,
  children,
  onDismiss,
  onComplete,
  hasBackdrop = false,
  isStepCountShown = false,
  handleRef,
  'data-testid': testId,
}: TourProps) {
  // Steps register on mount; insertion order (document order) defines the
  // sequence. A plain array keeps registration order deterministic.
  const [stepIds, setStepIds] = useState<string[]>([]);
  const [activeStepIndex, setActiveStepIndex] = useState(0);

  const registerStep = useCallback((id: string) => {
    setStepIds(prev => (prev.includes(id) ? prev : [...prev, id]));
    return () => {
      setStepIds(prev => prev.filter(stepId => stepId !== id));
    };
  }, []);

  // Reset to the first step whenever the tour is (re)started, so a dismissed
  // tour begins from the top next time isActive flips back on.
  useEffect(() => {
    if (!isActive) {
      setActiveStepIndex(0);
    }
  }, [isActive]);

  const stepCount = stepIds.length;

  const onNext = useCallback(() => {
    setActiveStepIndex(prev => {
      if (prev < stepCount - 1) {
        return prev + 1;
      }
      // Past the last step → complete.
      onDismiss('complete');
      onComplete?.();
      return prev;
    });
  }, [stepCount, onDismiss, onComplete]);

  const onPrevious = useCallback(() => {
    setActiveStepIndex(prev => (prev > 0 ? prev - 1 : prev));
  }, []);

  useImperativeHandle(handleRef, () => ({next: onNext, previous: onPrevious}), [
    onNext,
    onPrevious,
  ]);

  const activeStepId = isActive ? (stepIds[activeStepIndex] ?? null) : null;

  const contextValue = useMemo<TourContextValue>(
    () => ({
      registerStep,
      activeStepId,
      activeStepIndex,
      stepCount,
      isStepCountShown,
      hasBackdrop,
      onNext,
      onPrevious,
      onDismiss,
    }),
    [
      registerStep,
      activeStepId,
      activeStepIndex,
      stepCount,
      isStepCountShown,
      hasBackdrop,
      onNext,
      onPrevious,
      onDismiss,
    ],
  );

  return (
    <TourContext.Provider value={contextValue}>
      {isActive && hasBackdrop && activeStepId != null && (
        <div
          data-testid={testId}
          aria-hidden="true"
          onClick={() => onDismiss('backdrop')}
          {...stylex.props(styles.backdrop)}
        />
      )}
      {children}
    </TourContext.Provider>
  );
}

Tour.displayName = 'Tour';
