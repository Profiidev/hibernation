<script lang="ts" generics="S extends FormRecord = FormRecord">
  import * as Calendar from 'positron-components/components/ui/calendar';
  import * as Popover from 'positron-components/components/ui/popover';
  import * as Form from 'positron-components/components/ui/form';
  import { Button } from 'positron-components/components/ui/button';
  import CalendarIcon from '@lucide/svelte/icons/calendar';
  import {
    CalendarDate,
    fromDate,
    getLocalTimeZone,
    toCalendarDate,
    type DateValue
  } from '@internationalized/date';
  import type {
    FormPath,
    FormRecord,
    SuperForm
  } from 'positron-components/components/form/types';
  import { get } from 'svelte/store';

  interface Props {
    formData: SuperForm<S>;
    key: FormPath<S>;
    label: string;
    disabled?: boolean;
    placeholder?: string;
    minValue?: DateValue;
    maxValue?: DateValue;
  }

  let {
    formData: form,
    key,
    label,
    disabled,
    placeholder,
    maxValue,
    minValue
  }: Props = $props();

  let open = $state(false);
  let formData = $derived(form.form);
  let date = $derived(
    toCalendarDate(fromDate($formData[key] as Date, getLocalTimeZone()))
  );
</script>

<Form.Field {form} name={key} class="mt-2 flex w-full flex-col">
  <Form.Control>
    {#snippet children({ props })}
      <Form.Label>{label}</Form.Label>
      <Popover.Root bind:open>
        <Popover.Trigger {...props} {disabled}>
          {#snippet child({ props })}
            <Button
              {...props}
              variant="outline"
              class="cursor-pointer justify-between font-normal"
            >
              {date
                ? date
                    .toDate(getLocalTimeZone())
                    .toLocaleString(
                      navigator.languages || [navigator.language],
                      {
                        year: 'numeric',
                        month: '2-digit',
                        day: '2-digit'
                      }
                    )
                : placeholder || 'Select date'}
              <CalendarIcon />
            </Button>
          {/snippet}
        </Popover.Trigger>
        <Popover.Content class="w-auto overflow-hidden p-0" align="start">
          {/* @ts-ignore */ null}
          <Calendar.Calendar
            type="single"
            bind:value={date}
            captionLayout="dropdown"
            onValueChange={(val) => {
              formData.set({
                ...get(formData),
                [key]:
                  val instanceof CalendarDate
                    ? val.toDate(getLocalTimeZone())
                    : null
              });
              open = false;
            }}
            {minValue}
            {maxValue}
          />
        </Popover.Content>
      </Popover.Root>
    {/snippet}
  </Form.Control>
  <Form.FieldErrors />
</Form.Field>
