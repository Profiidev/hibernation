import { get, post, ResponseType } from 'positron-components/backend';

export interface CacheInfo {
  id: string;
  name: string;
  size: number;
  quota: number;
  public: boolean;
}

export const listCaches = async (fetch: typeof window.fetch = window.fetch) => {
  let ret = await get<CacheInfo[]>('/api/cache', {
    res_type: ResponseType.Json,
    fetch
  });

  if (ret && Array.isArray(ret)) {
    return ret;
  }
};

export const getCacheInfo = async (
  uuid: string,
  fetch: typeof window.fetch = window.fetch
) => {
  let ret = await get<CacheInfo>(`/api/cache/${uuid}`, {
    res_type: ResponseType.Json,
    fetch
  });

  if (ret && typeof ret === 'object') {
    return ret;
  }
};

export const size_to_gib = (size: number) => {
  return size / 1024;
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
  return await post<CreateCacheResponse>('/api/cache', {
    body: data,
    res_type: ResponseType.Json
  });
};
