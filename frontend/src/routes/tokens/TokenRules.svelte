<script lang="ts">
  import Multiselect from '@profidev/pleiades/components/table/multiselect.svelte';
  import { Button } from '@profidev/pleiades/components/ui/button';
  import { Checkbox } from '@profidev/pleiades/components/ui/checkbox';
  import { Label } from '@profidev/pleiades/components/ui/label';
  import { Textarea } from '@profidev/pleiades/components/ui/textarea';
  import Trash from '@lucide/svelte/icons/trash';
  import Plus from '@lucide/svelte/icons/plus';
  import FlaskConical from '@lucide/svelte/icons/flask-conical';
  import {
    testTokenRule,
    type CacheInfo,
    type TestRuleResponse,
    type TokenRule
  } from '$lib/client';
  import { linesToPatterns, patternsToLines } from './rules';

  interface Props {
    caches: CacheInfo[];
    rules: TokenRule[];
    disabled?: boolean;
  }

  let { caches, rules = $bindable(), disabled }: Props = $props();

  let remainingCaches = $derived(
    caches.filter((cache) => !rules.some((rule) => rule.cache === cache.uuid))
  );
  // Tester state per rule index
  let samples: string[] = $state([]);
  let results: (TestRuleResponse | undefined)[] = $state([]);
  let testing: boolean[] = $state([]);

  const cacheName = (uuid: string) =>
    caches.find((c) => c.uuid === uuid)?.name ?? uuid;

  const update = (i: number, patch: Partial<TokenRule>) => {
    // this needs to be done to trigger reactivity
    rules = rules.map((r, j) => (j === i ? { ...r, ...patch } : r));
  };

  const remove = (i: number) => {
    rules = rules.filter((_, j) => j !== i);
    samples = samples.filter((_, j) => j !== i);
    results = results.filter((_, j) => j !== i);
    testing = testing.filter((_, j) => j !== i);
  };

  const runTest = async (i: number) => {
    const rule = rules[i];
    testing[i] = true;
    const res = await testTokenRule({
      body: {
        cache: rule.cache,
        exclude: rule.exclude,
        include: rule.include,
        samples: linesToPatterns(samples[i] ?? '')
      }
    });
    testing[i] = false;
    results[i] = res.data ?? {
      allowed: [],
      denied: [],
      error: 'Failed to test rule',
      samples: []
    };
  };
</script>

<div class="flex flex-col gap-4">
  {#each rules as rule, i (rule.cache)}
    <div class="flex flex-col gap-3 rounded-md border p-3">
      <div class="grid grid-cols-[1fr_2.5rem] gap-2">
        <Multiselect
          data={[
            { name: cacheName(rule.cache), uuid: rule.cache },
            ...remainingCaches
          ].map((cache) => ({
            label: cache.name,
            value: cache.uuid
          }))}
          label="Cache"
          single
          selected={[rule.cache]}
          {disabled}
          onSelectChange={(selected) => {
            if (selected.length > 0) {
              update(i, { cache: selected[0] });
            }
          }}
        />
        <Button
          size="icon"
          type="button"
          variant="destructive"
          class="min-w-10"
          aria-label="Remove rule"
          {disabled}
          onclick={() => remove(i)}
        >
          <Trash />
        </Button>
      </div>
      <div class="flex flex-wrap gap-4">
        {#each ['pull', 'push', 'delete'] as const as action (action)}
          <Label class="flex items-center gap-2 capitalize">
            <Checkbox
              checked={rule[action]}
              {disabled}
              onCheckedChange={(checked) => update(i, { [action]: checked })}
            />
            {action}
          </Label>
        {/each}
      </div>
      <div class="grid grid-cols-1 gap-2 md:grid-cols-2">
        <Label class="flex flex-col items-start gap-1">
          Include (regex per line, empty = all)
          <Textarea
            class="font-mono"
            placeholder="^myapp-"
            value={patternsToLines(rule.include)}
            {disabled}
            onchange={(e) =>
              update(i, { include: linesToPatterns(e.currentTarget.value) })}
          />
        </Label>
        <Label class="flex flex-col items-start gap-1">
          Exclude (regex per line)
          <Textarea
            class="font-mono"
            placeholder="-dev$"
            value={patternsToLines(rule.exclude)}
            {disabled}
            onchange={(e) =>
              update(i, { exclude: linesToPatterns(e.currentTarget.value) })}
          />
        </Label>
      </div>
      <p class="text-muted-foreground text-sm">
        Patterns match the store path name after the hash, e.g. <code
          >myapp-1.0</code
        >. A path is allowed if it matches any include and no exclude pattern.
      </p>
      <div class="flex flex-col gap-2">
        <Label class="flex flex-col items-start gap-1">
          Test names (one per line)
          <Textarea
            class="font-mono"
            placeholder="myapp-1.0"
            bind:value={samples[i]}
          />
        </Label>
        <Button
          type="button"
          variant="outline"
          class="w-fit cursor-pointer"
          disabled={testing[i]}
          onclick={() => runTest(i)}
        >
          <FlaskConical />
          Test patterns
        </Button>
        {#if results[i]}
          {@const result = results[i]}
          {#if result.error}
            <p class="text-destructive text-sm">{result.error}</p>
          {:else}
            <ul class="font-mono text-sm">
              {#each result.samples as sample, s (s)}
                <li
                  class={sample.allowed ? 'text-green-600' : 'text-destructive'}
                >
                  {sample.allowed ? 'allowed' : 'denied'}: {sample.path}
                </li>
              {/each}
            </ul>
            <div class="grid grid-cols-1 gap-2 text-sm md:grid-cols-2">
              <div>
                <h6 class="font-medium">Allowed paths in cache</h6>
                <ul class="max-h-40 overflow-auto font-mono break-all">
                  {#each result.allowed as path (path)}
                    <li>{path}</li>
                  {:else}
                    <li class="text-muted-foreground">None</li>
                  {/each}
                </ul>
              </div>
              <div>
                <h6 class="font-medium">Denied paths in cache</h6>
                <ul class="max-h-40 overflow-auto font-mono break-all">
                  {#each result.denied as path (path)}
                    <li>{path}</li>
                  {:else}
                    <li class="text-muted-foreground">None</li>
                  {/each}
                </ul>
              </div>
            </div>
          {/if}
        {/if}
      </div>
    </div>
  {:else}
    <p class="text-muted-foreground text-sm">
      No rules: this token can not access any cache.
    </p>
  {/each}
  {#if remainingCaches.length > 0}
    <Button
      type="button"
      size="icon"
      aria-label="Add rule"
      {disabled}
      onclick={() => {
        rules = [
          ...rules,
          {
            cache: remainingCaches[0].uuid,
            delete: false,
            exclude: [],
            include: [],
            pull: true,
            push: false
          }
        ];
      }}
    >
      <Plus />
    </Button>
  {/if}
</div>
