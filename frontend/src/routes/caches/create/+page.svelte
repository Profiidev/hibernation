<script lang="ts">
  import { goto } from '$app/navigation';
  import { RequestError } from 'positron-components/backend';
  import { toast } from 'positron-components/components/util/general';
  import type { Stage } from '$lib/components/form/types.svelte';
  import MultiStepForm from '$lib/components/form/MultiStepForm.svelte';
  import Information from './Information.svelte';
  import { createCache } from '$lib/backend/cache.svelte';

  let stages: Stage[] = [
    {
      title: 'Create Cache',
      content: Information,
      data: {}
    }
  ];

  const submit = async (rawData: object) => {
    let anyData = rawData as any;
    let res = await createCache(anyData);

    if (typeof res === 'string') {
      if (res === RequestError.Conflict) {
        return {
          error: 'A cache with this name already exists.',
          field: 'name'
        };
      } else if (res === RequestError.NotAcceptable) {
        return {
          error: 'Invalid signature key format.',
          field: 'sig_key'
        };
      } else {
        return { error: 'Error creating cache.' };
      }
    } else {
      toast.success('Cache created successfully.');
      setTimeout(() => {
        goto(`/caches/${res.uuid}`);
      });
    }
  };
</script>

<MultiStepForm {stages} onsubmit={submit} cancelHref="/caches" />
