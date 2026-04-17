import type { ColumnDef } from '@tanstack/table-core';
import { createColumnCell } from 'positron-components/components/table/helpers.svelte';
import { renderComponent } from 'positron-components/components/ui/data-table';
import TableHead from './TableHead.svelte';
import * as DataTable from 'positron-components/components/ui/data-table';
import Actions from './Actions.svelte';
import type { SearchResult } from '$lib/client';

const createColumn = <C>(
  key: string,
  title: string,
  formatter?: (value: any) => string
): ColumnDef<C> => ({
  accessorKey: key,
  ...createColumnCell(key, formatter),
  header: () =>
    renderComponent(TableHead, {
      title
    })
});

export const columns = ({
  write_access,
  delete_path
}: {
  write_access: boolean;
  delete_path: (path: string) => void;
}): ColumnDef<SearchResult>[] => [
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
  createColumn('accessed', 'Access Count'),
  {
    accessorKey: 'actions',
    cell: ({ row }) =>
      DataTable.renderComponent(Actions, {
        delete_disabled: !write_access,
        remove: () => delete_path(row.original.store_path)
      }),
    enableHiding: false,
    header: () => {}
  }
];
