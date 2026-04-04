<script lang="ts">
  import { ModeWatcher } from 'positron-components/components/util/general';
  import { Toaster } from 'positron-components/components/ui/sonner';
  import '../app.css';
  import { connectWebsocket } from '$lib/backend/updater.svelte';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import Sidebar from '$lib/components/navigation/sidebar/Sidebar.svelte';
  import { page } from '$app/state';
  import { noSidebarPaths } from '$lib/components/navigation/sidebar/items.svelte';
  import { setMode } from 'mode-watcher';
  import { testToken } from '$lib/client';

  let { children, data } = $props();

  onMount(() => {
    setMode('dark');
    testToken().then(({ data: dataRaw }) => {
      let { valid } = (dataRaw as { valid: boolean } | undefined) ?? {
        valid: false
      };
      // can also be undefined if there was an error
      if (valid === false) {
        if (!noSidebarPaths.includes(page.url.pathname)) {
          goto('/login');
        }
      } else {
        connectWebsocket(data.user?.uuid ?? '');
      }
    });
  });
</script>

<ModeWatcher />
<Toaster position="top-right" closeButton={true} richColors={true} />

{#if noSidebarPaths.includes(page.url.pathname)}
  {@render children()}
{:else}
  <Sidebar
    user={data.user ?? {
      email: '',
      name: '',
      permissions: [],
      uuid: ''
    }}
  >
    {@render children()}
  </Sidebar>
{/if}
