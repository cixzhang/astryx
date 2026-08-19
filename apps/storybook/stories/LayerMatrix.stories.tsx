// Copyright (c) Meta Platforms, Inc. and affiliates.

/**
 * @file LayerMatrix.stories.tsx
 * @input Uses every overlay family in core + lab
 * @output One story per (outer, inner) layer pairing, driven by a browser harness
 * @position Spike harness for the shared dismissal stack. Each story exists to
 *   be pressed: a driver opens the outer layer, opens the inner one, and counts
 *   what one Escape removed.
 *
 * Every trigger carries a `data-testid` so the driver never guesses at text.
 */

import {useMemo, useState} from 'react';
import type {Meta, StoryObj} from '@storybook/react';
import {AlertDialog} from '@astryxdesign/core/AlertDialog';
import {BottomSheet, BottomSheetSwitcher} from '@astryxdesign/core/BottomSheet';
import {Button} from '@astryxdesign/core/Button';
import {CommandPalette} from '@astryxdesign/core/CommandPalette';
import {ComplexSelector} from '@astryxdesign/core/ComplexSelector';
import {ContextMenu} from '@astryxdesign/core/ContextMenu';
import {Dialog, DialogHeader} from '@astryxdesign/core/Dialog';
import {DropdownMenu} from '@astryxdesign/core/DropdownMenu';
import {HoverCard} from '@astryxdesign/core/HoverCard';
import {Layout, LayoutContent} from '@astryxdesign/core/Layout';
import {Lightbox} from '@astryxdesign/core/Lightbox';
import {MobileNav} from '@astryxdesign/core/MobileNav';
import {MultiSelector} from '@astryxdesign/core/MultiSelector';
import {Popover} from '@astryxdesign/core/Popover';
import {Section} from '@astryxdesign/core/Section';
import {Selector} from '@astryxdesign/core/Selector';
import {SideNavItem, SideNavSection} from '@astryxdesign/core/SideNav';
import {VStack} from '@astryxdesign/core/Stack';
import {Text} from '@astryxdesign/core/Text';
import {Tooltip} from '@astryxdesign/core/Tooltip';
import {Typeahead, createStaticSource} from '@astryxdesign/core/Typeahead';
import {Drawer, InfoTip} from '@astryxdesign/lab';

const meta: Meta = {
  title: 'Spike/Layer Matrix',
  parameters: {layout: 'fullscreen'},
};
export default meta;

type Story = StoryObj;

const FRUITS = ['Apple', 'Banana', 'Cherry', 'Mango'];

function useFruitSource() {
  return useMemo(
    () =>
      createStaticSource(
        FRUITS.map(label => ({id: label.toLowerCase(), label})),
      ),
    [],
  );
}

/**
 * The outer Dialog every "inner layer" case opens inside. `renderInner` is the
 * layer under test, rendered in the dialog's own subtree — the nesting real
 * code writes, and the one that used to close two layers on one press.
 */
function OuterDialog({
  children,
  label = 'outer',
}: {
  children: React.ReactNode;
  label?: string;
}) {
  const [isOpen, setIsOpen] = useState(false);
  return (
    <div style={{padding: 24}}>
      <Button
        label="Open outer"
        data-testid="open-outer"
        onClick={() => setIsOpen(true)}
      />
      <Dialog
        isOpen={isOpen}
        onOpenChange={setIsOpen}
        width={560}
        aria-label={label}>
        <Layout
          header={<DialogHeader title="Outer" onOpenChange={setIsOpen} />}
          content={<LayoutContent>{children}</LayoutContent>}
        />
      </Dialog>
    </div>
  );
}

// ─── Group A: an inner layer opened inside an open Dialog ────────────────────

export const DialogInDialog: Story = {
  render: function Render() {
    const [isOpen, setIsOpen] = useState(false);
    return (
      <OuterDialog>
        <Button
          label="Open inner"
          data-testid="open-inner"
          onClick={() => setIsOpen(true)}
        />
        <Dialog
          isOpen={isOpen}
          onOpenChange={setIsOpen}
          width={380}
          aria-label="inner">
          <Layout
            header={<DialogHeader title="Inner" onOpenChange={setIsOpen} />}
            content={
              <LayoutContent>
                <Text type="body">Inner dialog</Text>
              </LayoutContent>
            }
          />
        </Dialog>
      </OuterDialog>
    );
  },
};

export const AlertDialogInDialog: Story = {
  render: function Render() {
    const [isOpen, setIsOpen] = useState(false);
    return (
      <OuterDialog>
        <Button
          label="Open inner"
          data-testid="open-inner"
          onClick={() => setIsOpen(true)}
        />
        <AlertDialog
          isOpen={isOpen}
          onOpenChange={setIsOpen}
          title="Delete item?"
          description="This cannot be undone."
          actionLabel="Delete"
          onAction={() => setIsOpen(false)}
        />
      </OuterDialog>
    );
  },
};

