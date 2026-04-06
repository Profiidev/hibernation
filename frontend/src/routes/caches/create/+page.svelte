<script lang="ts">
  import { goto } from '$app/navigation';
  import { toast } from 'positron-components/components/util/general';
  import type { Stage } from 'positron-components/components/form/types';
  import MultiStepForm from 'positron-components/components/form/multistep-form.svelte';
  import Information from './Information.svelte';
  import { createCache } from '$lib/client';

  let stages: Stage[] = [
    {
      title: 'Create Cache',
      content: Information,
      data: {}
    }
  ];

  const submit = async (rawData: object) => {
    let anyData = rawData as any;
    let res = await createCache({ body: anyData });

    if (!res.data) {
      if (res.response.status === 409) {
        return {
          error: 'A cache with this name already exists.',
          field: 'name'
        };
      } else if (res.response.status === 406) {
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
        goto(`/caches/${res.data.uuid}`);
      });
    }
  };
</script>

<MultiStepForm {stages} onsubmit={submit} cancelHref="/caches" />
