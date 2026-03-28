import { getGeneralSettings } from '$lib/backend/settings.svelte';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
  let generalSettings = await getGeneralSettings(fetch);

  return {
    generalSettings
  };
};
