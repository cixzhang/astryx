// Copyright (c) Meta Platforms, Inc. and affiliates.

'use client';

/**
 * @file useControlIcon.ts
 * @input Control icon name
 * @output Exports useControlIcon hook for theme-aware control icon renderers
 * @position Client hook for stateful checkbox/radio visuals
 */

import {useThemeName} from '../theme/useTheme';
import {getControlIcon} from './controlIconRegistry';
import type {ControlIconName, ControlIconRenderer} from './controlIcons';

export function useControlIcon(name: ControlIconName): ControlIconRenderer {
  return getControlIcon(name, useThemeName());
}
