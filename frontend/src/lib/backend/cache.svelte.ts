import { delete_, get, post, ResponseType } from 'positron-components/backend';

export interface CacheInfo {
  uuid: string;
  name: string;
  size: number;
  quota: number;
  public: boolean;
}

export const listCaches = async (fetch: typeof window.fetch = window.fetch) => {
  let ret = await get<CacheInfo[]>('/api/cache/management', {
    res_type: ResponseType.Json,
    fetch
  });

  if (ret && Array.isArray(ret)) {
    return ret;
  }
};

export type CacheDetails = CacheInfo & {
  sig_key: string;
  priority: number;
  nar_count: number;
};

export const getCacheDetails = async (
  uuid: string,
  fetch: typeof window.fetch = window.fetch
) => {
  let ret = await get<CacheDetails>(`/api/cache/management/${uuid}`, {
    res_type: ResponseType.Json,
    fetch
  });

  if (ret && typeof ret === 'object') {
    return ret;
  }
};

export const size_to_gib = (size: number) => {
  return size / 1024 / 1024 / 1024;
};

export interface CreateCacheRequest {
  name: string;
  public: boolean;
  quota: number;
  sig_key: string;
}

export interface CreateCacheResponse {
  uuid: string;
}

export const createCache = async (data: CreateCacheRequest) => {
  return await post<CreateCacheResponse>('/api/cache/management', {
    body: data,
    res_type: ResponseType.Json
  });
};

export interface DeleteCacheRequest {
  uuid: string;
}

export const deleteCache = async (data: DeleteCacheRequest) => {
  return await delete_(`/api/cache/management`, {
    body: data
  });
};
