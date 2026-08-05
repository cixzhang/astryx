// Copyright (c) Meta Platforms, Inc. and affiliates.

'use client';

/**
 * @file RadioControl.tsx
 * @input Uses React useId, mergeProps, themeProps
 * @output Exports RadioControl component, RadioControlProps, RadioControlSize
 * @position Core implementation; consumed by RadioListItem and index.ts
 *
 * The self-contained radio control: the visually-hidden native
 * `<input type="radio">` plus its `astryx-radio` circle and `astryx-radio-dot`
 * inner dot. Takes everything as props so it works standalone, and is composed
 * by RadioListItem (which reads RadioListContext and forwards the values down).
 *
 * SYNC: When modified, update these files to stay in sync:
 * - /packages/core/src/RadioList/RadioControl.doc.mjs
 * - /packages/core/src/RadioList/RadioControl.test.tsx
 * - /packages/core/src/RadioList/index.ts
 * - /apps/storybook/stories/RadioControl.stories.tsx
 * - /packages/cli/assets/templates/blocks/components/RadioList/ (showcase blocks)
 */

import React, {useId} from 'react';
import * as stylex from '@stylexjs/stylex';
import type {BaseProps} from '../BaseProps';
import {
  colorVars,
  durationVars,
  easeVars,
  borderVars,
} from '../theme/tokens.stylex';
import {mergeProps} from '../utils';
import {radioScope} from './radio.markers.stylex';
import {themeProps} from '../utils/themeProps';

/**
 * Size of the radio control, matching RadioListSize.
 */
export type RadioControlSize = 'sm' | 'md';

const styles = stylex.create({
  radioWrapper: {
    position: 'relative',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: 0,
    isolation: 'isolate',
  },
  input: {
    position: 'absolute',
    margin: 0,
    padding: 0,
    opacity: 0,
    cursor: 'pointer',
    zIndex: 1,
  },
  inputDisabled: {
    cursor: 'not-allowed',
  },
  radio: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    borderWidth: borderVars['--border-width'],
    borderStyle: 'solid',
    borderRadius: '50%',
    transitionProperty: 'background-color, border-color',
    transitionDuration: durationVars['--duration-fast'],
    transitionTimingFunction: easeVars['--ease-standard'],
    boxSizing: 'border-box',
  },
  radioUnchecked: {
    borderColor: {
      default: colorVars['--color-border-emphasized'],
      [stylex.when.ancestor(':hover', radioScope)]: {
        '@media (hover: hover)': `color-mix(in srgb, ${colorVars['--color-border-emphasized']}, ${colorVars['--color-tint-hover']} 20%)`,
      },
    },
    backgroundColor: {
      default: colorVars['--color-background-surface'],
      [stylex.when.ancestor(':hover', radioScope)]: {
        '@media (hover: hover)': `color-mix(in srgb, ${colorVars['--color-background-surface']}, ${colorVars['--color-tint-hover']} 5%)`,
      },
    },
  },
  radioChecked: {
    borderColor: {
      default: colorVars['--color-accent'],
      [stylex.when.ancestor(':hover', radioScope)]: {
        '@media (hover: hover)': `color-mix(in srgb, ${colorVars['--color-accent']}, ${colorVars['--color-tint-hover']} 15%)`,
      },
    },
    backgroundColor: {
      default: colorVars['--color-accent'],
      [stylex.when.ancestor(':hover', radioScope)]: {
        '@media (hover: hover)': `color-mix(in srgb, ${colorVars['--color-accent']}, ${colorVars['--color-tint-hover']} 15%)`,
      },
    },
  },
  radioWrapperFocus: {
    outline: {
      default: 'none',
      ':has(:focus-visible)': `2px solid ${colorVars['--color-accent']}`,
    },
    outlineOffset: {
      default: '0',
      ':has(:focus-visible)': '2px',
    },
    borderRadius: '50%',
  },
  radioDisabled: {
    opacity: 0.5,
    borderColor: colorVars['--color-border'],
  },
  radioDisabledUnchecked: {
    backgroundColor: colorVars['--color-background-muted'],
  },
  innerDot: {
    borderRadius: '50%',
    backgroundColor: {
      default: colorVars['--color-on-accent'],
      // Forced colors (Windows High Contrast) strips painted backgrounds,
      // which would make the selected dot invisible — checked and unchecked
      // radios would look identical. CanvasText keeps the dot perceivable on
      // the Canvas circle fill (WCAG 1.4.11).
      '@media (forced-colors: active)': 'CanvasText',
    },
  },
});

const wrapperSizeStyles = stylex.create({
  sm: {
    width: 20,
    height: 20,
  },
  md: {
    width: 24,
    height: 24,
  },
});

const radioSizeStyles = stylex.create({
  sm: {
    width: 20,
    height: 20,
  },
  md: {
    width: 24,
    height: 24,
  },
});

