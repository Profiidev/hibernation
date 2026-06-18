import type { PageLoad } from './$types';
import {
  listCachesSimple,
  listGroupsSimple,
  userInfoDetail
} from '$lib/client';

export const load: PageLoad = async ({ params, fetch }) => {
  const resPromise = userInfoDetail({
    fetch,
    path: { uuid: params.uuid }
  });
  const groupsPromise = listGroupsSimple({
    fetch
  }).then((res) => res.data ?? []);
  const cachesPromise = listCachesSimple({ fetch });

  return {
    cachesPromise,
    groupsPromise,
    userInfoPromise: resPromise,
    uuid: params.uuid
  };
};
