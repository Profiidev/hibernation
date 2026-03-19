import z from 'zod';

export const information = z.object({
  name: z.string().min(1, 'Name is required').default(''),
  public: z.boolean().default(false),
  quota: z
    .number()
    .min(1, 'Quota must be a positive number')
    .default(10 * 1024),
  sig_key: z.string().min(1, 'Signature key is required').default('')
});
