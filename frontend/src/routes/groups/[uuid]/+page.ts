import type { PageLoad } from './$types';
import { groupInfo, listCachesSimpleGroup, listUsersSimple } from '$lib/client';

export const load: PageLoad = async ({ params, fetch }) => {
  const resPromise = groupInfo({
    fetch,
    path: { uuid: params.uuid }
  });
  const usersPromise = listUsersSimple({ fetch });
  const cachesPromise = listCachesSimpleGroup({ fetch });

  return {
    cachesPromise,
    groupRes: resPromise,
    usersPromise,
    uuid: params.uuid
  };
};
