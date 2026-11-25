import type { EditTokenRequest, TokenDetails, TokenRule } from '$lib/client';
import type { FormValue } from '@profidev/pleiades/components/form/types';
import { z } from 'zod';

export const tokenSettings = z.object({
  exp: z
    .date('Invalid date')
    // oxlint-disable-next-line max-nested-calls
    .default(new Date(new Date().getTime() + 30 * 24 * 60 * 60 * 1000)),
  name: z.string().min(1, 'Token name is required') // Default to 30 days
});

export const reformatData = (
  data: FormValue<typeof tokenSettings>,
  uuid: string,
  restricted: boolean,
  rules: TokenRule[]
): EditTokenRequest => ({
  exp: data.exp,
  name: data.name,
  restricted,
  rules: restricted ? rules : [],
  uuid
});

export const formatData = (
  user: TokenDetails
): FormValue<typeof tokenSettings> => ({
  exp: new Date(user.exp),
  name: user.name
});
