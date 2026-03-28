import z from 'zod';

export const information = z.object({
  name: z
    .string()
    .min(1, 'Cache name cannot be empty')
    .max(63, 'Cache name is too long (max 63 characters)')
    .regex(
      /^(?![0-9-])[a-zA-Z0-9-]{0,62}(?<!-)$/,
      'Cache name must be lowercase, alphanumeric, and cannot start or end with a hyphen'
    )
    .default(''),
  public: z.boolean().default(false),
  quota: z
    .number()
    .min(1, 'Quota must be a positive number')
    .default(10 * 1024),
  sig_key: z.string().min(1, 'Signature key is required').default('')
});
