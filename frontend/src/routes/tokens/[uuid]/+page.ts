import type { PageLoad } from './$types';
import { listCaches, tokenInfo } from '$lib/client';

export const load: PageLoad = ({ params, fetch }) => {
  const res = tokenInfo({
    fetch,
    path: { uuid: params.uuid }
  });

  return {
    cachesRes: listCaches({ fetch }),
    tokenRes: res,
    uuid: params.uuid
  };
};
