import { EvictionPolicy } from '$lib/client';
import { z } from 'zod';

export const cacheSchema = z.object({
  name: z
    .string()
    .min(1, 'Cache name cannot be empty')
    .max(63, 'Cache name is too long (max 63 characters)')
    .regex(
      /^(?![0-9-])[a-zA-Z0-9-]{0,62}(?<!-)$/,
      'Cache name must be lowercase, alphanumeric, and cannot start or end with a hyphen'
    )
    .default(''),
  priority: z
    .number()
    .int()
    .min(0, { message: 'Priority must be a non-negative integer' }),
  allow_force_push: z.boolean().default(false),
  eviction_policy: z
    .array(z.enum(EvictionPolicy))
    .default([EvictionPolicy.LEAST_RECENTLY_USED])
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
