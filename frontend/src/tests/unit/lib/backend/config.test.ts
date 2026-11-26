import { afterEach, describe, expect, it, vi } from 'vitest';

const load = async (browser: boolean) => {
  vi.resetModules();
  vi.doMock('$app/env', () => ({ browser }));
  return (await import('#lib/backend/config.ts')).createClientConfig;
};

afterEach(() => vi.doUnmock('$app/env'));

describe('createClientConfig', () => {
  it('keeps relative urls in the browser', async () => {
    const create = await load(true);
    const result = create({ headers: { 'x-test': '1' } });
    expect(result.headers).toEqual({ 'x-test': '1' });
    expect(result.baseUrl).toBeUndefined();
  });

  it('pins a placeholder origin during SSR so Request() accepts the url', async () => {
    const create = await load(false);
    expect(create({}).baseUrl).toBe('http://localhost:12356');
  });
});
