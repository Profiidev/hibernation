import type { TokenEditRequest, TokenInfo } from '$lib/backend/token.svelte';
import type { FormValue } from 'positron-components/components/form/types';
import { z } from 'zod';

export const tokenSettings = z.object({
  name: z.string().min(1, 'Token name is required'),
  exp: z
    .date('Invalid date')
    .default(new Date(new Date().getTime() + 30 * 24 * 60 * 60 * 1000)) // Default to 30 days
});

export const reformatData = (
  data: FormValue<typeof tokenSettings>,
  uuid: string
): TokenEditRequest => {
  return {
    uuid,
    name: data.name,
    exp: data.exp.toISOString()
  };
};

export const formatData = (
  user: TokenInfo
): FormValue<typeof tokenSettings> => {
  return {
    name: user.name,
    exp: new Date(user.exp)
  };
};
