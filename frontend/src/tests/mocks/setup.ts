import { beforeAll, vi } from 'vitest';
import { client as appClient } from '#lib/client/client.gen.ts';

// In the browser the client uses relative `/api/...` URLs, but Node's `fetch`
// (used under jsdom) rejects relative URLs, so requests must be absolute. Pin a
// Test origin: the generated MSW handlers read this same `baseUrl`, so the
// Request URL and the handler pattern stay aligned.
const TEST_BASE_URL = 'http://localhost';

// Start the MSW server once for the whole run. `onUnhandledRequest: 'error'`
// Makes any un-mocked `/api/...` call fail the test loudly; non-API requests
// (e.g. assets) are let through.
beforeAll(() => {
  appClient.setConfig({ ...appClient.getConfig(), baseUrl: TEST_BASE_URL });
});

// Superforms (SPA mode) finishes a submit with `applyAction(result)`, which in
// A real app writes the result into the page; superforms reads `$app/state` to
// Apply returned errors. Make `applyAction` feed the mocked page state, so
// Field errors reach the DOM.
vi.mock('$app/forms', async (importOriginal) => {
  const state = await import('../setup/app-state.svelte');
  return {
    ...(await importOriginal<object>()),
    applyAction: vi.fn(
      async (result: { type: string; status?: number; data?: unknown }) => {
        if (result.type === 'success' || result.type === 'failure') {
          state.page.form = result.data;
          state.page.status = result.status ?? 200;
        }
      }
    )
  };
});

// Jsdom lacks `elementFromPoint`; the pleiades OTP input calls it on input.
if (typeof document !== 'undefined' && !document.elementFromPoint) {
  document.elementFromPoint = () => null;
}

// Jsdom lacks the Web Animations API used by svelte transitions.
if (typeof Element !== 'undefined' && !Element.prototype.animate) {
  Element.prototype.animate = function animate() {
    const animation = {
      cancel: () => {},
      finished: Promise.resolve(),
      onfinish: null as null | (() => void),
      pause: () => {},
      play: () => {}
    };
    queueMicrotask(() => animation.onfinish?.());
    return animation as unknown as Animation;
  };
}

// Jsdom lacks pointer capture, which bits-ui Select calls on pointerdown.
if (typeof Element !== 'undefined' && !Element.prototype.hasPointerCapture) {
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.setPointerCapture = () => {};
  Element.prototype.releasePointerCapture = () => {};
}
