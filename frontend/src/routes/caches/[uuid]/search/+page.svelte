<script lang="ts">
  import { Input } from 'positron-components/components/ui/input';
  import { toast } from 'positron-components/components/util/general';
  import * as Select from 'positron-components/components/ui/select';
  import Table from '$lib/components/table/Table.svelte';
  import { columns } from './table.svelte';
  import {
    deletePath,
    SearchOrder,
    SearchSort,
    searchStorePaths,
    type SearchResult
  } from '$lib/client';

  let { data } = $props();

  let input = $state('');
  let sort: SearchSort = $state(SearchSort.STORE_PATH);
  let order: SearchOrder = $state(SearchOrder.ASC);
  let paths = $state<SearchResult[]>([]);
  let searchTrigger = $state(0);

  $effect(() => {
    let _ = searchTrigger;
    if (!input) return;
    searchStorePaths({
      path: { uuid: data.cacheInfo.uuid },
      body: {
        order,
        query: input,
        sort
      }
    }).then((result) => {
      if (!result.data) {
        toast.error('Failed to search cache');
        return;
      }
      paths = result.data;
    });
  });

  const delete_path = async (path: string) => {
    let res = await deletePath({
      path: { uuid: data.cacheInfo.uuid },
      body: { store_path: path }
    });

    if (res.error) {
      toast.error('Failed to delete path');
    } else {
      searchTrigger += 1;
    }
  };
</script>

<div class="flex w-full grow flex-col">
  <div class="flex w-full flex-none gap-2">
    <Input placeholder="Search..." bind:value={input} class="mb-1" />
    <Select.Root bind:value={sort} type="single" allowDeselect={false}>
      <Select.Trigger class="w-42">
        {sort}
      </Select.Trigger>
      <Select.Content>
        <Select.Group>
          {#each Object.values(SearchSort) as value}
            <Select.Item {value}>{value}</Select.Item>
          {/each}
        </Select.Group>
      </Select.Content>
    </Select.Root>
    <Select.Root bind:value={order} type="single" allowDeselect={false}>
      <Select.Trigger class="w-24">
        {order}
      </Select.Trigger>
      <Select.Content>
        <Select.Group>
          {#each Object.values(SearchOrder) as value}
            <Select.Item {value}>{value}</Select.Item>
          {/each}
        </Select.Group>
      </Select.Content>
    </Select.Root>
  </div>
  <Table
    data={paths}
    {columns}
    columnData={{
      write_access: data.cacheInfo.has_write_access,
      delete_path
    }}
    class="mt-2 min-h-0 grow"
  />
</div>