export const ContextMenuInDialog: Story = {
  render: () => (
    <OuterDialog>
      <ContextMenu
        items={[
          {label: 'Cut', onClick: () => {}},
          {label: 'Copy', onClick: () => {}},
        ]}>
        <div
          data-testid="open-inner"
          style={{padding: 32, border: '2px dashed #ccc'}}>
          Right-click here
        </div>
      </ContextMenu>
    </OuterDialog>
  ),
};

export const TooltipInDialog: Story = {
  render: () => (
    <OuterDialog>
      <Tooltip content="A hover tip">
        <Button label="Hover me" data-testid="open-inner" />
      </Tooltip>
    </OuterDialog>
  ),
};

export const HoverCardInDialog: Story = {
  render: () => (
    <OuterDialog>
      <HoverCard content={<Text type="body">Card body</Text>} delay={0}>
        <Button label="Hover me" data-testid="open-inner" />
      </HoverCard>
    </OuterDialog>
  ),
};

export const DrawerInDialog: Story = {
  render: function Render() {
    const [isOpen, setIsOpen] = useState(false);
    return (
      <OuterDialog>
        <Button
          label="Open inner"
          data-testid="open-inner"
          onClick={() => setIsOpen(true)}
        />
        <Drawer
          isOpen={isOpen}
          onClose={() => setIsOpen(false)}
          label="inner drawer"
          size={320}>
          <Section padding={4}>
            <Text type="body">Drawer body</Text>
          </Section>
        </Drawer>
      </OuterDialog>
    );
  },
};

export const BottomSheetInDialog: Story = {
  render: function Render() {
    const [isOpen, setIsOpen] = useState(false);
    return (
      <OuterDialog>
        <Button
          label="Open inner"
          data-testid="open-inner"
          onClick={() => setIsOpen(true)}
        />
        <BottomSheet isOpen={isOpen} onOpenChange={setIsOpen} label="inner sheet">
          <Section padding={4}>
            <Text type="body">Sheet body</Text>
          </Section>
        </BottomSheet>
      </OuterDialog>
    );
  },
};

export const CommandPaletteInDialog: Story = {
  render: function Render() {
    const [isOpen, setIsOpen] = useState(false);
    const source = useFruitSource();
    return (
      <OuterDialog>
        <Button
          label="Open inner"
          data-testid="open-inner"
          onClick={() => setIsOpen(true)}
        />
        <CommandPalette
          isOpen={isOpen}
          onOpenChange={setIsOpen}
          searchSource={source}
        />
      </OuterDialog>
    );
  },
};

export const PopoverInDialog: Story = {
  render: () => (
    <OuterDialog>
      <Popover content={<Text type="body">Popover body</Text>}>
        <Button label="Open inner" data-testid="open-inner" />
      </Popover>
    </OuterDialog>
  ),
};

export const DropdownMenuInDialog: Story = {
  render: () => (
    <OuterDialog>
      <div data-testid="inner-host">
        <DropdownMenu
          button={{label: 'Open inner'}}
          items={[
            {label: 'Edit', onClick: () => {}},
            {label: 'Delete', onClick: () => {}},
          ]}
        />
      </div>
    </OuterDialog>
  ),
};

export const SelectorInDialog: Story = {
  render: function Render() {
    const [value, setValue] = useState<string | undefined>();
    return (
      <OuterDialog>
        <div data-testid="inner-host">
          <Selector
            label="Fruit"
            options={FRUITS}
            value={value}
            onChange={setValue}
          />
        </div>
      </OuterDialog>
    );
  },
};

export const MultiSelectorInDialog: Story = {
  render: function Render() {
    const [value, setValue] = useState<string[]>([]);
    return (
      <OuterDialog>
        <div data-testid="inner-host">
          <MultiSelector
            label="Fruits"
            options={FRUITS}
            value={value}
            onChange={setValue}
          />
        </div>
      </OuterDialog>
    );
  },
};

export const ComplexSelectorInDialog: Story = {
  render: function Render() {
    const [value, setValue] = useState('Apple');
    return (
      <OuterDialog>
        <div data-testid="inner-host">
          <ComplexSelector<string>
            label="Fruit blend"
            value={value}
            onChange={setValue}
            triggerLabel={value}>
            {(selected, onChange, close) => (
              <VStack gap={2}>
                {FRUITS.map(fruit => (
                  <Button
                    key={fruit}
                    label={fruit}
                    variant={fruit === selected ? 'primary' : 'secondary'}
                    onClick={() => {
                      onChange(fruit);
                      close();
                    }}
                  />
                ))}
              </VStack>
            )}
          </ComplexSelector>
        </div>
      </OuterDialog>
    );
  },
};

