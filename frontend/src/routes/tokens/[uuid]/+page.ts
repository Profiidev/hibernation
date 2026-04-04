import type { PageLoad } from './$types';
import { redirect } from '@sveltejs/kit';
import { tokenInfo } from '$lib/client';

export const load: PageLoad = async ({ params, fetch }) => {
  let res = await tokenInfo({
    path: { uuid: params.uuid },
    fetch
  });

  if (!res.data) {
    if (res.response.status === 404) {
      redirect(307, '/tokens?error=token_not_found');
    } else {
      redirect(307, '/tokens?error=token_other');
    }
  }

  return {
    uuid: params.uuid,
    tokenInfo: res.data
  };
};
