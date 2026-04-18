<script lang="ts">
  import { Progress } from '@profidev/pleiades/components/ui/progress';
  import * as Code from '$lib/components/code';
  import { ScrollArea } from '@profidev/pleiades/components/ui/scroll-area';
  import { size_to_gib } from '$lib/backend/util.svelte.js';

  let { data } = $props();

  let size_gib = $derived(size_to_gib(data.cacheInfo.size ?? 0));
  let quota_gib = $derived(size_to_gib(data.cacheInfo.quota));
  let usage_percent = $derived((size_gib / quota_gib) * 100);

  const url = $derived.by(() => {
    let siteUrl = new URL(data.generalSettings?.site_url || 'localhost:5173');
    if (data.generalSettings?.virtual_host_routing) {
      siteUrl.host = `${data.cacheInfo.name}.${siteUrl.host}`;
      return siteUrl.origin;
    } else {
      siteUrl.pathname = `/api/nix/${data.cacheInfo.name}`;
      return siteUrl.href;
    }
  });
  const nixConfig = $derived(`{
  nixConfig = {
    extra-substituters = [
      "${url}"
    ];
    extra-trusted-public-keys = [
      "${data.cacheInfo.sig_key}"
    ];
  };

  ...
}`);
</script>

<ScrollArea class="h-full w-full">
  <div class="grid w-full grid-cols-1 gap-4 2xl:grid-cols-[1fr_auto_1fr]">
    <div class="flex min-w-0 flex-col">
      <p class="text-lg">Usage:</p>
      <div class="mt-2 mb-1 flex">
        <p>Number of paths:</p>
        <p class="text-muted-foreground ml-auto">
          {data.cacheInfo.nar_count}
        </p>
      </div>
      <div class="mb-1 flex">
        <p>Cache size:</p>
        <p class="text-muted-foreground ml-auto">
          {size_gib.toFixed(2)}GiB / {quota_gib.toFixed(2)}GiB ({usage_percent.toFixed(
            1
          )}%)
        </p>
      </div>
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
      <p class="mt-8 text-lg">Pushing paths:</p>
      <Code.Root
        code={`hibernation push ${data.cacheInfo.name} /nix/store/...`}
        lang="bash"
        class="mt-2 h-auto"
        hideLines
      >
        <Code.CopyButton />
      </Code.Root>
      <p class="mt-8 text-lg">Pulling paths:</p>
      <p class="text-md mt-2">Cache URL:</p>
      <Code.Root code={url} lang="nix" class="mt-2 h-auto" hideLines>
        <Code.CopyButton />
      </Code.Root>
      <p class="text-md mt-2">Cache Public Key:</p>
      <Code.Root
        code={data.cacheInfo.sig_key}
        lang="nix"
        hideLines
        class="mt-2 h-auto"
      >
        <Code.CopyButton />
      </Code.Root>
      <p class="text-md mt-2">Nix config:</p>
      <Code.Root code={nixConfig} lang="nix" class="mt-2 h-auto">
        <Code.CopyButton />
      </Code.Root>
    </div>
  </div>
</ScrollArea>
