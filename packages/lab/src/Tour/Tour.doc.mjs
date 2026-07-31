// Copyright (c) Meta Platforms, Inc. and affiliates.

/** @type {import('../../../core/src/docs-types').ComponentDoc} */

export const docs = {
  name: 'Tour',
  displayName: 'Tour',
  category: 'Feedback & Status',
  keywords: ["tour","nux","onboarding","walkthrough","product tour","coachmark","spotlight","guide","feature step","step"],
  description:
    'A guided product tour (NUX / onboarding). Tour is a controller that steps a user through a sequence of spotlight callouts anchored to elements on the page. It renders no chrome itself — it owns the active-step state and shares it with declaratively-nested TourStep children (step order follows the children). Experimental: lands in lab first (facebook/astryx#4239).',
  components: [
    {
      name: 'Tour',
      description:
        'Controller for a guided tour. Coordinates the active step among its TourStep children, exposes advance / retreat / complete / dismiss, and optionally dims the background. Controlled via isActive — the consumer owns "has this user seen the tour?".',
      props: [
        {
          name: 'isActive',
          type: 'boolean',
          description:
            'Whether the tour is running. When false nothing renders and the tour resets to the first step, so it restarts cleanly next time it becomes active.',
          required: true,
        },
        {
          name: 'children',
          type: 'ReactNode',
          description:
            'The tour steps — TourStep elements. Step order follows their order here; only the active step renders its callout.',
          slotElements: [{__element: 'TourStep', props: {heading: 'Step'}, children: 'Step body'}],
        },
        {
          name: 'onDismiss',
          type: '(source: TourDismissSource) => void',
          description:
            "Called on every exit, with the reason ('backdrop' | 'escape' | 'close' | 'skip' | 'complete'). The consumer flips isActive to false in response.",
          required: true,
        },
        {
          name: 'onComplete',
          type: '() => void',
          description: 'Called when the tour completes (advancing past the final step), after onDismiss(\'complete\').',
        },
        {
          name: 'hasBackdrop',
          type: 'boolean',
          description: 'Show a dimmed background behind the active step.',
          default: 'false',
        },
        {
          name: 'isStepCountShown',
          type: 'boolean',
          description: 'Show the step count ("2 of 5") in each step.',
          default: 'false',
        },
        {
          name: 'handleRef',
          type: 'Ref<TourHandle>',
          description: 'Imperative handle exposing next() / previous() for driving the tour from outside the step UI.',
        },
      ],
    },
    {
      name: 'TourStep',
      description:
        'A single spotlight step. Highlights a target element and renders a callout anchored to it (via the core Popover), with a heading, body, optional step progress, and back / next / close controls. Registers with the parent Tour on mount and renders its callout only while active.',
      props: [
        {
          name: 'targetRef',
          type: 'RefObject<HTMLElement>',
          description:
            "Ref to the element this step points at. The callout anchors to it like a Popover trigger; it must be a <button> or [role=\"button\"] element (Popover's anchorRef contract).",
          required: true,
        },
        {
          name: 'heading',
          type: 'ReactNode',
          description: 'Step heading.',
          required: true,
        },
        {
          name: 'children',
          type: 'ReactNode',
          description: 'Step body content.',
        },
        {
          name: 'placement',
          type: "'above' | 'below' | 'start' | 'end'",
          description: 'Where the callout sits relative to the target.',
          default: "'below'",
        },
      ],
    },
  ],
  usage: {
    description:
      'Use a Tour to introduce a feature or onboard a user through a few key parts of the UI. Compose it from TourStep children, each pointing at an element via targetRef; the Tour advances through them with Next/Back and ends on Done. It is controlled — keep isActive in your own state and persist "has seen this tour" yourself (the component only reports dismissal via onDismiss). Built on the existing Popover/Layer anchoring and overlay tokens rather than a bespoke positioning engine.',
    bestPractices: [
      { guidance: true, description: 'Keep tours short — a few high-value steps. Long tours get skipped.' },
      { guidance: true, description: 'Persist completion yourself (from onDismiss) so a user does not see the same tour on every visit.' },
      { guidance: true, description: 'Point each step at a stable, visible element; ensure the target is on-screen before its step becomes active.' },
      { guidance: false, description: 'Gate essential, must-see information behind a tour step — users can dismiss it; put critical info inline.' },
      { guidance: false, description: 'Use a tour as a substitute for clear UI — fix confusing interfaces rather than narrating them.' },
    ],
  },
};
