<script lang="ts">
  import { Separator } from 'positron-components/components/ui/separator';
  import SimpleSidebar from 'positron-components/components/nav/simple-sidebar.svelte';
  import { Button } from 'positron-components/components/ui/button';
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';

  const { children, data } = $props();

  const routes = $derived([
    {
      title: 'Overview',
      href: `/caches/${data.cacheInfo.uuid}`
    },
    {
      title: 'Search',
      href: `/caches/${data.cacheInfo.uuid}/search`
    },
    {
      title: 'Settings',
      href: `/caches/${data.cacheInfo.uuid}/settings`
    }
  ]);
</script>

<div class="flex h-full max-h-screen w-full flex-col space-y-6 p-4">
  <div class="mt-1! mb-0 ml-7 flex items-center md:m-0">
    <Button size="icon" variant="ghost" href="/caches" class="mr-2">
      <ArrowLeft class="size-5" />
    </Button>
    <h3 class="text-xl font-medium">Cache: {data.cacheInfo.name}</h3>
  </div>
  <Separator class="my-4" />
  <div
    class="flex min-h-0 grow flex-col space-y-4 lg:flex-row lg:space-y-0 lg:space-x-6"
  >
    <aside class="lg:w-40">
      <SimpleSidebar items={routes} class="" />
    </aside>
    <Separator orientation="horizontal" class="lg:hidden" />
    <Separator orientation="vertical" class="hidden lg:block" />
    {@render children()}
  </div>
</div>
