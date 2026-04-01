<script lang="ts">
  import { Button } from 'positron-components/components/ui/button';
  import Plus from '@lucide/svelte/icons/plus';
  import { toast } from 'positron-components/components/util/general';
  import { Permission } from '$lib/permissions.svelte.js';
  import { ScrollArea } from 'positron-components/components/ui/scroll-area';
  import Lock from '@lucide/svelte/icons/lock';
  import LockOpen from '@lucide/svelte/icons/lock-open';
  import { Badge } from 'positron-components/components/ui/badge';
  import { size_to_gib } from '$lib/backend/cache.svelte.js';
  import { Progress } from 'positron-components/components/ui/progress';

  const { data } = $props();

  $effect(() => {
    if (data.error) {
      if (data.error === 'cache_not_found') {
        toast.error('Cache not found');
      } else if (data.error === 'cache_other') {
        toast.error('Failed to load cache');
      }

      const url = new URL(window.location.href);
      url.searchParams.delete('error');
      window.history.replaceState({}, '', url);
    }
  });
</script>

<div class="p-4">
  <div class="ml-7 flex items-center md:m-0">
    <h3 class="text-xl font-medium">Caches</h3>
    <Button
      class="ml-auto cursor-pointer"
      href="/caches/create"
      disabled={!data.user?.permissions.includes(Permission.CACHE_CREATE)}
    >
      <Plus />
      Create
    </Button>
  </div>
  <ScrollArea class="mt-2 min-h-0 grow">
    <div
      class="grid size-full auto-rows-min grid-cols-[repeat(auto-fill,minmax(24rem,1fr))] gap-2"
    >
      {#if (data.caches?.length ?? 0) === 0}
        <p class="text-muted-foreground">
          No caches found. Create one to get started.
        </p>
      {/if}
      {#each data.caches as cache}
        {@const size_gib = size_to_gib(cache.size)}
        {@const quota_gib = size_to_gib(cache.quota)}
        {@const usage_percent = (size_gib / quota_gib) * 100}
        <Button
          class="flex h-24 w-full max-w-150 cursor-pointer flex-col items-start rounded-xl border p-2"
          variant="outline"
          href={'/caches/' + cache.uuid}
        >
          <div class="flex w-full items-center">
            <h4 class="text-xl font-medium">{cache.name}</h4>
            <Badge
              variant="outline"
              class={'ml-auto flex items-center ' +
                (cache.public
                  ? 'border-green-500 text-green-500'
                  : 'border-orange-400 text-orange-400')}
            >
              {#if cache.public}
                <LockOpen />
                Public
              {:else}
                <Lock />
                Private
              {/if}
            </Badge>
          </div>
          <p class="text-muted-foreground mt-2 ml-auto">
            {size_gib.toFixed(2)}GiB / {quota_gib.toFixed(2)}GiB ({usage_percent.toFixed(
              1
            )}%)
          </p>
          <Progress
            min={0}
            max={quota_gib}
            value={size_gib}
            class={(usage_percent > 90
              ? '*:data-[slot=progress-indicator]:bg-red-500'
              : usage_percent > 70
                ? '*:data-[slot=progress-indicator]:bg-yellow-500'
                : '') + ' h-2'}
          />
        </Button>
      {/each}
    </div>
  </ScrollArea>
</div>
