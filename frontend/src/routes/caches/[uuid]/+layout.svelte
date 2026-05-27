<script lang="ts">
  import { Separator } from '@profidev/pleiades/components/ui/separator';
  import SimpleSidebar from '@profidev/pleiades/components/nav/simple-sidebar.svelte';
  import { Button } from '@profidev/pleiades/components/ui/button';
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import type { CacheDetails } from '$lib/client/types.gen.js';
  import { goto } from '$app/navigation';
  import { Skeleton } from '@profidev/pleiades/components/ui/skeleton';

  const { children, data } = $props();

  let cacheInfo: CacheDetails | undefined = $state();

  $effect(() => {
    data.cacheRes.then((res) => {
      if (!res.data) {
        if (res.response?.status === 404) {
          goto('/caches?error=not_found');
        } else {
          goto('/caches?error=other');
        }
        return;
      }

      cacheInfo = res.data;
    });
  });

  const routes = $derived([
    {
      title: 'Overview',
      href: `/caches/${data.uuid}`
    },
    {
      title: 'Search',
      href: `/caches/${data.uuid}/search`
    },
    {
      title: 'Settings',
      href: `/caches/${data.uuid}/settings`
    }
  ]);
</script>

<div class="flex h-full max-h-screen w-full flex-col space-y-6 p-4">
  <div class="mt-1! mb-0 ml-7 flex items-center md:m-0">
    <Button size="icon" variant="ghost" href="/caches" class="mr-2">
      <ArrowLeft class="size-5" />
    </Button>
    <h3 class="flex text-xl font-medium">
      Cache: {#if !cacheInfo}
        <Skeleton class="ml-2 h-7 w-20" />
      {:else}
        {cacheInfo.name}
      {/if}
    </h3>
  </div>
  <Separator class="my-4" />
  <div
    class="flex min-h-0 grow flex-col space-y-4 lg:flex-row lg:space-y-0 lg:space-x-6"
  >
    <aside class="lg:w-40 lg:min-w-40">
      <SimpleSidebar items={routes} class="" />
    </aside>
    <Separator orientation="horizontal" class="lg:hidden" />
    <Separator orientation="vertical" class="hidden lg:block" />
    {@render children()}
  </div>
</div>
