import { listCaches } from '$lib/client';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, url }) => {
  const caches = await listCaches({ fetch });

  return {
    caches: caches.data,
    error: url.searchParams.get('error')
  };
};
