import z from 'zod';

export const information = z.object({
  name: z.string().min(1, 'Name is required').default(''),
  exp: z
    .date('Invalid date')
    .default(new Date(new Date().getTime() + 30 * 24 * 60 * 60 * 1000)) // Default to 30 days
});
