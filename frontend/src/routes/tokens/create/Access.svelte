<script lang="ts">
  import BaseForm from '@profidev/pleiades/components/form/base-form.svelte';
  import type { StageProps } from '@profidev/pleiades/components/form/types';
  import { Label } from '@profidev/pleiades/components/ui/label';
  import { Switch } from '@profidev/pleiades/components/ui/switch';
  import { onMount } from 'svelte';
  import { z } from 'zod';
  import {
    listCaches,
    type CacheInfo,
    type TokenRule
  } from '#lib/client/index.ts';
  import TokenRules from '../TokenRules.svelte';

  let { initialValue, onsubmit, footer, isLoading }: StageProps = $props();

  // Rules live outside the form schema, like cache mappings on the group page
  const schema = z.object({});
  // svelte-ignore state_referenced_locally
  let restricted = $state<boolean>(initialValue?.restricted ?? false);
  // svelte-ignore state_referenced_locally
  let rules = $state<TokenRule[]>(initialValue?.rules ?? []);
  let caches: CacheInfo[] = $state([]);

  onMount(async () => {
    caches = (await listCaches()).data ?? [];
  });

  const value = () => ({ restricted, rules: restricted ? rules : [] });

  export const getValue = () => value();
</script>

<BaseForm {schema} onsubmit={() => onsubmit(value())} {footer} bind:isLoading>
  {#snippet children()}
    <div class="flex max-h-[60vh] min-h-0 flex-col gap-4">
      <Label class="flex items-center gap-2">
        <Switch bind:checked={restricted} />
        Restrict to specific caches and paths
      </Label>
      {#if restricted}
        <TokenRules {caches} bind:rules />
      {:else}
        <p class="text-muted-foreground text-sm">
          The token gets the same cache access as your account.
        </p>
      {/if}
    </div>
  {/snippet}
</BaseForm>
