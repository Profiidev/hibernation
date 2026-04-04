<script lang="ts">
  import { goto } from '$app/navigation';
  import { toast } from 'positron-components/components/util/general';
  import type { Stage } from '$lib/components/form/types.svelte';
  import MultiStepForm from '$lib/components/form/MultiStepForm.svelte';
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
