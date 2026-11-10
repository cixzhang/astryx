// The gallery's cases: one per visual state QA captures. Families add theirs as they land.
export type GalleryCase = { id: string; family: string; title: string; props: Record<string, unknown> };

export const cases: GalleryCase[] = [
  { id: 'foundations-0001', family: 'Foundations', title: 'Token swatches', props: {} },
];
