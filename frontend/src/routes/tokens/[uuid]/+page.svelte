<script lang="ts">
  import { Separator } from '@profidev/pleiades/components/ui/separator';
  import { Button } from '@profidev/pleiades/components/ui/button';
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import Trash from '@lucide/svelte/icons/trash';
  import FormDialog from '@profidev/pleiades/components/form/form-dialog.svelte';
  import { z } from 'zod';
  import { toast } from '@profidev/pleiades/components/util/general';
  import { goto } from '$app/navigation';
  import BaseForm from '@profidev/pleiades/components/form/base-form.svelte';
  import { formatData, tokenSettings, reformatData } from './schema.svelte.js';
  import type { FormValue } from '@profidev/pleiades/components/form/types';
  import FormInput from '@profidev/pleiades/components/form/form-input.svelte';
  import Save from '@lucide/svelte/icons/save';
  import { Spinner } from '@profidev/pleiades/components/ui/spinner';
  import FormDateInput from '@profidev/pleiades/components/form/form-date-input.svelte';
  import { Input } from '@profidev/pleiades/components/ui/input';
  import { Label } from '@profidev/pleiades/components/ui/label';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import { CopyButton } from '@profidev/pleiades/components/ui-extra/copy-button';
  import { onMount } from 'svelte';
  import { today, getLocalTimeZone } from '@internationalized/date';
  import {
    deleteToken,
    editToken,
    tokenRegenerate,
    type TokenInfo
  } from '$lib/client';
  import { Skeleton } from '@profidev/pleiades/components/ui/skeleton';

  const { data } = $props();

  let deleteOpen = $state(false);
  let regenerateOpen = $state(false);
  let isLoading = $state(false);
  let token = $state<string>();
  let tokenInfo: TokenInfo | undefined = $state();
  let form: BaseForm<typeof tokenSettings> | undefined = $state();
  let readonly = $derived(!tokenInfo);

  onMount(() => {
    let newToken = sessionStorage.getItem('newToken');
    if (newToken) {
      token = newToken;
      sessionStorage.removeItem('newToken');
    }
  });

  $effect(() => {
    data.tokenRes.then((res) => {
      if (!res.data) {
        if (res.response?.status === 404) {
          goto('/tokens?error=not_found');
        } else {
          goto('/tokens?error=other');
        }
        return;
      }

      tokenInfo = res.data;
      form?.setValue(formatData(tokenInfo));
    });
  });

  const deleteItemConfirm = async () => {
    if (!tokenInfo) return;
    isLoading = true;
    let ret = await deleteToken({ body: { uuid: tokenInfo.uuid } });
    isLoading = false;

    if (ret.error) {
      return { error: 'Failed to delete token' };
    } else {
      toast.success(`Token ${tokenInfo.name} deleted successfully`);
      setTimeout(() => {
        goto('/tokens');
      });
    }
  };

  const regenerateConfirm = async () => {
    if (!tokenInfo) return;
    isLoading = true;
    let res = await tokenRegenerate({
      path: { uuid: tokenInfo.uuid }
    });
    isLoading = false;

    if (!res.data) {
      return { error: 'Failed to regenerate token' };
    } else {
      toast.success(`Token ${tokenInfo.name} regenerated successfully`);
      token = res.data.token;
    }
  };

  const onsubmit = async (form: FormValue<typeof tokenSettings>) => {
    if (!tokenInfo) return;
    let newToken = reformatData(form, tokenInfo.uuid);
    let res = await editToken({ body: newToken });

    if (res.error) {
      if (res.response?.status === 409) {
        return {
          error: 'This token name is already in use',
          field: 'name'
        } as const;
      } else {
        return { error: 'Failed to update token' };
      }
    } else {
      toast.success(`Token ${tokenInfo.name} updated successfully`);
      // do not trigger form reset
      return { error: '' };
    }
  };
</script>

<div class="flex h-full w-full flex-col space-y-6 p-4">
  <div class="mt-1! mb-0 ml-7 flex items-center md:m-0">
    <Button size="icon" variant="ghost" href="/tokens" class="mr-2">
      <ArrowLeft class="size-5" />
    </Button>
    <h3 class="flex text-xl font-medium">
      Token: {#if !tokenInfo}
        <Skeleton class="ml-2 h-7 w-20" />
      {:else}
        {tokenInfo.name}
      {/if}
    </h3>
    <Button
      class="ml-auto cursor-pointer"
      onclick={() => (deleteOpen = true)}
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
      <BaseForm schema={tokenSettings} {onsubmit} bind:this={form}>
        {#snippet children({ props })}
          <div class="grid grid-cols-1 gap-4 lg:grid-cols-[1fr_auto_1fr]">
            <div>
              <FormInput
                {...props}
                key="name"
                label="Token Name"
                placeholder="Enter name"
                disabled={readonly}
              />
              <FormDateInput
                {...props}
                key="exp"
                label="Expiration Date"
                placeholder="Enter date"
                minValue={today(getLocalTimeZone())}
                disabled={readonly}
              />
              <Label
                >Token
                {#if token}
                  <span class="text-destructive">
                    (Can not be viewed again!)
                  </span>
                {/if}
              </Label>
              <div class="mt-2 flex gap-2">
                {#if token}
                  <CopyButton
                    text={token}
                    variant="outline"
                    class="grow justify-start"
                  >
                    <span class="truncate">{token}</span>
                  </CopyButton>
                {:else}
                  <Input
                    value="Can not be viewed."
                    readonly
                    class="text-destructive"
                  />
                {/if}
                <Button
                  variant="destructive"
                  class="cursor-pointer"
                  onclick={() => (regenerateOpen = true)}
                  disabled={readonly}
                >
                  <RotateCcw />
                  Regenerate
                </Button>
              </div>
            </div>
          </div>
        {/snippet}
        {#snippet footer({
          isLoading,
          isError
        }: {
          isLoading: boolean;
          isError: boolean;
        })}
          <div class="mt-4 grid w-full grid-cols-1 gap-8 lg:grid-cols-2">
            <Button
              class="ml-auto cursor-pointer"
              type="submit"
              disabled={isLoading || readonly}
              variant={isError ? 'destructive' : undefined}
            >
              {#if isLoading}
                <Spinner />
              {:else if isError}
                <RotateCcw />
              {:else}
                <Save />
              {/if}
              {isError ? 'Retry' : 'Save Changes'}</Button
            >
          </div>
        {/snippet}
      </BaseForm>
    </div>
  </div>
</div>
<FormDialog
  title={`Delete Token`}
  description={`Do you really want to delete the token ${tokenInfo?.name}?`}
  confirm="Delete"
  confirmVariant="destructive"
  onsubmit={deleteItemConfirm}
  bind:open={deleteOpen}
  bind:isLoading
  schema={z.object({})}
/>
<FormDialog
  title={`Regenerate Token`}
  description={`Do you really want to regenerate the token ${tokenInfo?.name}?`}
  confirm="Regenerate"
  confirmVariant="destructive"
  onsubmit={regenerateConfirm}
  bind:open={regenerateOpen}
  bind:isLoading
  schema={z.object({})}
/>
