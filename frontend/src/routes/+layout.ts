import { redirect } from '@sveltejs/kit';
import type { LayoutLoad } from './$types';
import { noSidebarPaths } from '$lib/components/nav.svelte';
import { info, isSetup } from '$lib/client';

export const load: LayoutLoad = async ({ fetch, url }) => {
  let { data: status } = await isSetup({ fetch });

  if (!status?.is_setup && url.pathname !== '/setup') {
    redirect(302, '/setup');
  }

  let { data: user, response } = await info({ fetch });

  if (
    !user &&
    response.status !== 401 &&
    !noSidebarPaths.includes(url.pathname)
  ) {
    redirect(302, '/setup');
  }

  if (typeof user === 'string') {
    user = undefined;
  }

  return {
    user
  };
};
