import { getSetupStatus } from '$lib/backend/setup.svelte';
import { redirect } from '@sveltejs/kit';
import type { LayoutLoad } from './$types';
import { getUserInfo, type UserInfo } from '$lib/backend/user.svelte';
import { noSidebarPaths } from '$lib/components/navigation/sidebar/items.svelte';
import { RequestError } from 'positron-components/backend';

export const load: LayoutLoad = async ({ fetch, url }) => {
  let status = await getSetupStatus(fetch);

  if (!status?.is_setup && url.pathname !== '/setup') {
  }

  let user: UserInfo | RequestError | undefined = await getUserInfo(fetch);

  if (
    typeof user === 'string' &&
    user !== RequestError.Unauthorized &&
    !noSidebarPaths.includes(url.pathname)
  ) {
  }

  if (typeof user === 'string') {
    user = undefined;
  }

  return {
    user
  };
};
