import type { ComponentPropsWithoutRef, FC } from 'react';

type PraxisNodeProperties = ComponentPropsWithoutRef<'div'> & Record<string, unknown>;

export const PraxisNode: FC<PraxisNodeProperties> = (properties) => (
  <div data-testid="praxis-node" {...properties} />
);
