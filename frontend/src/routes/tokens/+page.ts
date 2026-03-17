import { listTokens } from '$lib/backend/token.svelte';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, url }) => {
  let tokens = await listTokens(fetch);
  return {
    error: url.searchParams.get('error'),
    tokens
  };
};
