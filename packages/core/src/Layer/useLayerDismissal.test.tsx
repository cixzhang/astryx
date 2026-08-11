// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file useLayerDismissal.test.tsx
 * @input Uses @testing-library/react, vitest
 * @output Tests for the shared layer dismissal stack
 * @position Colocated tests for useLayerDismissal + layerStack
 *
 * Every case here is "one Escape press, which layers reacted?" — the single
 * question the stack exists to answer.
 */

import {render, fireEvent} from '@testing-library/react';
import {describe, expect, it, vi, afterEach} from 'vitest';
import {useRef} from 'react';

import {LayerDepthProvider} from './LayerDepthContext';
import {useLayerDismissal} from './useLayerDismissal';
import {resetLayerStackForTests} from './layerStack';
import type {LayerEscapeBehavior} from './layerStack';

afterEach(() => {
  resetLayerStackForTests();
});

/** A layer that registers itself and renders its children one level deeper. */
function Layer({
  onDismiss,
  behavior = 'close',
  isEnabled = true,
  isActive = true,
  isPresent,
  children,
}: {
  onDismiss: () => void;
  behavior?: LayerEscapeBehavior;
  isEnabled?: boolean;
  isActive?: boolean;
  isPresent?: () => boolean;
  children?: React.ReactNode;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  useLayerDismissal({
    isActive,
    onDismiss,
    escapeBehavior: behavior,
    isEnabled,
    isPresent,
    getContainer: () => containerRef.current,
  });
  return (
    <div ref={containerRef}>
      <LayerDepthProvider>{children}</LayerDepthProvider>
    </div>
  );
}

/** A layer that reports no depth — like a bare focus trap, which renders nothing. */
function FlatLayer({
  onDismiss,
  children,
}: {
  onDismiss: () => void;
  children?: React.ReactNode;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  useLayerDismissal({
    isActive: true,
    onDismiss,
    getContainer: () => containerRef.current,
  });
  return <div ref={containerRef}>{children}</div>;
}

const pressEscape = () => fireEvent.keyDown(document, {key: 'Escape'});

describe('useLayerDismissal', () => {
  it('dismisses only the top-most layer when both open in the same commit', () => {
    // The regression: an inner and outer layer that mount together. React runs
    // child effects first, so registration order reports the inner layer as the
    // OLDER one — nesting, not order, has to decide.
    const outer = vi.fn();
    const inner = vi.fn();

    render(
      <Layer onDismiss={outer}>
        <Layer onDismiss={inner} />
      </Layer>,
    );

    pressEscape();
    expect(inner).toHaveBeenCalledTimes(1);
    expect(outer).not.toHaveBeenCalled();
  });

  it('falls through to the outer layer once the inner one closes', () => {
    const outer = vi.fn();
    const inner = vi.fn();
    const {rerender} = render(
      <Layer onDismiss={outer}>
        <Layer onDismiss={inner} />
      </Layer>,
    );
    rerender(<Layer onDismiss={outer} />);

    pressEscape();
    expect(outer).toHaveBeenCalledTimes(1);
    expect(inner).not.toHaveBeenCalled();
  });

  it('dismisses the later of two unrelated layers', () => {
    const first = vi.fn();
    const second = vi.fn();
    const {rerender} = render(<Layer onDismiss={first} />);
    rerender(
      <>
        <Layer onDismiss={first} />
        <Layer onDismiss={second} />
      </>,
    );

    pressEscape();
    expect(second).toHaveBeenCalledTimes(1);
    expect(first).not.toHaveBeenCalled();
  });

  it('resolves nesting by DOM containment when the tree reports equal depth', () => {
    // A bare focus trap renders nothing, so it cannot push a depth provider.
    // Containment is the only nesting signal those layers have.
    const outer = vi.fn();
    const inner = vi.fn();
    render(
      <FlatLayer onDismiss={outer}>
        <FlatLayer onDismiss={inner} />
      </FlatLayer>,
    );

    pressEscape();
    expect(inner).toHaveBeenCalledTimes(1);
    expect(outer).not.toHaveBeenCalled();
  });

  describe('escapeBehavior', () => {
    it("'close' consumes the press, so one Escape affects exactly one layer", () => {
      // A hover tip inside a modal: the tip is on top, so it takes the press
      // and the modal stays open. A second Escape closes the modal.
      const modal = vi.fn();
      const tip = vi.fn();
      render(
        <Layer onDismiss={modal}>
          <Layer onDismiss={tip} />
        </Layer>,
      );

      pressEscape();
      expect(tip).toHaveBeenCalledTimes(1);
      expect(modal).not.toHaveBeenCalled();
    });

    it("'block' consumes the press without dismissing anything", () => {
      const host = vi.fn();
      const required = vi.fn();
      render(
        <Layer onDismiss={host}>
          <Layer onDismiss={required} behavior="block" />
        </Layer>,
      );

      pressEscape();
      expect(required).not.toHaveBeenCalled();
      expect(host).not.toHaveBeenCalled();
    });
  });

  describe('presence', () => {
    it('skips a registered layer that is not on screen', () => {
      // Hover layers stay registered for their lifetime because their open
      // state lags the DOM. An absent one must not claim the press — that is
      // exactly the bug where a HoverCard trigger ate Escapes while idle.
      const below = vi.fn();
      const absent = vi.fn();
      render(
        <Layer onDismiss={below}>
          <Layer onDismiss={absent} isPresent={() => false} />
        </Layer>,
      );

      pressEscape();
      expect(absent).not.toHaveBeenCalled();
      expect(below).toHaveBeenCalledTimes(1);
    });

    it('lets a present layer claim the press over the one beneath', () => {
      const below = vi.fn();
      const present = vi.fn();
      render(
        <Layer onDismiss={below}>
          <Layer onDismiss={present} isPresent={() => true} />
        </Layer>,
      );

      pressEscape();
      expect(present).toHaveBeenCalledTimes(1);
      expect(below).not.toHaveBeenCalled();
    });
  });

  describe('opting out', () => {
    it('skips a disabled layer entirely, so the press reaches the one below', () => {
      const below = vi.fn();
      const optedOut = vi.fn();
      render(
        <Layer onDismiss={below}>
          <Layer onDismiss={optedOut} isEnabled={false} />
        </Layer>,
      );

      pressEscape();
      expect(optedOut).not.toHaveBeenCalled();
      expect(below).toHaveBeenCalledTimes(1);
    });

    it('does not register an inactive layer', () => {
      const onDismiss = vi.fn();
      render(<Layer onDismiss={onDismiss} isActive={false} />);

      pressEscape();
      expect(onDismiss).not.toHaveBeenCalled();
    });
  });

  describe('deferring to content', () => {
    it('stands down when content already handled the press', () => {
      // preventDefault from inside the layer — an editor claiming Escape for
      // its own find widget, for instance.
      const onDismiss = vi.fn();
      render(
        <Layer onDismiss={onDismiss}>
          <button
            type="button"
            data-testid="editor"
            onKeyDown={e => {
              if (e.key === 'Escape') {
                e.preventDefault();
              }
            }}
          />
        </Layer>,
      );

      fireEvent.keyDown(document.querySelector('[data-testid="editor"]')!, {
        key: 'Escape',
      });
      expect(onDismiss).not.toHaveBeenCalled();
    });

    it('ignores Escape that is cancelling an IME composition', () => {
      const onDismiss = vi.fn();
      render(<Layer onDismiss={onDismiss} />);

      fireEvent.keyDown(document, {key: 'Escape', isComposing: true});
      expect(onDismiss).not.toHaveBeenCalled();

      fireEvent.keyDown(document, {key: 'Escape', keyCode: 229});
      expect(onDismiss).not.toHaveBeenCalled();

      pressEscape();
      expect(onDismiss).toHaveBeenCalledTimes(1);
    });
  });

  it('leaves the event alone when no layer is open', () => {
    // Nothing registered: the browser keeps its own Escape behavior (exiting
    // fullscreen, closing a native picker).
    render(<div />);
    const event = new KeyboardEvent('keydown', {
      key: 'Escape',
      bubbles: true,
      cancelable: true,
    });
    document.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(false);
  });

  it('claims the press it handles so the browser does not act too', () => {
    // preventDefault is what stops the native close-watcher dismissing a second
    // layer behind the stack's back.
    render(<Layer onDismiss={vi.fn()} />);
    const event = new KeyboardEvent('keydown', {
      key: 'Escape',
      bubbles: true,
      cancelable: true,
    });
    document.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
  });
});
