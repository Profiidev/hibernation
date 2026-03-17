<script lang="ts">
  import { Separator } from 'positron-components/components/ui/separator';
  import { Button } from 'positron-components/components/ui/button';
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import Trash from '@lucide/svelte/icons/trash';
  import FormDialog from 'positron-components/components/form/form-dialog.svelte';
  import { z } from 'zod';
  import { toast } from 'positron-components/components/util/general';
  import { goto } from '$app/navigation';
  import BaseForm from 'positron-components/components/form/base-form.svelte';
  import { formatData, tokenSettings, reformatData } from './schema.svelte.js';
  import type { FormValue } from 'positron-components/components/form/types';
  import { RequestError } from 'positron-components/backend';
  import FormInput from 'positron-components/components/form/form-input.svelte';
  import Save from '@lucide/svelte/icons/save';
  import { Spinner } from 'positron-components/components/ui/spinner';
  import {
    deleteToken,
    editToken,
    regenerateToken
  } from '$lib/backend/token.svelte.js';
  import FormDateInput from '$lib/components/form/FormDateInput.svelte';
  import { Input } from 'positron-components/components/ui/input';
  import { Label } from 'positron-components/components/ui/label';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import { CopyButton } from 'positron-components/components/ui-extra/copy-button';

  const { data } = $props();

  let deleteOpen = $state(false);
  let regenerateOpen = $state(false);
  let isLoading = $state(false);
  let token = $state<string>();

  const deleteItemConfirm = async () => {
    isLoading = true;
    let ret = await deleteToken({ uuid: data.tokenInfo.uuid });
    isLoading = false;

    if (ret) {
      return { error: 'Failed to delete token' };
    } else {
      toast.success(`Token ${data.tokenInfo.name} deleted successfully`);
      setTimeout(() => {
        goto('/tokens');
      });
    }
  };

  const regenerateConfirm = async () => {
    isLoading = true;
    let res = await regenerateToken(data.tokenInfo.uuid);
    isLoading = false;

    if (typeof res !== 'object') {
      return { error: 'Failed to regenerate token' };
    } else {
      toast.success(`Token ${data.tokenInfo.name} regenerated successfully`);
      token = res.token;
    }
  };

  const onsubmit = async (form: FormValue<typeof tokenSettings>) => {
    let token = reformatData(form, data.tokenInfo.uuid);
    let res = await editToken(token);

    if (res) {
      if (res === RequestError.Conflict) {
        return { error: 'This token name is already in use', field: 'name' };
      } else {
        return { error: 'Failed to update token' };
      }
    } else {
      toast.success(`Token ${data.tokenInfo.name} updated successfully`);
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
    <h3 class="text-xl font-medium">Token: {data.tokenInfo.name}</h3>
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
      <BaseForm
        schema={tokenSettings}
        {onsubmit}
        initialValue={formatData(data.tokenInfo)}
      >
        {#snippet children({ props })}
          <div class="grid grid-cols-1 gap-4 lg:grid-cols-[1fr_auto_1fr]">
            <div>
              <FormInput
                {...props}
                key="name"
                label="Token Name"
                placeholder="Enter name"
              />
              <FormDateInput
                {...props}
                key="exp"
                label="Expiration Date"
                placeholder="Enter date"
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
                >
                  <RotateCcw />
                  Regenerate
                </Button>
              </div>
            </div>
          </div>
        {/snippet}
        {#snippet footer({ isLoading }: { isLoading: boolean })}
          <div class="mt-4 grid w-full grid-cols-1 gap-8 lg:grid-cols-2">
            <Button
              class="ml-auto cursor-pointer"
              type="submit"
              disabled={isLoading}
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
    </div>
  </div>
</div>
<FormDialog
  title={`Delete Token`}
  description={`Do you really want to delete the token ${data.tokenInfo.name}?`}
  confirm="Delete"
  confirmVariant="destructive"
  onsubmit={deleteItemConfirm}
  bind:open={deleteOpen}
  bind:isLoading
  schema={z.object({})}
/>
<FormDialog
  title={`Regenerate Token`}
  description={`Do you really want to regenerate the token ${data.tokenInfo.name}?`}
  confirm="Regenerate"
  confirmVariant="destructive"
  onsubmit={regenerateConfirm}
  bind:open={regenerateOpen}
  bind:isLoading
  schema={z.object({})}
/>
