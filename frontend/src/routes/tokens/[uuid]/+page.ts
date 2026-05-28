import type { PageLoad } from './$types';
import { tokenInfo } from '$lib/client';

export const load: PageLoad = ({ params, fetch }) => {
  const res = tokenInfo({
    fetch,
    path: { uuid: params.uuid }
  });

  return {
    tokenRes: res,
    uuid: params.uuid
  };
};
