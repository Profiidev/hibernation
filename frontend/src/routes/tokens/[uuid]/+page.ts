import { RequestError } from 'positron-components/backend';
import type { PageLoad } from './$types';
import { redirect } from '@sveltejs/kit';
import { getTokenInfo } from '$lib/backend/token.svelte';

export const load: PageLoad = async ({ params, fetch }) => {
  let res = await getTokenInfo(params.uuid, fetch);

  if (typeof res !== 'object') {
    if (res === RequestError.NotFound) {
      redirect(307, '/tokens?error=token_not_found');
    } else {
      redirect(307, '/tokens?error=token_other');
    }
  }

  return {
    uuid: params.uuid,
    tokenInfo: res
  };
};
