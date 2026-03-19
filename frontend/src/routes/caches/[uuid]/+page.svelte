<script lang="ts">
  import { Separator } from 'positron-components/components/ui/separator';
  import { Button } from 'positron-components/components/ui/button';
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import Trash from '@lucide/svelte/icons/trash';

  const { data } = $props();

  let token = $state<string>();
</script>

<div class="flex h-full w-full flex-col space-y-6 p-4">
  <div class="mt-1! mb-0 ml-7 flex items-center md:m-0">
    <Button size="icon" variant="ghost" href="/caches" class="mr-2">
      <ArrowLeft class="size-5" />
    </Button>
    <h3 class="text-xl font-medium">Cache: {data.cacheInfo.name}</h3>
    <Button
      class="ml-auto cursor-pointer"
      onclick={() => console.log('123')}
      variant="destructive"
    >
      <Trash />
      Delete
    </Button>
  </div>
  <Separator class="my-4" />
  <div
    class="flex grow flex-col space-y-4 lg:flex-row lg:space-y-0 lg:space-x-6"
  >
    <div class="flex-1">
      <h4 class="mb-2">Settings</h4>
    </div>
  </div>
</div>
