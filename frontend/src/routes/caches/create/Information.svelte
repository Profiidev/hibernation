<script lang="ts">
  import BaseForm from '@profidev/pleiades/components/form/base-form.svelte';
  import { information } from './schema.svelte';
  import FormInput from '@profidev/pleiades/components/form/form-input.svelte';
  import type { StageProps } from '@profidev/pleiades/components/form/types';
  import FormSwitch from '@profidev/pleiades/components/form/form-switch.svelte';

  let { initialValue, onsubmit, footer, isLoading }: StageProps = $props();

  let form: BaseForm<typeof information> | undefined = $state();

  export const getValue = () => {
    return form?.getValue();
  };
</script>

<BaseForm
  schema={information}
  {onsubmit}
  {footer}
  {initialValue}
  bind:this={form}
  bind:isLoading
>
  {#snippet children({ props })}
    <FormInput
      {...props}
      key="name"
      label="Cache Name"
      placeholder="Cache name"
    />
    <FormInput
      {...props}
      key="quota"
      label="Quota (MiB)"
      placeholder={`${10 * 1024}`}
      type="number"
    />
    <FormInput
      {...props}
      key="sig_key"
      label="Signing Key"
      placeholder="cache.example.com-1:mB9FSh9qf2..."
    />
    <FormSwitch {...props} key="public" label="Public Cache" />
  {/snippet}
</BaseForm>
