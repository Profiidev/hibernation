import { EvictionPolicy } from '$lib/backend/cache.svelte';
import { z } from 'zod';

export const cacheSchema = z.object({
  name: z.string().min(1, { message: 'Name is required' }),
  priority: z
    .number()
    .int()
    .min(0, { message: 'Priority must be a non-negative integer' }),
  allow_force_push: z.boolean().default(false),
  eviction_policy: z
    .array(z.enum(EvictionPolicy))
    .default([EvictionPolicy.LeastRecentlyUsed])
});

export const quotaSchema = z.object({
  quota: z
    .number()
    .min(1, 'Quota must be a positive number')
    .default(10 * 1024)
});

export const keySchema = z.object({
  sig_key: z.string().min(1, 'Signature key is required').default('')
});
