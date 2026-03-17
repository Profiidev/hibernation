import {
  delete_,
  get,
  post,
  put,
  ResponseType
} from 'positron-components/backend';

export interface TokenInfo {
  uuid: string;
  name: string;
  exp: string;
  last_used?: string;
}

export const listTokens = async (fetch: typeof window.fetch = window.fetch) => {
  let ret = await get<TokenInfo[]>('/api/token', {
    res_type: ResponseType.Json,
    fetch
  });

  if (Array.isArray(ret)) {
    return ret;
  }
};

export const getTokenInfo = async (
  uuid: string,
  fetch: typeof window.fetch = window.fetch
) => {
  let ret = await get<TokenInfo>(`/api/token/${uuid}`, {
    res_type: ResponseType.Json,
    fetch
  });

  if (ret && typeof ret === 'object') {
    return ret;
  }
};

export interface TokenCreateRequest {
  name: string;
  exp: string;
}

export interface TokenCreateResponse {
  uuid: string;
  token: string;
}

export const createToken = async (data: TokenCreateRequest) => {
  return await post<TokenCreateResponse>('/api/token', {
    body: data,
    res_type: ResponseType.Json
  });
};

export interface TokenDeleteRequest {
  uuid: string;
}

export const deleteToken = async (data: TokenDeleteRequest) => {
  return await delete_('/api/token', {
    body: data
  });
};

export interface TokenEditRequest {
  uuid: string;
  name: string;
  exp: string;
}

export const editToken = async (data: TokenEditRequest) => {
  return await put<void>('/api/token', {
    body: data
  });
};
