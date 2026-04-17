import type { PageLoad } from './$types';
import { redirect } from '@sveltejs/kit';
import { tokenInfo } from '$lib/client';

export const load: PageLoad = async ({ params, fetch }) => {
  const res = await tokenInfo({
    fetch,
    path: { uuid: params.uuid }
  });

  if (!res.data) {
    if (res.response.status === 404) {
      redirect(307, '/tokens?error=token_not_found');
    } else {
      redirect(307, '/tokens?error=token_other');
    }
  }

  return {
    tokenInfo: res.data,
    uuid: params.uuid
  };
};
