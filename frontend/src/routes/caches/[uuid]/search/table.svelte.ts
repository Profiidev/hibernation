import type { ColumnDef } from '@tanstack/table-core';
import { createColumnCell } from 'positron-components/components/table/helpers.svelte';
import type { SearchResult } from '$lib/backend/cache.svelte';
import { renderComponent } from 'positron-components/components/ui/data-table';
import TableHead from './TableHead.svelte';

const createColumn = <T, C>(
  key: string,
  title: string,
  formatter?: (value: T) => string
): ColumnDef<C> => {
  return {
    accessorKey: key,
    ...createColumnCell(key, formatter),
    header: ({ column }) =>
      renderComponent(TableHead, {
        title
      })
  };
};

export const columns = (): ColumnDef<SearchResult>[] => [
  createColumn('store_path', 'StorePath'),
  createColumn(
    'size',
    'Size',
    (value: number) => `${(value / (1024 * 1024)).toFixed(2)} MiB`
  ),
  createColumn('created_at', 'Created At', (value: string) =>
    new Date(value).toLocaleString(navigator.languages || [navigator.language])
  ),
  createColumn(
    'last_accessed_at',
    'Last Accessed At',
    (value: string | undefined) =>
      value
        ? new Date(value).toLocaleString(
            navigator.languages || [navigator.language]
          )
        : 'Never'
  ),
  createColumn('accessed', 'Access Count')
];
