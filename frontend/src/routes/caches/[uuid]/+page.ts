import { getGeneralSettings } from '#lib/client/index.ts';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
  const generalSettings = getGeneralSettings({ fetch }).then(
    ({ data }) => data
  );

  return {
    generalSettings
  };
};