export const TypeaheadInDialog: Story = {
  render: function Render() {
    const source = useFruitSource();
    const [value, setValue] = useState<{id: string; label: string} | null>(
      null,
    );
    return (
      <OuterDialog>
        <div data-testid="inner-host">
          <Typeahead
            label="Fruit"
            searchSource={source}
            value={value}
            onChange={setValue}
            hasEntriesOnFocus
          />
        </div>
      </OuterDialog>
    );
  },
};

export const LightboxInDialog: Story = {
  render: function Render() {
    const [isOpen, setIsOpen] = useState(false);
    return (
      <OuterDialog>
        <Button
          label="Open inner"
          data-testid="open-inner"
          onClick={() => setIsOpen(true)}
        />
        <Lightbox
          isOpen={isOpen}
          onOpenChange={setIsOpen}
          media={{
            src: 'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300"><rect width="400" height="300" fill="%23888"/></svg>',
            alt: 'Grey rectangle',
          }}
        />
      </OuterDialog>
    );
  },
};

export const MobileNavInDialog: Story = {
  render: function Render() {
    const [isOpen, setIsOpen] = useState(false);
    return (
      <OuterDialog>
        <Button
          label="Open inner"
          data-testid="open-inner"
          onClick={() => setIsOpen(true)}
        />
        <MobileNav isOpen={isOpen} onOpenChange={setIsOpen} header="Navigation">
          <SideNavSection title="Main">
            <SideNavItem label="Dashboard" href="/dashboard" />
          </SideNavSection>
        </MobileNav>
      </OuterDialog>
    );
  },
};

export const InfoTipInDialog: Story = {
  render: () => (
    <OuterDialog>
      <div data-testid="inner-host">
        <InfoTip content="Extra detail" />
      </div>
    </OuterDialog>
  ),
};

/** Controlled hover layers: the consumer owns visibility, so Escape should not fight them. */
export const ControlledTooltipInDialog: Story = {
  render: () => (
    <OuterDialog>
      <Tooltip content="A controlled tip" isOpen>
        <Button label="Trigger" data-testid="open-inner" />
      </Tooltip>
    </OuterDialog>
  ),
};

export const ControlledHoverCardInDialog: Story = {
  render: () => (
    <OuterDialog>
      <HoverCard content={<Text type="body">Card body</Text>} isOpen>
        <Button label="Trigger" data-testid="open-inner" />
      </HoverCard>
    </OuterDialog>
  ),
};

export const RequiredDialogInDialog: Story = {
  render: function Render() {
    const [isOpen, setIsOpen] = useState(false);
    return (
      <OuterDialog>
        <Button
          label="Open inner"
          data-testid="open-inner"
          onClick={() => setIsOpen(true)}
        />
        <Dialog
          isOpen={isOpen}
          onOpenChange={setIsOpen}
          purpose="required"
          width={380}
          aria-label="required">
          <Layout
            header={<DialogHeader title="Choose" />}
            content={
              <LayoutContent>
                <Button
                  label="Accept"
                  onClick={() => setIsOpen(false)}
                  data-testid="accept"
                />
              </LayoutContent>
            }
          />
        </Dialog>
      </OuterDialog>
    );
  },
};

// ─── Group B: standalone layers, nothing underneath ──────────────────────────

function Standalone({children}: {children: React.ReactNode}) {
  return (
    <div style={{padding: 24}} data-testid="inner-host">
      {children}
    </div>
  );
}

export const PopoverStandalone: Story = {
  render: () => (
    <Standalone>
      <Popover content={<Text type="body">Popover body</Text>}>
        <Button label="Open inner" data-testid="open-inner" />
      </Popover>
    </Standalone>
  ),
};

export const DropdownMenuStandalone: Story = {
  render: () => (
    <Standalone>
      <DropdownMenu
        button={{label: 'Open inner'}}
        items={[{label: 'Edit', onClick: () => {}}]}
      />
    </Standalone>
  ),
};

export const SelectorStandalone: Story = {
  render: function Render() {
    const [value, setValue] = useState<string | undefined>();
    return (
      <Standalone>
        <Selector
          label="Fruit"
          options={FRUITS}
          value={value}
          onChange={setValue}
        />
      </Standalone>
    );
  },
};

export const MultiSelectorStandalone: Story = {
  render: function Render() {
    const [value, setValue] = useState<string[]>([]);
    return (
      <Standalone>
        <MultiSelector
          label="Fruits"
          options={FRUITS}
          value={value}
          onChange={setValue}
        />
      </Standalone>
    );
  },
};