const dotSizeStyles = stylex.create({
  sm: {
    width: 8,
    height: 8,
  },
  md: {
    width: 10,
    height: 10,
  },
});

export interface RadioControlProps extends Omit<
  BaseProps<HTMLInputElement>,
  'onChange'
> {
  ref?: React.Ref<HTMLInputElement>;
  /**
   * Whether the radio is selected. Named `checked` to mirror the native
   * `<input type="radio">` it controls.
   */
  // eslint-disable-next-line @astryx/boolean-prop-naming
  checked: boolean;
  /**
   * Callback fired with `value` when the user selects this radio. No-op while
   * disabled.
   */
  onChange: (value: string) => void;
  /**
   * The value submitted / reported when this radio is selected.
   */
  value: string;
  /**
   * The HTML `name` shared by the radio group so the browser roves and
   * single-selects within it.
   */
  name: string;
  /**
   * Accessible name for a standalone control, applied as `aria-label` on the
   * input. Omit it when an external `<label htmlFor>` names the input (as
   * RadioList does) to avoid double-naming.
   */
  label?: string;
  /**
   * The size of the radio control.
   * @default 'md'
   */
  size?: RadioControlSize;
  /**
   * Whether the radio is disabled.
   * @default false
   */
  isDisabled?: boolean;
  /**
   * Whether the radio is required.
   * @default false
   */
  isRequired?: boolean;
  /**
   * When disabled, keep the input focusable via `aria-disabled` instead of the
   * native `disabled` attribute (and detach it from the form so it is excluded
   * from submission). Lets a group's disabled-reason tooltip stay keyboard- and
   * AT-discoverable. Selection is still blocked by the onChange guard.
   * @default false
   */
  // eslint-disable-next-line @astryx/boolean-prop-naming
  keepFocusableWhenDisabled?: boolean;
  /**
   * Id applied to the input so an external `<label htmlFor>` can target it. When
   * omitted, a unique id is generated.
   */
  id?: string;
}

/**
 * A self-contained radio control: the native `<input type="radio">` and its
 * `astryx-radio` circle. Works standalone and is composed by RadioListItem.
 *
 * Provide `label` for a standalone control (it becomes the input's
 * `aria-label`); omit it when an external `<label htmlFor>` already names the
 * input (as RadioList does) so the control is not double-named.
 *
 * @example
 * ```
 * <RadioControl
 *   label="Email"
 *   name="notify"
 *   value="email"
 *   checked={value === 'email'}
 *   onChange={setValue}
 * />
 * ```
 */
export function RadioControl({
  ref,
  checked,
  onChange,
  value,
  name,
  label,
  size = 'md',
  isDisabled = false,
  isRequired = false,
  keepFocusableWhenDisabled = false,
  id,
  xstyle,
  className,
  style,
  ...rest
}: RadioControlProps) {
  const generatedID = useId();
  const inputID = id ?? generatedID;
  const keepsFocusable = isDisabled && keepFocusableWhenDisabled;

  return (
    <div
      {...stylex.props(
        styles.radioWrapper,
        wrapperSizeStyles[size],
        !isDisabled && styles.radioWrapperFocus,
      )}>
      <input
        ref={ref}
        id={inputID}
        type="radio"
        name={name}
        value={value}
        checked={checked}
        disabled={isDisabled && !keepsFocusable}
        aria-disabled={keepsFocusable ? 'true' : undefined}
        // A focusable-disabled radio is not natively disabled, so detach it
        // from the form instead: it keeps its name (grouping) but is excluded
        // from submission, matching a natively disabled control.
        form={keepsFocusable ? '' : undefined}
        required={isRequired}
        aria-label={label}
        onChange={() => {
          if (isDisabled) {
            return;
          }
          onChange(value);
        }}
        {...mergeProps(
          stylex.props(
            styles.input,
            wrapperSizeStyles[size],
            isDisabled && styles.inputDisabled,
            xstyle,
          ),
          className,
          style,
        )}
        {...rest}
      />
      <div
        aria-hidden="true"
        {...mergeProps(
          themeProps('radio', {
            size,
            checked: checked ? 'checked' : null,
            disabled: isDisabled ? 'disabled' : null,
          }),
          stylex.props(
            styles.radio,
            radioSizeStyles[size],
            checked ? styles.radioChecked : styles.radioUnchecked,
            isDisabled && styles.radioDisabled,
            isDisabled && !checked && styles.radioDisabledUnchecked,
          ),
        )}>
        {checked && (
          <div
            {...mergeProps(
              themeProps('radio-dot', {size}),
              stylex.props(styles.innerDot, dotSizeStyles[size]),
            )}
          />
        )}
      </div>
    </div>
  );
}

RadioControl.displayName = 'RadioControl';
