<script lang="ts">
  import * as Card from '@profidev/pleiades/components/ui/card';
  import { onMount } from 'svelte';
  import { Input } from '@profidev/pleiades/components/ui/input';
  import { Button } from '@profidev/pleiades/components/ui/button';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import { newCode } from '$lib/client';
  import { sendCliCode } from '$lib/backend/util.svelte.js';

  type CliAuthStatus =
    | 'Requesting'
    | 'Error'
    | 'Success'
    | 'Finished'
    | 'SendError';

  let { data } = $props();

  let status = $state('Requesting' as CliAuthStatus);
  let error = $state(false);
  let code = $state('');

  const authCli = async () => {
    let res = await newCode();
    if (!res.data) {
      error = true;
      status = 'Error';
    } else {
      error = false;
      status = 'Success';
      code = res.data.code;
      let ret = await sendCliCode(res.data.code);
      if (!ret) {
        status = 'Finished';
        window.close();
      } else {
        status = 'SendError';
      }
    }
  };

  onMount(authCli);
</script>

<div class="grid h-full place-items-center">
  <Card.Root class="mx-auto w-full max-w-sm">
    <Card.Header>
      <Card.Title>CLI Auth</Card.Title>
    </Card.Header>
    <Card.Content>
      {#if status === 'Requesting'}
        <p>Requesting new CLI auth code...</p>
      {:else if status === 'Error'}
        <p class="text-red-500">Failed to get CLI auth code</p>
      {:else if status === 'Success'}
        <p>Trying to authenticate CLI with code:</p>
        <Input value={code} readonly class="my-2 w-full" />
        <p>If it is not working try to paste the code into the CLI manually.</p>
      {:else if status === 'SendError'}
        <p>Failed to send code. Please enter it manually:</p>
        <Input value={code} readonly class="my-2 w-full" />
      {:else if status === 'Finished'}
        <p>CLI authenticated successfully! You can now close this window.</p>
      {/if}
      {#if error}
        <Button variant="outline" class="mt-4 w-full" onclick={authCli}>
          <RotateCcw class="mr-2" />
          Try Again
        </Button>
      {/if}
    </Card.Content>
  </Card.Root>
</div>
