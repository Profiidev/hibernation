import type { ColumnDef } from '@tanstack/table-core';
import * as DataTable from '@profidev/pleiades/components/ui/data-table';
import { createColumn } from '@profidev/pleiades/components/table/helpers.svelte';
import type { TokenInfo } from '$lib/client';
import Actions from '@profidev/pleiades/components/table/actions.svelte';

export const columns = ({
  deleteToken
}: {
  deleteToken: (token: TokenInfo) => void;
}): ColumnDef<TokenInfo>[] => [
  createColumn('name', 'Name'),
  createColumn('exp', 'Expires At', (value: string) =>
    new Date(value).toLocaleString(navigator.languages || [navigator.language])
  ),
  createColumn('last_used', 'Last Used', (value: string | undefined) =>
    value
      ? new Date(value).toLocaleString(
          navigator.languages || [navigator.language]
        )
      : 'Never'
  ),
  createColumn('uuid', 'UUID'),
  {
    accessorKey: 'actions',
    cell: ({ row }) =>
      DataTable.renderComponent(Actions, {
        delete_disabled: false,
        edit: `/tokens/${row.original.uuid}`,
        edit_disabled: false,
        remove: () => deleteToken(row.original)
      }),
    enableHiding: false,
    header: () => {}
  }
];
