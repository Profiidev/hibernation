import { get } from 'positron-components/backend';

export const size_to_gib = (size: number) => size / 1024 / 1024 / 1024;

export const sendCliCode = async (code: string) =>
  await get(`http://localhost:16401?code=${code}`);
