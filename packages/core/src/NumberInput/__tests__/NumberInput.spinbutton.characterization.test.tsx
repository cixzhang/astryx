// Copyright (c) Meta Platforms, Inc. and affiliates.
/** @vitest-environment jsdom */

/**
 * @file NumberInput.spinbutton.characterization.test.tsx
 * @input Uses the current NumberInput public surface
 * @output Characterization lock for its shipped spinbutton semantics and keyboard stepping
 * @position Temporary LOCK step before extracting the shared Spinbutton contract.
 */

import {cleanup, fireEvent, render, screen} from '@testing-library/react';
import {afterEach, describe, expect, it, vi} from 'vitest';
import {NumberInput} from '../NumberInput';

afterEach(cleanup);

describe('NumberInput shipped spinbutton contract', () => {
  it('exposes its role, persistent label, current value, and bounds', () => {
    render(
      <NumberInput
        label="Quantity"
        value={5}
        min={1}
        max={9}
        onChange={() => {}}
      />,
    );

    const input = screen.getByRole('spinbutton', {name: 'Quantity'});
    expect(input).toHaveAttribute('aria-valuenow', '5');
    expect(input).toHaveAttribute('aria-valuemin', '1');
    expect(input).toHaveAttribute('aria-valuemax', '9');
  });

  it('exposes formatted committed values without replacing the numeric value', () => {
    render(
      <NumberInput
        label="Storage"
        value={5}
        onChange={() => {}}
        formatValue={value => `${value} GB`}
      />,
    );

    const input = screen.getByRole('spinbutton', {name: 'Storage'});
    expect(input).toHaveAttribute('aria-valuenow', '5');
    expect(input).toHaveAttribute('aria-valuetext', '5 GB');
  });

  it('distinguishes disabled and read-only states', () => {
    const {rerender} = render(
      <NumberInput label="Quantity" value={5} onChange={() => {}} isDisabled />,
    );
    expect(screen.getByRole('spinbutton', {name: 'Quantity'})).toBeDisabled();

    rerender(
      <NumberInput label="Quantity" value={5} onChange={() => {}} isReadOnly />,
    );
    const readOnly = screen.getByRole('spinbutton', {name: 'Quantity'});
    expect(readOnly).not.toBeDisabled();
    expect(readOnly).toHaveAttribute('readonly');
  });

  it('steps up and down from the committed value with unmodified arrow keys', () => {
    const onChange = vi.fn();
    render(<NumberInput label="Quantity" value={5} onChange={onChange} />);
    const input = screen.getByRole('spinbutton', {name: 'Quantity'});

    fireEvent.keyDown(input, {key: 'ArrowUp'});
    expect(onChange).toHaveBeenLastCalledWith(6);
    fireEvent.keyDown(input, {key: 'ArrowDown'});
    expect(onChange).toHaveBeenLastCalledWith(4);
  });

  it('blocks arrow-key stepping while read-only', () => {
    const onChange = vi.fn();
    render(
      <NumberInput label="Quantity" value={5} onChange={onChange} isReadOnly />,
    );

    const input = screen.getByRole('spinbutton', {name: 'Quantity'});
    fireEvent.keyDown(input, {key: 'ArrowUp'});
    fireEvent.keyDown(input, {key: 'ArrowDown'});
    expect(onChange).not.toHaveBeenCalled();
  });
});
