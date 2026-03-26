<script lang="ts">
  import { Button } from 'positron-components/components/ui/button';
  import {
    clearCache,
    deleteCache,
    editCache,
    EvictionPolicy
  } from '$lib/backend/cache.svelte.js';
  import BaseForm from 'positron-components/components/form/base-form.svelte';
  import { cacheSchema, keySchema, quotaSchema } from './schema.svelte.js';
  import FormInput from 'positron-components/components/form/form-input.svelte';
  import { Spinner } from 'positron-components/components/ui/spinner';
  import Save from '@lucide/svelte/icons/save';
  import FormSwitch from 'positron-components/components/form/form-switch.svelte';
  import type { FormValue } from 'positron-components/components/form/types';
  import { toast } from 'positron-components/components/util/general';
  import { RequestError } from 'positron-components/backend';
  import FormDialog from 'positron-components/components/form/form-dialog.svelte';
  import { z } from 'zod';
  import Trash from '@lucide/svelte/icons/trash';
  import Ban from '@lucide/svelte/icons/ban';
  import KeyRound from '@lucide/svelte/icons/key-round';
  import HardDrive from '@lucide/svelte/icons/hard-drive';
  import Lock from '@lucide/svelte/icons/lock';
  import LockOpen from '@lucide/svelte/icons/lock-open';
  import { goto } from '$app/navigation';
  import FormSelect from 'positron-components/components/form/form-select.svelte';
  import { TagsInput } from 'positron-components/components/ui-extra/tags-input';
  import { Label } from 'positron-components/components/ui/label';

  const { data } = $props();

  let downstreamCaches = $derived(data.cacheInfo.downstream_caches);
  let readonly = $derived(!data.cacheInfo.has_write_access);
  let deleteOpen = $state(false);
  let clearOpen = $state(false);
  let keyOpen = $state(false);
  let quotaOpen = $state(false);
  let visibilityOpen = $state(false);

  const onsubmit = async (form: FormValue<typeof cacheSchema>) => {
    let res = await editCache(data.cacheInfo.uuid, {
      ...data.cacheInfo,
      ...form,
      eviction_policy: form.eviction_policy[0],
      downstream_caches: downstreamCaches
    });

    if (res) {
      if (res === RequestError.Conflict) {
        return { error: 'This cache name is already in use', field: 'name' };
      } else {
        return { error: 'Failed to update cache' };
      }
    } else {
      toast.success(`Cache ${data.cacheInfo.name} updated successfully`);
      return { error: '' };
    }
  };

  const deleteConfirm = async () => {
    let res = await deleteCache({ uuid: data.cacheInfo.uuid });

    if (res) {
      return { error: 'Failed to delete cache' };
    } else {
      toast.success(`Cache ${data.cacheInfo.name} deleted successfully`);
      setTimeout(() => {
        goto('/caches');
      });
    }
  };

  const clearConfirm = async () => {
    let res = await clearCache(data.cacheInfo.uuid);

    if (res) {
      return { error: 'Failed to clear cache' };
    } else {
      toast.success(`Cache ${data.cacheInfo.name} cleared successfully`);
    }
  };

  const visibilityConfirm = async () => {
    let res = await editCache(data.cacheInfo.uuid, {
      ...data.cacheInfo,
      public: !data.cacheInfo.public
    });

    if (res) {
      toast.error(
        `Failed to change cache visibility for ${data.cacheInfo.name}`
      );
    } else {
      toast.success(
        `Cache ${data.cacheInfo.name} is now ${data.cacheInfo.public ? 'private' : 'public'}`
      );
    }
  };

  const quotaConfirm = async (form: FormValue<typeof quotaSchema>) => {
    let res = await editCache(data.cacheInfo.uuid, {
      ...data.cacheInfo,
      quota: form.quota * 1024 * 1024
    });

    if (res) {
      return { error: 'Failed to change cache quota' };
    } else {
      toast.success(`Cache ${data.cacheInfo.name} quota updated successfully`);
    }
  };

  const keyConfirm = async (form: FormValue<typeof keySchema>) => {
    let res = await editCache(data.cacheInfo.uuid, {
      ...data.cacheInfo,
      sig_key: form.sig_key
    });

    if (res) {
      if (res === RequestError.NotAcceptable) {
        return {
          error: 'The provided key is not a valid public signing key',
          field: 'sig_key'
        };
      } else {
        return { error: 'Failed to change cache key' };
      }
    } else {
      toast.success(`Cache ${data.cacheInfo.name} key updated successfully`);
    }
  };
</script>

