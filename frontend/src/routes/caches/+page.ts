import { listCaches } from '$lib/client';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, url }) => {
  const caches = listCaches({ fetch }).then(({ data }) => data);

  return {
    caches,
    error: url.searchParams.get('error')
  };
};
