// Copyright (c) Meta Platforms, Inc. and affiliates.

/** @type {import('@astryxdesign/cli/authoring').ComponentDoc} */

export const docs = {
  name: 'RadioControl',
  subComponentOf: 'RadioList',
  displayName: 'Radio Control',
  isHiddenFromOverview: true,
  description:
    'Standalone radio input and circle. Composed by RadioListItem; usable on its own with props (no RadioList context).',
  playground: {
    defaults: {
      label: 'Option',
      name: 'radio-control',
      value: 'option-1',
      checked: true,
    },
  },
  props: [
    {
      name: 'checked',
      type: 'boolean',
      description: 'Whether the radio is selected.',
      required: true,
    },
    {
      name: 'onChange',
      type: '(value: string) => void',
      description:
        'Callback fired with `value` when the user selects this radio. No-op while disabled.',
      required: true,
    },
    {
      name: 'value',
      type: 'string',
      description: 'The value reported when this radio is selected.',
      required: true,
    },
    {
      name: 'name',
      type: 'string',
      description:
        'The HTML name shared by the radio group so the browser single-selects within it.',
      required: true,
    },
    {
      name: 'label',
      type: 'string',
      description:
        'Accessible name for a standalone control (applied as aria-label). Omit when an external <label htmlFor> names the input (as RadioList does) to avoid double-naming.',
    },
    {
      name: 'size',
      type: "'sm' | 'md'",
      description: 'Size of the radio control.',
      default: "'md'",
    },
    {
      name: 'isDisabled',
      type: 'boolean',
      description: 'Whether the radio is disabled.',
      default: 'false',
    },
    {
      name: 'isRequired',
      type: 'boolean',
      description: 'Whether the radio is required.',
      default: 'false',
    },
    {
      name: 'keepFocusableWhenDisabled',
      type: 'boolean',
      description:
        'When disabled, keep the input focusable via aria-disabled (and detached from the form) so a group disabled-reason tooltip stays keyboard-discoverable. Selection stays blocked.',
      default: 'false',
    },
    {
      name: 'id',
      type: 'string',
      description:
        'Id applied to the input so an external <label htmlFor> can target it. When omitted, a unique id is generated.',
    },
  ],
};

export const docsZh = {
  name: 'RadioControl',
  isHiddenFromOverview: true,
  displayName: 'Radio Control',
  description:
    '独立的单选输入和圆圈。由 RadioListItem 组合；也可通过 props 独立使用（无需 RadioList 上下文）。',
  props: [
    {
      name: 'checked',
      type: 'boolean',
      description: '单选按钮是否被选中。',
      required: true,
    },
    {
      name: 'onChange',
      type: '(value: string) => void',
      description: '用户选择此单选按钮时触发，回调参数为 value。禁用时不触发。',
      required: true,
    },
    {
      name: 'value',
      type: 'string',
      description: '选中此单选按钮时上报的值。',
      required: true,
    },
    {
      name: 'name',
      type: 'string',
      description: '单选组共享的 HTML name，使浏览器在组内单选。',
      required: true,
    },
    {
      name: 'label',
      type: 'string',
      description:
        '独立控件的无障碍名称（作为 aria-label）。当外部 <label htmlFor> 已命名输入（如 RadioList）时省略，避免重复命名。',
    },
    {
      name: 'size',
      type: "'sm' | 'md'",
      description: '单选控件的尺寸。',
      default: "'md'",
    },
    {
      name: 'isDisabled',
      type: 'boolean',
      description: '是否禁用单选按钮。',
      default: 'false',
    },
    {
      name: 'isRequired',
      type: 'boolean',
      description: '是否为必填。',
      default: 'false',
    },
    {
      name: 'keepFocusableWhenDisabled',
      type: 'boolean',
      description:
        '禁用时通过 aria-disabled 保持可聚焦（并脱离表单），使组禁用原因提示可通过键盘发现。选择仍被阻止。',
      default: 'false',
    },
    {
      name: 'id',
      type: 'string',
      description:
        '应用于输入的 id，使外部 <label htmlFor> 可定位。省略时自动生成唯一 id。',
    },
  ],
};

export const docsDense = {
  name: 'RadioControl',
  isHiddenFromOverview: true,
  displayName: 'Radio Control',
  description:
    'Standalone radio input + circle. Composed by RadioListItem; usable alone via props.',
  propDescriptions: {
    checked: 'Whether the radio is selected.',
    onChange: 'Fired w/ value on select. No-op while disabled.',
    value: 'Value reported when selected.',
    name: 'HTML name shared by the radio group.',
    label: 'Accessible name (aria-label); omit when external <label htmlFor> names it.',
    size: 'Size of the radio control.',
    isDisabled: 'Whether the radio is disabled.',
    isRequired: 'Whether the radio is required.',
    keepFocusableWhenDisabled:
      'Keep focusable via aria-disabled when disabled (for group tooltip).',
    id: 'Id for the input so external <label htmlFor> can target it.',
  },
};
