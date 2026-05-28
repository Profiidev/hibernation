import type { PageLoad } from './$types';
import {
  listCachesSimple,
  listGroupsSimple,
  mailActive,
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
  const mailPromise = mailActive({ fetch }).then(
    (res) => res.data?.active ?? false
  );
  const cachesPromise = listCachesSimple({ fetch });

  return {
    cachesPromise,
    groupsPromise,
    mailActivePromise: mailPromise,
    userInfoPromise: resPromise,
    uuid: params.uuid
  };
};
