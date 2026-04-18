import type { LayoutLoad } from './$types';
import { redirect } from '@sveltejs/kit';
import { cacheDetails } from '$lib/client';

export const load: LayoutLoad = async ({ params, fetch }) => {
  const { data, response } = await cacheDetails({
    fetch,
    path: { uuid: params.uuid }
  });

  if (!data) {
    if (response.status === 404) {
      redirect(307, '/caches?error=cache_not_found');
    } else {
      redirect(307, '/caches?error=cache_other');
    }
  }

  return {
    cacheInfo: data,
    uuid: params.uuid
  };
};
