// Reactive stand-in for `$app/state`. Superforms 3 reads `page` through
// `toStore(() => page)`, so `form` and `status` must be reactive. `form` is raw
// (like in SvelteKit) because superforms structured-clones it, which fails on
// `$state` proxies.
let form = $state.raw<unknown>(null);
let status = $state(200);

export const page = {
  data: {},
  error: null,
  get form() {
    return form;
  },
  set form(value: unknown) {
    form = value;
  },
  params: {},
  route: { id: null },
  get status() {
    return status;
  },
  set status(value: number) {
    status = value;
  },
  url: new URL('http://localhost/')
};

export const navigating = {
  complete: Promise.resolve(),
  from: null,
  to: null,
  type: null
};

export const updated = { current: false };
