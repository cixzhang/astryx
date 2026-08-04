// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file controlIconRegistry.ts
 * @input Theme source and control icon name
 * @output Exports getControlIcon/useControlIcon backing helpers
 * @position Server-safe control icon resolver for stateful checkbox/radio visuals
 */

import type {DefinedTheme} from '../theme/defineTheme';
import {getRegisteredTheme} from '../theme/themeRegistry';
import {
  defaultControlIcons,
  type ControlIconName,
  type ControlIconRenderer,
} from './controlIcons';

export type ControlIconRegistrySource =
  DefinedTheme | string | null | undefined;

function getTheme(source: ControlIconRegistrySource): DefinedTheme | null {
  if (source == null) {
    return null;
  }
  return typeof source === 'string' ? getRegisteredTheme(source) : source;
}

export function getControlIcon(
  name: ControlIconName,
  source?: ControlIconRegistrySource,
): ControlIconRenderer {
  return getTheme(source)?.controlIcons?.[name] ?? defaultControlIcons[name];
}
