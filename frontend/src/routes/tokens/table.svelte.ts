import type { ColumnDef } from '@tanstack/table-core';
import * as DataTable from 'positron-components/components/ui/data-table';
import { createColumn } from 'positron-components/components/table/helpers.svelte';
import type { TokenInfo } from '$lib/client';
import Actions from 'positron-components/components/table/actions.svelte';

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
    header: () => {},
    cell: ({ row }) => {
      return DataTable.renderComponent(Actions, {
        edit_disabled: false,
        delete_disabled: false,
        editHref: `/tokens/${row.original.uuid}`,
        remove: () => deleteToken(row.original)
      });
    },
    enableHiding: false
  }
];
