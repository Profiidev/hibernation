<script lang="ts">
  import BaseForm from 'positron-components/components/form/base-form.svelte';
  import { information } from './schema.svelte';
  import FormInput from 'positron-components/components/form/form-input.svelte';
  import type { StageProps } from 'positron-components/components/form/types';
  import FormDateInput from 'positron-components/components/form/form-date-input.svelte';
  import { today, getLocalTimeZone } from '@internationalized/date';

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
      label="Token Name"
      placeholder="Enter name"
    />
    <FormDateInput
      {...props}
      key="exp"
      label="Expiration Date"
      placeholder="Enter date"
      minValue={today(getLocalTimeZone())}
    />
  {/snippet}
</BaseForm>
