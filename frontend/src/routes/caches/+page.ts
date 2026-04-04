import { listCaches } from '$lib/client';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, url }) => {
  let caches = await listCaches({ fetch });

  return {
    error: url.searchParams.get('error'),
    caches: caches.data
  };
};
