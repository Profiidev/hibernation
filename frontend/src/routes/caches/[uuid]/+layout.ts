import { RequestError } from 'positron-components/backend';
import type { LayoutLoad } from './$types';
import { redirect } from '@sveltejs/kit';
import { getCacheDetails } from '$lib/backend/cache.svelte';

export const load: LayoutLoad = async ({ params, fetch }) => {
  let res = await getCacheDetails(params.uuid, fetch);

  if (typeof res !== 'object') {
    if (res === RequestError.NotFound) {
      redirect(307, '/caches?error=cache_not_found');
    } else {
      redirect(307, '/caches?error=cache_other');
    }
  }

  return {
    uuid: params.uuid,
    cacheInfo: res
  };
};
