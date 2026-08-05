// Copyright (c) Meta Platforms, Inc. and affiliates.

import {useState} from 'react';
import type {Meta, StoryObj} from '@storybook/react';
import {RadioControl} from '@astryxdesign/core/RadioList';

const meta: Meta<typeof RadioControl> = {
  title: 'Core/RadioControl',
  component: RadioControl,
  tags: ['autodocs'],
  argTypes: {
    label: {
      control: 'text',
      description: 'Accessible name (aria-label) for a standalone control',
    },
    value: {
      control: 'text',
      description: 'Value reported when this radio is selected',
    },
    name: {
      control: 'text',
      description: 'HTML name shared by the radio group',
    },
    checked: {
      control: 'boolean',
      description: 'Whether the radio is selected',
    },
    size: {
      control: 'select',
      options: ['sm', 'md'],
      description: 'Size of the radio control',
    },
    isDisabled: {
      control: 'boolean',
      description: 'Whether the radio is disabled',
    },
    isRequired: {
      control: 'boolean',
      description: 'Whether the radio is required',
    },
  },
};

export default meta;
type Story = StoryObj<typeof RadioControl>;

export const Default: Story = {
  // A single radio can't normally be un-selected by clicking it (native radio
  // behavior — you deselect by choosing another in the group; see
  // ControlledGroup). To make this standalone demo interactive, we toggle on
  // pointer/keyboard activation via onClick so you can flip it on and off.
  render: args => {
    const [checked, setChecked] = useState(args.checked ?? false);
    return (
      <RadioControl
        {...args}
        checked={checked}
        onChange={() => setChecked(true)}
        onClick={() => setChecked(c => !c)}
      />
    );
  },
  args: {
    label: 'Email',
    name: 'notify',
    value: 'email',
    checked: true,
  },
};

export const Sizes: Story = {
  render: () => (
    <div style={{display: 'flex', alignItems: 'center', gap: 24}}>
      <RadioControl
        label="Small"
        name="sizes"
        value="sm"
        size="sm"
        checked
        onChange={() => {}}
      />
      <RadioControl
        label="Medium"
        name="sizes"
        value="md"
        size="md"
        checked
        onChange={() => {}}
      />
    </div>
  ),
};

export const Disabled: Story = {
  render: () => (
    <div style={{display: 'flex', alignItems: 'center', gap: 24}}>
      <RadioControl
        label="Disabled unchecked"
        name="disabled"
        value="a"
        checked={false}
        isDisabled
        onChange={() => {}}
      />
      <RadioControl
        label="Disabled checked"
        name="disabled"
        value="b"
        checked
        isDisabled
        onChange={() => {}}
      />
    </div>
  ),
};

export const ControlledGroup: Story = {
  render: () => {
    const [value, setValue] = useState('email');
    const options = [
      {label: 'Email', value: 'email'},
      {label: 'SMS', value: 'sms'},
      {label: 'Push', value: 'push'},
    ];
    return (
      <div style={{display: 'flex', flexDirection: 'column', gap: 12}}>
        {options.map(opt => (
          <label
            key={opt.value}
            style={{display: 'flex', alignItems: 'center', gap: 8}}>
            <RadioControl
              name="channel"
              value={opt.value}
              checked={value === opt.value}
              onChange={setValue}
            />
            {opt.label}
          </label>
        ))}
      </div>
    );
  },
};
