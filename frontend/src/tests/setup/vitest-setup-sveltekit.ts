import { vi } from 'vitest';

// Global stubs for SvelteKit's `$app/*` modules so components built on
// Sveltekit-superforms (which call `beforeNavigate`, read `page` state, etc.)
// Can render under jsdom. Individual tests may still override
// `$app/navigation` with their own `vi.mock` to assert navigation calls.

vi.mock('$app/navigation', () => ({
  afterNavigate: vi.fn(),
  beforeNavigate: vi.fn(),
  disableScrollHandling: vi.fn(),
  goto: vi.fn(async () => Promise.resolve()),
  invalidate: vi.fn(async () => Promise.resolve()),
  invalidateAll: vi.fn(async () => Promise.resolve()),
  onNavigate: vi.fn(),
  preloadCode: vi.fn(async () => Promise.resolve()),
  preloadData: vi.fn(async () => Promise.resolve()),
  pushState: vi.fn(),
  refreshAll: vi.fn(async () => Promise.resolve()),
  replaceState: vi.fn()
}));

vi.mock('$app/state', async () => import('./app-state.svelte'));
