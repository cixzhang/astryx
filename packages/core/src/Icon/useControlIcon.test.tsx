// Copyright (c) Meta Platforms, Inc. and affiliates.

import {describe, expect, it} from 'vitest';
import {renderHook} from '@testing-library/react';
import type {PropsWithChildren, ReactNode} from 'react';
import {Theme} from '../theme/Theme';
import {defineTheme} from '../theme/defineTheme';
import {useControlIcon} from './useControlIcon';

function createThemeWrapper(theme: ReturnType<typeof defineTheme>) {
  function ThemeWrapper({children}: PropsWithChildren): ReactNode {
    return <Theme theme={theme}>{children}</Theme>;
  }
  return ThemeWrapper;
}

describe('useControlIcon', () => {
  it('returns the default control icon renderer without a theme override', () => {
    const {result} = renderHook(() => useControlIcon('checkbox'));

    expect(result.current({state: 'checked'})).toBeTruthy();
  });

  it('resolves a control icon renderer from the nearest theme', () => {
    const themeRenderer = () => 'theme-checkbox';
    const theme = defineTheme({
      name: 'brand-control-icons',
      controlIcons: {checkbox: themeRenderer},
    });

    const {result} = renderHook(() => useControlIcon('checkbox'), {
      wrapper: createThemeWrapper(theme),
    });

    expect(result.current).toBe(themeRenderer);
    expect(result.current({state: 'unchecked'})).toBe('theme-checkbox');
  });
});
