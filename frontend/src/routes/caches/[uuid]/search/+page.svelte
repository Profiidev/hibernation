<script lang="ts">
  import {
    searchCache,
    SearchOrder,
    SearchSort,
    type SearchResult
  } from '$lib/backend/cache.svelte';
  import { Input } from 'positron-components/components/ui/input';
  import { toast } from 'positron-components/components/util/general';
  import * as Select from 'positron-components/components/ui/select';
  import Table from '$lib/components/table/Table.svelte';
  import { columns } from './table.svelte';

  let { data } = $props();

  let input = $state('');
  let sort: SearchSort = $state(SearchSort.StorePath);
  let order: SearchOrder = $state(SearchOrder.Ascending);
  let paths = $state<SearchResult[]>([]);

  $effect(() => {
    searchCache(data.cacheInfo.uuid, input, sort, order).then((result) => {
      if (result === undefined) {
        toast.error('Failed to search cache');
        return;
      }
      paths = result;
    });
  });
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
  <Table data={paths} {columns} class="mt-2 min-h-0 grow" />
</div>
