import type { LayoutLoad } from './$types';
import { cacheDetails } from '$lib/client';

export const load: LayoutLoad = ({ params, fetch }) => {
  const cacheRes = cacheDetails({
    fetch,
    path: { uuid: params.uuid }
  });

  return {
    cacheRes,
    uuid: params.uuid
  };
};
