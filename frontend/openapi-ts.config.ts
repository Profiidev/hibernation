import { defineConfig } from '@hey-api/openapi-ts';

export default defineConfig({
  input: 'http://localhost:5173/openapi.json',
  output: {
    path: 'src/lib/client',
    postProcess: ['prettier']
  },
  plugins: [
    '@hey-api/typescript',
    {
      name: '@hey-api/sdk',
      operations: {
        methodName: (name) => {
          return name.replace(/^([a-z]+)Api/i, '$1');
        }
      }
    },
    '@hey-api/transformers',
    '@hey-api/client-fetch'
  ],
  logs: './build'
});
