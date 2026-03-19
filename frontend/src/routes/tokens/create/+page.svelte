<script lang="ts">
  import { goto } from '$app/navigation';
  import { RequestError } from 'positron-components/backend';
  import { toast } from 'positron-components/components/util/general';
  import type { Stage } from '$lib/components/form/types.svelte';
  import MultiStepForm from '$lib/components/form/MultiStepForm.svelte';
  import Information from './Information.svelte';
  import { createToken } from '$lib/backend/token.svelte';

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
    let res = await createToken(anyData);

    if (typeof res === 'string') {
      if (res === RequestError.Conflict) {
        return {
          error: 'A token with this name already exists.',
          path: 'name'
        };
      } else {
        return { error: 'Error creating token.' };
      }
    } else {
      toast.success('Token created successfully.');
      sessionStorage.setItem('newToken', res.token);
      setTimeout(() => {
        goto(`/tokens/${res.uuid}`);
      });
    }
  };
</script>

<MultiStepForm {stages} onsubmit={submit} cancelHref="/tokens" />
