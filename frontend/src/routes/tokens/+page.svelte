<script lang="ts">
  import { Button } from 'positron-components/components/ui/button';
  import FormDialog from 'positron-components/components/form/form-dialog.svelte';
  import Plus from '@lucide/svelte/icons/plus';
  import Table from '$lib/components/table/Table.svelte';
  import { columns } from './table.svelte';
  import { z } from 'zod';
  import { toast } from 'positron-components/components/util/general';
  import { invalidate } from '$app/navigation';
  import { Permission } from '$lib/permissions.svelte';
  import { deleteToken, type TokenInfo } from '$lib/backend/token.svelte';

  const { data } = $props();

  let selected: TokenInfo | undefined = $state();
  let deleteOpen = $state(false);
  let isLoading = $state(false);

  $effect(() => {
    if (data.error) {
      if (data.error === 'token_not_found') {
        toast.error('Token not found');
      } else if (data.error === 'token_other') {
        toast.error('Failed to load token');
      }

      const url = new URL(window.location.href);
      url.searchParams.delete('error');
      window.history.replaceState({}, '', url);
    }
  });

  const deleteItemConfirm = async () => {
    if (!selected) return;

    isLoading = true;
    let ret = await deleteToken({
      uuid: selected.uuid
    });
    isLoading = false;

    if (ret) {
      return { error: 'Failed to delete token' };
    } else {
      toast.success(`Token ${selected.name} deleted successfully`);
      invalidate((url) => url.pathname.startsWith('/api/token'));
    }
  };

  const startDeleteToken = (item: TokenInfo) => {
    selected = item;
    deleteOpen = true;
  };
</script>

<div class="p-4">
  <div class="ml-7 flex items-center md:m-0">
    <h3 class="text-xl font-medium">Token</h3>
    <Button class="ml-auto cursor-pointer" href="/tokens/create">
      <Plus />
      Create
    </Button>
  </div>
  <Table
    data={data.tokens}
    {columns}
    class="mt-4"
    columnData={{
      deleteToken: startDeleteToken
    }}
  />
</div>
<FormDialog
  title={`Delete Token`}
  description={`Do you really want to delete the token ${selected?.name}?`}
  confirm="Delete"
  confirmVariant="destructive"
  onsubmit={deleteItemConfirm}
  bind:open={deleteOpen}
  bind:isLoading
  schema={z.object({})}
/>
