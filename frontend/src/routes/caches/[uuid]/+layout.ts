import type { LayoutLoad } from './$types';
import { redirect } from '@sveltejs/kit';
import { cacheDetails } from '$lib/client';

export const load: LayoutLoad = async ({ params, fetch }) => {
  let { data, response } = await cacheDetails({
    path: { uuid: params.uuid },
    fetch
  });

  if (!data) {
    if (response.status === 404) {
      redirect(307, '/caches?error=cache_not_found');
    } else {
      redirect(307, '/caches?error=cache_other');
    }
  }

  return {
    uuid: params.uuid,
    cacheInfo: data
  };
};