export const ComplexSelectorStandalone: Story = {
  render: function Render() {
    const [value, setValue] = useState('Apple');
    return (
      <Standalone>
        <ComplexSelector<string>
          label="Fruit blend"
          value={value}
          onChange={setValue}
          triggerLabel={value}>
          {(selected, onChange, close) => (
            <VStack gap={2}>
              {FRUITS.map(fruit => (
                <Button
                  key={fruit}
                  label={fruit}
                  variant={fruit === selected ? 'primary' : 'secondary'}
                  onClick={() => {
                    onChange(fruit);
                    close();
                  }}
                />
              ))}
            </VStack>
          )}
        </ComplexSelector>
      </Standalone>
    );
  },
};

export const TypeaheadStandalone: Story = {
  render: function Render() {
    const source = useFruitSource();
    const [value, setValue] = useState<{id: string; label: string} | null>(
      null,
    );
    return (
      <Standalone>
        <Typeahead
          label="Fruit"
          searchSource={source}
          value={value}
          onChange={setValue}
          hasEntriesOnFocus
        />
      </Standalone>
    );
  },
};

export const TooltipStandalone: Story = {
  render: () => (
    <Standalone>
      <Tooltip content="A hover tip">
        <Button label="Hover me" data-testid="open-inner" />
      </Tooltip>
    </Standalone>
  ),
};

export const HoverCardStandalone: Story = {
  render: () => (
    <Standalone>
      <HoverCard content={<Text type="body">Card body</Text>} delay={0}>
        <Button label="Hover me" data-testid="open-inner" />
      </HoverCard>
    </Standalone>
  ),
};

// ─── Group C: BottomSheetSwitcher ────────────────────────────────────────────

function SwitcherFlow({
  hasScrim,
  children,
}: {
  hasScrim: boolean;
  children?: React.ReactNode;
}) {
  const [active, setActive] = useState<string | null>(null);
  return (
    <div style={{padding: 24, minHeight: 400}}>
      <Button
        label="Open switcher"
        data-testid="open-switcher"
        onClick={() => setActive('one')}
      />
      <BottomSheetSwitcher
        activeSheet={active}
        onActiveSheetChange={setActive}
        hasScrim={hasScrim}
        aria-label="switcher">
        <BottomSheet sheetId="one" label="Step one" height="hug">
          <Section padding={4}>
            <VStack gap={3}>
              <Text type="body">Step one</Text>
              {children}
            </VStack>
          </Section>
        </BottomSheet>
      </BottomSheetSwitcher>
    </div>
  );
}

export const SwitcherAloneNonModal: Story = {
  render: () => <SwitcherFlow hasScrim={false} />,
};

export const SwitcherAloneModal: Story = {
  render: () => <SwitcherFlow hasScrim />,
};

export const SwitcherWithTooltip: Story = {
  render: () => (
    <SwitcherFlow hasScrim={false}>
      <Tooltip content="A hover tip">
        <Button label="Hover me" data-testid="open-inner" />
      </Tooltip>
    </SwitcherFlow>
  ),
};

export const DialogInSwitcher: Story = {
  render: function Render() {
    const [isOpen, setIsOpen] = useState(false);
    return (
      <SwitcherFlow hasScrim={false}>
        <Button
          label="Open inner"
          data-testid="open-inner"
          onClick={() => setIsOpen(true)}
        />
        <Dialog
          isOpen={isOpen}
          onOpenChange={setIsOpen}
          width={360}
          aria-label="inner">
          <Layout
            header={<DialogHeader title="Inner" onOpenChange={setIsOpen} />}
            content={
              <LayoutContent>
                <Text type="body">Inner dialog</Text>
              </LayoutContent>
            }
          />
        </Dialog>
      </SwitcherFlow>
    );
  },
};

/** A non-modal switcher opened over an already-open Dialog. */
export const SwitcherOverDialog: Story = {
  render: function Render() {
    const [isDialogOpen, setIsDialogOpen] = useState(false);
    const [active, setActive] = useState<string | null>(null);
    return (
      <div style={{padding: 24, minHeight: 400}}>
        <Button
          label="Open outer"
          data-testid="open-outer"
          onClick={() => setIsDialogOpen(true)}
        />
        <Dialog
          isOpen={isDialogOpen}
          onOpenChange={setIsDialogOpen}
          width={520}
          aria-label="outer">
          <Layout
            header={
              <DialogHeader title="Outer" onOpenChange={setIsDialogOpen} />
            }
            content={
              <LayoutContent>
                <Button
                  label="Open switcher"
                  data-testid="open-switcher"
                  onClick={() => setActive('one')}
                />
              </LayoutContent>
            }
          />
        </Dialog>
        <BottomSheetSwitcher
          activeSheet={active}
          onActiveSheetChange={setActive}
          hasScrim={false}
          aria-label="switcher">
          <BottomSheet sheetId="one" label="Step one" height="hug">
            <Section padding={4}>
              <Text type="body">Step one</Text>
            </Section>
          </BottomSheet>
        </BottomSheetSwitcher>
      </div>
    );
  },
};
