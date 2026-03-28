import * as core from '@actions/core';
import * as tc from '@actions/tool-cache';

const run = async () => {
  const version = core.getInput('version') || 'latest';
  const repo = 'ProfiiDev/hibernation';

  const platform = process.platform;
  const arch = process.arch;

  let osPart = '';
  switch (platform) {
    case 'darwin':
      osPart = 'darwin';
      break;
    case 'linux':
      osPart = 'linux';
      break;
    default:
      throw new Error(`Unsupported platform: ${platform}`);
  }

  let archPart = '';
  switch (arch) {
    case 'x64':
      archPart = 'x86-64';
      break;
    case 'arm64':
      archPart = 'arm64';
      break;
    default:
      throw new Error(`Unsupported architecture: ${arch}`);
  }

  const suffix = platform === 'linux' ? '-gnu' : '';
  const assetName = `hibernation-${osPart}-${archPart}${suffix}.tar.gz`;

  const url =
    version === 'latest'
      ? `https://github.com/repos/${repo}/releases/latest/download/${assetName}`
      : `https://github.com/repos/${repo}/releases/download/${version}/${assetName}`;

  core.info(`Downloading ${assetName} from ${url}`);

  const pathToTarball = await tc.downloadTool(url);
  const pathToCli = await tc.extractTar(pathToTarball);

  core.addPath(pathToCli);

  core.info(`Successfully installed hibernation CLI`);
};

try {
  run();
} catch (error: any) {
  core.setFailed(error.message);
}
