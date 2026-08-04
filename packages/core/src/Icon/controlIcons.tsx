// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file controlIcons.tsx
 * @input Control icon state props
 * @output Default checkbox/radio control icon renderers and control icon types
 * @position Stateful control-icon layer used for checkbox/radio visuals
 */

import type {ReactNode} from 'react';

export interface ControlIconRenderArgs {
  state: 'unchecked' | 'checked' | 'indeterminate';
  size?: 'sm' | 'md' | (string & {});
  isDisabled?: boolean;
  isHovered?: boolean;
  isPressed?: boolean;
}

export type ControlIconRenderer = (props: ControlIconRenderArgs) => ReactNode;

export interface ControlIconMap {
  checkbox: true;
  radio: true;
}

export type ControlIconName = keyof ControlIconMap & string;
export type ControlIconRegistry = Partial<
  Record<ControlIconName, ControlIconRenderer>
>;

const controlSize = (size: ControlIconRenderArgs['size']): number =>
  size === 'sm' ? 20 : 24;

const boxStyle = {
  boxSizing: 'border-box',
  display: 'inline-flex',
  alignItems: 'center',
  justifyContent: 'center',
  flexShrink: 0,
} as const;

function checkboxColor(
  state: ControlIconRenderArgs['state'],
  isDisabled?: boolean,
) {
  if (isDisabled && state === 'unchecked') {
    return {
      borderColor: 'var(--color-border)',
      backgroundColor: 'var(--color-background-muted)',
      color: 'var(--color-on-accent)',
      opacity: 0.5,
    };
  }
  if (isDisabled) {
    return {
      borderColor: 'var(--color-border)',
      backgroundColor: 'var(--color-accent)',
      color: 'var(--color-on-accent)',
      opacity: 0.5,
    };
  }
  if (state === 'unchecked') {
    return {
      borderColor: 'var(--color-border-emphasized)',
      backgroundColor: 'var(--color-background-surface)',
      color: 'var(--color-accent)',
      opacity: 1,
    };
  }
  return {
    borderColor: 'var(--color-accent)',
    backgroundColor: 'var(--color-accent)',
    color: 'var(--color-on-accent)',
    opacity: 1,
  };
}

export function defaultCheckboxControlIcon({
  state,
  size = 'md',
  isDisabled,
}: ControlIconRenderArgs): ReactNode {
  const px = controlSize(size);
  const colors = checkboxColor(state, isDisabled);

  return (
    <span
      aria-hidden="true"
      style={{
        ...boxStyle,
        width: px,
        height: px,
        border: `var(--border-width) solid ${colors.borderColor}`,
        borderRadius: 'var(--radius-inner)',
        backgroundColor: colors.backgroundColor,
        color: colors.color,
        opacity: colors.opacity,
      }}>
      {state === 'checked' && (
        <svg
          aria-hidden="true"
          viewBox="0 0 10 10"
          fill="none"
          stroke="currentColor"
          strokeWidth={1.5}
          strokeLinecap="round"
          strokeLinejoin="round"
          style={{width: px === 20 ? 12 : 14, height: px === 20 ? 12 : 14}}>
          <path d="M8.5 2.5L4 7.5L1.5 5" />
        </svg>
      )}
      {state === 'indeterminate' && (
        <span
          aria-hidden="true"
          style={{
            display: 'block',
            width: px === 20 ? 10 : 12,
            height: 2,
            borderRadius: 999,
            backgroundColor: 'currentColor',
          }}
        />
      )}
    </span>
  );
}

function radioColor(
  state: ControlIconRenderArgs['state'],
  isDisabled?: boolean,
) {
  if (isDisabled && state === 'unchecked') {
    return {
      borderColor: 'var(--color-border)',
      backgroundColor: 'var(--color-background-muted)',
      color: 'var(--color-on-accent)',
      opacity: 0.5,
    };
  }
  if (isDisabled) {
    return {
      borderColor: 'var(--color-border)',
      backgroundColor: 'var(--color-accent)',
      color: 'var(--color-on-accent)',
      opacity: 0.5,
    };
  }
  if (state === 'unchecked') {
    return {
      borderColor: 'var(--color-border-emphasized)',
      backgroundColor: 'var(--color-background-surface)',
      color: 'var(--color-on-accent)',
      opacity: 1,
    };
  }
  return {
    borderColor: 'var(--color-accent)',
    backgroundColor: 'var(--color-accent)',
    color: 'var(--color-on-accent)',
    opacity: 1,
  };
}

export function defaultRadioControlIcon({
  state,
  size = 'md',
  isDisabled,
}: ControlIconRenderArgs): ReactNode {
  const px = controlSize(size);
  const colors = radioColor(state, isDisabled);

  return (
    <span
      aria-hidden="true"
      style={{
        ...boxStyle,
        width: px,
        height: px,
        border: `var(--border-width) solid ${colors.borderColor}`,
        borderRadius: '50%',
        backgroundColor: colors.backgroundColor,
        color: colors.color,
        opacity: colors.opacity,
      }}>
      {state === 'checked' && (
        <span
          aria-hidden="true"
          style={{
            display: 'block',
            width: px === 20 ? 8 : 10,
            height: px === 20 ? 8 : 10,
            borderRadius: '50%',
            backgroundColor: 'currentColor',
          }}
        />
      )}
    </span>
  );
}

export const defaultControlIcons: Required<ControlIconRegistry> = {
  checkbox: defaultCheckboxControlIcon,
  radio: defaultRadioControlIcon,
};
