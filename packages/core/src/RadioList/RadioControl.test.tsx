// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file RadioControl.test.tsx
 * @input Uses vitest, @testing-library/react, RadioControl
 * @output Unit tests for the standalone RadioControl behavior
 * @position Testing; validates RadioControl.tsx implementation
 *
 * SYNC: When RadioControl.tsx changes, update tests to match new behavior
 */

import {describe, it, expect, vi} from 'vitest';
import {render, screen, fireEvent} from '@testing-library/react';

import {RadioControl} from './RadioControl';

describe('RadioControl', () => {
  it('renders a standalone radio with an accessible name and does not throw', () => {
    render(
      <RadioControl
        label="Email"
        name="notify"
        value="email"
        checked={false}
        onChange={() => {}}
      />,
    );
    const radio = screen.getByRole('radio', {name: 'Email'});
    expect(radio).toBeInTheDocument();
    expect(radio).toHaveAttribute('aria-label', 'Email');
  });

  it('reflects the checked prop', () => {
    const {rerender} = render(
      <RadioControl
        label="Email"
        name="notify"
        value="email"
        checked={false}
        onChange={() => {}}
      />,
    );
    expect(screen.getByRole('radio', {name: 'Email'})).not.toBeChecked();

    rerender(
      <RadioControl
        label="Email"
        name="notify"
        value="email"
        checked={true}
        onChange={() => {}}
      />,
    );
    expect(screen.getByRole('radio', {name: 'Email'})).toBeChecked();
  });

  it('fires onChange with the value when clicked', () => {
    const onChange = vi.fn();
    render(
      <RadioControl
        label="Email"
        name="notify"
        value="email"
        checked={false}
        onChange={onChange}
      />,
    );
    fireEvent.click(screen.getByRole('radio', {name: 'Email'}));
    expect(onChange).toHaveBeenCalledWith('email');
  });

  it('does not fire onChange when disabled', () => {
    const onChange = vi.fn();
    render(
      <RadioControl
        label="Email"
        name="notify"
        value="email"
        checked={false}
        isDisabled
        onChange={onChange}
      />,
    );
    const radio = screen.getByRole('radio', {name: 'Email', hidden: true});
    expect(radio).toBeDisabled();
    // A native disabled input drops the event; fire on the DOM node directly to
    // prove the onChange guard also blocks selection.
    fireEvent.click(radio);
    expect(onChange).not.toHaveBeenCalled();
  });

  it('renders the astryx-radio and astryx-radio-dot theme targets', () => {
    const {container} = render(
      <RadioControl
        label="Email"
        name="notify"
        value="email"
        checked={true}
        onChange={() => {}}
      />,
    );
    expect(container.querySelector('.astryx-radio')).toBeInTheDocument();
    expect(container.querySelector('.astryx-radio-dot')).toBeInTheDocument();
  });

  it('does not render the inner dot when unchecked', () => {
    const {container} = render(
      <RadioControl
        label="Email"
        name="notify"
        value="email"
        checked={false}
        onChange={() => {}}
      />,
    );
    expect(container.querySelector('.astryx-radio')).toBeInTheDocument();
    expect(
      container.querySelector('.astryx-radio-dot'),
    ).not.toBeInTheDocument();
  });

  it('uses the provided id so an external label can target it', () => {
    render(
      <>
        <label htmlFor="my-radio">External label</label>
        <RadioControl
          id="my-radio"
          name="notify"
          value="email"
          checked={false}
          onChange={() => {}}
        />
      </>,
    );
    // Named by the external <label htmlFor>, not aria-label (no double-naming).
    const radio = screen.getByRole('radio', {name: 'External label'});
    expect(radio).toHaveAttribute('id', 'my-radio');
    expect(radio).not.toHaveAttribute('aria-label');
  });

  it('keeps a disabled control focusable when keepFocusableWhenDisabled is set', () => {
    const onChange = vi.fn();
    render(
      <RadioControl
        label="Email"
        name="notify"
        value="email"
        checked={false}
        isDisabled
        keepFocusableWhenDisabled
        onChange={onChange}
      />,
    );
    const radio = screen.getByRole('radio', {name: 'Email'});
    expect(radio).not.toBeDisabled();
    expect(radio).toHaveAttribute('aria-disabled', 'true');
    expect(radio).toHaveAttribute('form', '');
    fireEvent.click(radio);
    expect(onChange).not.toHaveBeenCalled();
  });
});
