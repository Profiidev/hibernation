import * as core from '@actions/core';
import * as tc from '@actions/tool-cache';
import { execSync } from 'child_process';

const execCmd = (command: string): boolean => {
  try {
    execSync(command, {
      encoding: 'utf-8'
    });
    return true;
  } catch (error) {
    core.error(`Command failed: ${command}`);
    return false;
  }
};

const run = async () => {
  const version = core.getInput('version') || 'latest';
  const url = core.getInput('url');
  const token = core.getInput('token');
  const signingKey = core.getInput('signing-key');
  const repo = 'ProfiiDev/hibernation';

  const platform = process.platform;
  const arch = process.arch;

  let osPart = '';
  switch (platform) {
    case 'darwin':
      osPart = 'macos';
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

  const downloadUrl =
    version === 'latest'
      ? `https://github.com/${repo}/releases/latest/download/${assetName}`
      : `https://github.com/${repo}/releases/download/${version}/${assetName}`;

  core.info(`Downloading ${assetName} from ${downloadUrl}`);

  const pathToTarball = await tc.downloadTool(downloadUrl);
  const pathToCli = await tc.extractTar(pathToTarball);

  core.addPath(pathToCli);

  core.info(`Successfully installed hibernation CLI`);

  if (url && token) {
    execCmd(`hibernation auth --url ${url} ${token}`);
    core.info('Configured hibernation CLI with provided URL and token');
  } else if (url) {
    execCmd(`hibernation set-url ${url}`);
    core.info('Configured hibernation CLI with provided URL');
  } else if (token) {
    core.warning('Token provided without URL, skipping configuration');
  }

  if (signingKey) {
    core.exportVariable('HIBERNATION_SIGNING_KEY', signingKey);
    core.info('Set signing key for hibernation CLI');
  }
};

try {
  run();
} catch (error: any) {
  core.setFailed(error.message);
}
