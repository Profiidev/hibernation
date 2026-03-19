import { RequestError } from 'positron-components/backend';
import type { PageLoad } from './$types';
import { redirect } from '@sveltejs/kit';
import { getCacheInfo } from '$lib/backend/cache.svelte';

export const load: PageLoad = async ({ params, fetch }) => {
  let res = await getCacheInfo(params.uuid, fetch);

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
