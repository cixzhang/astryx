import type { Answer, Result, Sources } from './app.contract.d.ts';
import { cases } from './cases.ts';

export const grants = '';

const sources: Sources = {
  galleryCases: (): Result<'galleryCases'> => ({ items: cases.map(({ id, family, title }) => ({ id, family, title })) }),
};
export const answer: Answer = (source, args, store, storage, native) =>
  sources[source](args, store, storage, native);
