// Copyright (c) Meta Platforms, Inc. and affiliates.

import type {Meta, StoryObj} from '@storybook/react';
import {useRef, useState} from 'react';
import {Tour, TourStep} from '@astryxdesign/lab';
import {Button} from '@astryxdesign/core/Button';
import {Heading} from '@astryxdesign/core/Heading';
import {HStack, VStack} from '@astryxdesign/core/Stack';
import {Text} from '@astryxdesign/core/Text';

const meta: Meta<typeof Tour> = {
  title: 'Lab/Tour',
  component: Tour,
  tags: ['autodocs'],
  parameters: {
    layout: 'fullscreen',
  },
  decorators: [
    Story => (
      <div style={{minHeight: 480, padding: 32}}>
        <Story />
      </div>
    ),
  ],
};

export default meta;
type Story = StoryObj<typeof Tour>;

export const Showcase: Story = {
  render: () => {
    const [isActive, setIsActive] = useState(false);
    const saveRef = useRef<HTMLButtonElement>(null);
    const shareRef = useRef<HTMLButtonElement>(null);
    const settingsRef = useRef<HTMLButtonElement>(null);

    return (
      <VStack gap={4}>
        <HStack gap={2}>
          <Button ref={saveRef} variant="secondary" label="Save" />
          <Button ref={shareRef} variant="secondary" label="Share" />
          <Button ref={settingsRef} variant="secondary" label="Settings" />
        </HStack>

        <Button label="Start tour" onClick={() => setIsActive(true)} />

        <Tour
          isActive={isActive}
          hasBackdrop
          isStepCountShown
          onDismiss={() => setIsActive(false)}>
          <TourStep targetRef={saveRef} heading="Save your work">
            Changes save automatically to the cloud as you go.
          </TourStep>
          <TourStep targetRef={shareRef} heading="Share with your team">
            Invite teammates and manage access from here.
          </TourStep>
          <TourStep targetRef={settingsRef} heading="Tune your setup">
            Adjust preferences and defaults in Settings.
          </TourStep>
        </Tour>
      </VStack>
    );
  },
};

export const WithoutBackdrop: Story = {
  render: () => {
    const [isActive, setIsActive] = useState(false);
    const targetRef = useRef<HTMLButtonElement>(null);

    return (
      <VStack gap={4}>
        <Heading level={3}>Feature callout</Heading>
        <Text type="body">
          A single-step tour with no dimmed background — a lightweight
          coachmark.
        </Text>
        <Button ref={targetRef} variant="secondary" label="New feature" />
        <Button label="Highlight it" onClick={() => setIsActive(true)} />

        <Tour isActive={isActive} onDismiss={() => setIsActive(false)}>
          <TourStep
            targetRef={targetRef}
            heading="Try the new feature"
            placement="below">
            We just shipped this — click to explore.
          </TourStep>
        </Tour>
      </VStack>
    );
  },
};
