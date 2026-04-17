import { getGeneralSettings } from '$lib/client';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
  const { data: generalSettings } = await getGeneralSettings({ fetch });

  return {
    generalSettings
  };
};
