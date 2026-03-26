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

export enum EvictionPolicy {
  OldestFirst = 'OldestFirst',
  LeastRecentlyUsed = 'LeastRecentlyUsed',
  LeastFrequentlyUsed = 'LeastFrequentlyUsed'
}

export type CacheDetails = CacheInfo & {
  sig_key: string;
  priority: number;
  nar_count: number;
  allow_force_push: boolean;
  eviction_policy: EvictionPolicy;
  has_write_access: boolean;
  downstream_caches: string[];
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

export enum SearchSort {
  StorePath = 'StorePath',
  Size = 'Size',
  Created = 'Created',
  Accessed = 'Accessed',
  AccessCount = 'AccessCount'
}

export enum SearchOrder {
  Ascending = 'Asc',
  Descending = 'Desc'
}

export interface SearchResult {
  store_path: string;
  size: number;
  created_at: string;
  last_accessed_at: string;
  accessed: number;
}

export const searchCache = async (
  cache: string,
  query: string,
  sort: SearchSort,
  order: SearchOrder
) => {
  if (!cache || !query) {
    return [];
  }

  let ret = await post<SearchResult[]>(
    `/api/cache/management/${cache}/search`,
    {
      body: { query, sort, order },
      res_type: ResponseType.Json
    }
  );

  if (ret && Array.isArray(ret)) {
    return ret;
  }
};

export interface CacheEdit {
  name: string;
  public: boolean;
  quota: number;
  sig_key: string;
  priority: number;
  allow_force_push: boolean;
  eviction_policy: EvictionPolicy;
  downstream_caches: string[];
}

export const editCache = async (uuid: string, data: CacheEdit) => {
  return await post(`/api/cache/management/${uuid}`, {
    body: data
  });
};

export const clearCache = async (uuid: string) => {
  return await delete_(`/api/cache/management/${uuid}`);
};

export const deletePath = async (cache: string, store_path: string) => {
  return await delete_(`/api/cache/management/${cache}/path`, {
    body: { store_path }
  });
};
