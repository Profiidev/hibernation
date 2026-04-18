<script lang="ts">
  import { goto } from '$app/navigation';
  import { toast } from '@profidev/pleiades/components/util/general';
  import type { Stage } from '@profidev/pleiades/components/form/types';
  import MultiStepForm from '@profidev/pleiades/components/form/multistep-form.svelte';
  import Information from './Information.svelte';
  import { createToken } from '$lib/client';

  let stages: Stage[] = [
    {
      title: 'Create Token',
      content: Information,
      data: {}
    }
  ];

  const submit = async (rawData: object) => {
    let anyData = rawData as any;
    anyData.exp = (anyData.exp as Date).toISOString();
    let res = await createToken({ body: anyData });

    if (!res.data) {
      if (res.response.status === 409) {
        return {
          error: 'A token with this name already exists.',
          field: 'name'
        };
      } else {
        return { error: 'Error creating token.' };
      }
    } else {
      toast.success('Token created successfully.');
      sessionStorage.setItem('newToken', res.data.token);
      setTimeout(() => {
        goto(`/tokens/${res.data.uuid}`);
      });
    }
  };
</script>

<MultiStepForm {stages} onsubmit={submit} cancelHref="/tokens" />
