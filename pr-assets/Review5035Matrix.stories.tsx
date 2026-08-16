// Copyright (c) Meta Platforms, Inc. and affiliates.
// Scratch review harness for PR #5035 (SegmentedControl label clamp).
// Not for landing.

import {useState} from 'react';
import type {Meta, StoryObj} from '@storybook/react';
import {
  SegmentedControl,
  SegmentedControlItem,
} from '@astryxdesign/core/SegmentedControl';
import {Icon} from '@astryxdesign/core/Icon';
import {
  Squares2X2Icon,
  ListBulletIcon,
  TableCellsIcon,
} from '@heroicons/react/24/outline';

const LONG = 'Grid view of all items in this collection';

const meta: Meta = {
  title: 'Review/5035',
  parameters: {layout: 'fullscreen'},
};
export default meta;
type S = StoryObj;

function Control({
  layout,
  labels,
  icons = false,
  hidden = false,
}: {
  layout: 'hug' | 'fill';
  labels: string[];
  icons?: boolean;
  hidden?: boolean;
}) {
  const [value, setValue] = useState('i0');
  const iconList = [Squares2X2Icon, ListBulletIcon, TableCellsIcon];
  return (
    <SegmentedControl
      value={value}
      onChange={setValue}
      label="View mode"
      layout={layout}>
      {labels.map((l, i) => (
        <SegmentedControlItem
          key={i}
          value={`i${i}`}
          label={l}
          isLabelHidden={hidden}
          icon={
            icons ? (
              <Icon icon={iconList[i % 3]} color="inherit" />
            ) : undefined
          }
        />
      ))}
    </SegmentedControl>
  );
}

function Shot({
  name,
  width,
  children,
}: {
  name: string;
  width?: number | string;
  children: React.ReactNode;
}) {
  return (
    <div
      data-shot={name}
      style={{
        padding: 12,
        background: '#fff',
        width: width ?? 'auto',
        boxSizing: 'content-box',
      }}>
      {children}
    </div>
  );
}

// 1 --------------------------------------------------------------- baseline
export const Baseline: S = {
  render: () => (
    <Shot name="baseline" width={600}>
      <Control layout="hug" labels={['Grid', 'List', 'Table']} />
    </Shot>
  ),
};

// 2 ------------------------------------------------- long label, hug, wide
export const LongHugWide: S = {
  render: () => (
    <Shot name="long-hug-wide" width={600}>
      <Control layout="hug" labels={[LONG, 'List', 'Table']} />
    </Shot>
  ),
};

// 3 ------------------------------------------------- long label, hug, 320
export const LongHug320: S = {
  render: () => (
    <Shot name="long-hug-320" width={320}>
      <Control layout="hug" labels={[LONG, 'List', 'Table']} />
    </Shot>
  ),
};

// 4 ------------------------------------------------ long label, fill, 320
export const LongFill320: S = {
  render: () => (
    <Shot name="long-fill-320" width={320}>
      <Control layout="fill" labels={[LONG, 'List', 'Table']} />
    </Shot>
  ),
};

// 5 ------------------------------------ mixed: one long + two short, fill
export const MixedFill320: S = {
  render: () => (
    <Shot name="mixed-fill-320" width={320}>
      <Control layout="fill" labels={['Grid', LONG, 'Table']} />
    </Shot>
  ),
};

// 6 ------------------------------------------ grandparent width sources
const boxLabel: React.CSSProperties = {
  font: '11px ui-monospace, monospace',
  color: '#555',
  marginBottom: 4,
};

export const WidthSources: S = {
  render: () => (
    <Shot name="width-sources" width={520}>
      <div style={{display: 'grid', gap: 16}}>
        <div>
          <div style={boxLabel}>(a) fixed-width div: 320px</div>
          <div style={{width: 320, outline: '1px dashed #c00'}}>
            <Control layout="fill" labels={[LONG, 'List', 'Table']} />
          </div>
        </div>
        <div>
          <div style={boxLabel}>(b) width:auto block (sizes to content)</div>
          <div style={{outline: '1px dashed #c00'}}>
            <Control layout="fill" labels={[LONG, 'List', 'Table']} />
          </div>
        </div>
        <div>
          <div style={boxLabel}>
            (c) flex parent, child flex:1 (row is 320px)
          </div>
          <div style={{display: 'flex', width: 320, outline: '1px dashed #c00'}}>
            <div style={{flex: 1}}>
              <Control layout="fill" labels={[LONG, 'List', 'Table']} />
            </div>
          </div>
        </div>
        <div>
          <div style={boxLabel}>
            (d) grid track: grid-template-columns 200px 1fr (row is 320px)
          </div>
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: '200px 1fr',
              width: 320,
              outline: '1px dashed #c00',
            }}>
            <div style={{outline: '1px dotted #06c'}}>
              <Control layout="fill" labels={[LONG, 'List', 'Table']} />
            </div>
            <div style={{font: '11px monospace', color: '#888'}}>1fr</div>
          </div>
        </div>
      </div>
    </Shot>
  ),
};

// 7 ------------------------------------------- ancestor overflow: hidden
export const AncestorOverflow: S = {
  render: () => (
    <Shot name="ancestor-overflow" width={420}>
      <div style={{display: 'grid', gap: 16}}>
        <div>
          <div style={boxLabel}>ancestor overflow: visible (default)</div>
          <div style={{width: 320, outline: '1px dashed #c00'}}>
            <Control layout="fill" labels={[LONG, 'List', 'Table']} />
          </div>
        </div>
        <div>
          <div style={boxLabel}>ancestor overflow: hidden</div>
          <div
            style={{width: 320, overflow: 'hidden', outline: '1px dashed #c00'}}>
            <Control layout="fill" labels={[LONG, 'List', 'Table']} />
          </div>
        </div>
      </div>
    </Shot>
  ),
};

// 8 --------------------------------------------------------------- RTL
export const Rtl: S = {
  render: () => (
    <Shot name="rtl-fill-320" width={320}>
      <div dir="rtl">
        <Control layout="fill" labels={['عرض شبكي لكل العناصر في هذه المجموعة', 'قائمة', 'جدول']} />
      </div>
    </Shot>
  ),
};

// 9 --------------------------------------------------------- text zoom
export const Zoom200: S = {
  render: () => (
    <Shot name="zoom-200-fill-320" width={320}>
      <Control layout="fill" labels={[LONG, 'List', 'Table']} />
    </Shot>
  ),
};

// 10 -------------------------------------------------- forced colors case
export const ForcedColors: S = {
  render: () => (
    <Shot name="forced-colors-fill-320" width={320}>
      <Control layout="fill" labels={[LONG, 'List', 'Table']} />
    </Shot>
  ),
};

// 11 ------------------------------------------------ icon + label variants
export const IconPaths: S = {
  render: () => (
    <Shot name="icon-paths" width={420}>
      <div style={{display: 'grid', gap: 16}}>
        <div>
          <div style={boxLabel}>icon + long label, fill, 320px</div>
          <div style={{width: 320}}>
            <Control layout="fill" icons labels={[LONG, 'List', 'Table']} />
          </div>
        </div>
        <div>
          <div style={boxLabel}>isLabelHidden (icon only), fill, 320px</div>
          <div style={{width: 320}}>
            <Control
              layout="fill"
              icons
              hidden
              labels={[LONG, 'List', 'Table']}
            />
          </div>
        </div>
        <div>
          <div style={boxLabel}>icon + short labels, hug</div>
          <Control layout="hug" icons labels={['Grid', 'List', 'Table']} />
        </div>
      </div>
    </Shot>
  ),
};
