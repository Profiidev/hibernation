import { get, post, ResponseType } from 'positron-components/backend';

export interface CodeResponse {
  code: string;
}

export const newCliCode = async () => {
  return await post<CodeResponse>('/api/cli', {
    res_type: ResponseType.Json
  });
};

export const sendCliCode = async (code: string, user: string) => {
  return await get('http://localhost:16401?code=' + code);
};