<div class="grow">
  <BaseForm
    schema={cacheSchema}
    {onsubmit}
    initialValue={{
      ...data.cacheInfo,
      eviction_policy: [data.cacheInfo.eviction_policy]
    }}
    class="flex flex-1 flex-col"
  >
    {#snippet children({ props })}
      <div class="grid grid-cols-1 gap-4 xl:grid-cols-[1fr_auto_1fr]">
        <div>
          <FormInput
            {...props}
            key="name"
            label="Cache Name"
            placeholder="Enter cahce name"
            {readonly}
          />
          <FormInput
            {...props}
            key="priority"
            label="Priority"
            placeholder="Enter priority"
            type="number"
            {readonly}
          />
          <FormSelect
            {...props}
            {readonly}
            key="eviction_policy"
            label="Eviction Policy"
            single
            data={[
              { value: EvictionPolicy.OldestFirst, label: 'Oldest First' },
              {
                value: EvictionPolicy.LeastRecentlyUsed,
                label: 'Least Recently Used'
              },
              {
                value: EvictionPolicy.LeastFrequentlyUsed,
                label: 'Least Frequently Used'
              }
            ]}
          />
          <Label class="mb-2">Downstream Caches</Label>
          <TagsInput
            placeholder="Add a downstream cache"
            bind:value={downstreamCaches}
            suggestions={[
              'https://cache.nixos.org/',
              'https://nix-community.cachix.org/',
              'https://cache.garnix.io/'
            ]}
            validate={(val) => {
              let res = z.url().safeParse(val);
              if (!res.success) return undefined;
              return val;
            }}
          />
          <FormSwitch
            disabled={readonly || props.disabled}
            formData={props.formData}
            key="allow_force_push"
            label="Allow Force Push"
          />
        </div>
      </div>
    {/snippet}
    {#snippet footer({ isLoading }: { isLoading: boolean })}
      <div class="mt-4 grid w-full grid-cols-1 gap-8 xl:grid-cols-2">
        <Button
          class="ml-auto cursor-pointer"
          type="submit"
          disabled={isLoading || readonly}
        >
          {#if isLoading}
            <Spinner />
          {:else}
            <Save />
          {/if}
          Save Changes</Button
        >
      </div>
    {/snippet}
  </BaseForm>
  <p class="text-destructive mt-8">Danger Zone</p>
  <div class="grid grid-cols-1 gap-4 xl:grid-cols-[1fr_auto_1fr]">
    <div
      class="border-destructive/50 bg-destructive/10 mt-4 flex w-full flex-col items-start gap-4 rounded-md border p-4"
    >
      <div class="flex w-full items-center gap-4">
        <p>Change cache visibility</p>
        <Button
          class="ml-auto cursor-pointer"
          variant="destructive"
          onclick={() => (visibilityOpen = true)}
          disabled={readonly}
        >
          {#if data.cacheInfo.public}
            <Lock />
            Make Private
          {:else}
            <LockOpen />
            Make Public
          {/if}
        </Button>
      </div>
      <div class="flex w-full items-center gap-4">
        <p>Change the cache quota</p>
        <Button
          class="ml-auto cursor-pointer"
          variant="destructive"
          onclick={() => (quotaOpen = true)}
          disabled={readonly}
        >
          <HardDrive />
          Change Quota
        </Button>
      </div>
      <div class="flex w-full items-center gap-4">
        <p>Change public signing key</p>
        <Button
          class="ml-auto cursor-pointer"
          variant="destructive"
          onclick={() => (keyOpen = true)}
          disabled={readonly}
        >
          <KeyRound />
          Change Key
        </Button>
      </div>
      <div class="flex w-full items-center gap-4">
        <p>Clear all data from the cache.</p>
        <Button
          class="ml-auto cursor-pointer"
          variant="destructive"
          onclick={() => (clearOpen = true)}
          disabled={readonly}
        >
          <Ban />
          Clear Cache
        </Button>
      </div>
      <div class="flex w-full items-center gap-4">
        <p>Delete the cache and all its data</p>
        <Button
          class="ml-auto cursor-pointer"
          variant="destructive"
          onclick={() => (deleteOpen = true)}
          disabled={readonly}
        >
          <Trash />
          Delete Cache
        </Button>
      </div>
    </div>
  </div>
</div>
<FormDialog
  title={`Change Cache Visibility`}
  description={`Do you really want to make the cache ${data.cacheInfo.name} ${data.cacheInfo.public ? 'private' : 'public'}? This might have implications on who can access the cache and its data.`}
  confirm={data.cacheInfo.public ? 'Make Private' : 'Make Public'}
  confirmVariant="destructive"
  onsubmit={visibilityConfirm}
  bind:open={visibilityOpen}
  schema={z.object({})}
/>
<FormDialog
  title={`Change Quota`}
  description={`Do you really want to change the quota of cache ${data.cacheInfo.name}? This can lead to the deletion of cache entries if the new quota is smaller than the current usage.`}
  confirm="Change"
  confirmVariant="destructive"
  initialValue={{ quota: data.cacheInfo.quota / (1024 * 1024) }}
  onsubmit={quotaConfirm}
  bind:open={quotaOpen}
  schema={quotaSchema}
>
  {#snippet children({ props })}
    <FormInput
      {...props}
      key="quota"
      label="Quota (in MiB)"
      placeholder="Enter new cache quota in MiB"
      type="number"
    />
  {/snippet}
</FormDialog>
<FormDialog
  title={`Change Key`}
  description={`Do you really want to change the key of cache ${data.cacheInfo.name}? This prevents all pushes using the old key.`}
  confirm="Change"
  confirmVariant="destructive"
  onsubmit={keyConfirm}
  bind:open={keyOpen}
  schema={keySchema}
>
  {#snippet children({ props })}
    <FormInput
      {...props}
      key="sig_key"
      label="Public Signing Key"
      placeholder="Enter new public signing key"
    />
  {/snippet}
</FormDialog>
<FormDialog
  title={`Clear Cache`}
  description={`Do you really want to clear the cache ${data.cacheInfo.name}? This action cannot be undone.`}
  confirm="Clear"
  confirmVariant="destructive"
  onsubmit={clearConfirm}
  bind:open={clearOpen}
  schema={z.object({})}
/>
<FormDialog
  title={`Delete Cache`}
  description={`Do you really want to delete the cache ${data.cacheInfo.name}? This action cannot be undone.`}
  confirm="Delete"
  confirmVariant="destructive"
  onsubmit={deleteConfirm}
  bind:open={deleteOpen}
  schema={z.object({})}
/>
