# Hibernation

Hibernation is a self-hosted [Nix](https://nixos.org/) binary cache.

## Installation

### Docker

```bash
docker run \
  -p 8000:8000 \
  --name hibernation \
  -v hibernation_data:/data \
  ghcr.io/profiidev/hibernation/hibernation:latest
```

### Docker Compose

```yaml
services:
  hibernation:
    image: ghcr.io/profiidev/hibernation/hibernation:latest
    ports:
      - '8000:8000'
    volumes:
      - hibernation_data:/data
volumes:
  hibernation_data:
```

### Configuration

The critical configuration is done via environment variables everything else via the UI:

| Variable            | Description                                                                | Default                              |
| ------------------- | -------------------------------------------------------------------------- | ------------------------------------ |
| DB_URL              | PostgreSQL or SQLite connection URL                                        | sqlite:/data/hibernation.db?mode=rwc |
| DATABASE_LOGGING    | Enable SQL query logging                                                   | false                                |
| STORAGE_PATH        | Directory for storing Nix store paths localy (only used when not using S3) | /data/storage                        |
| S3_HOST             | S3-compatible storage host URL (e.g., MinIO)                               | -                                    |
| S3_ACCESS_KEY       | Access key for S3 storage                                                  | -                                    |
| S3_SECRET_KEY       | Secret key for S3 storage                                                  | -                                    |
| S3_REGION           | Region for S3 storage                                                      | -                                    |
| S3_BUCKET           | Bucket name for S3 storage                                                 | -                                    |
| S3_FORCE_PATH_STYLE | Whether to use path-style URLs for S3 (required for MinIO)                 | false                                |
| LOG_LEVEL           | Log level for the backend (e.g., info, debug)                              | info                                 |
| METRICS_NAME        | The name to use as the app label in Prometheus metrics                     | hibernation                          |
| PORT                | Port for the backend server to listen on                                   | 8000                                 |

See `backend/src/config.rs` for all non-standard configuration options.

## CLI Usage

The CLI tool allows you to interact with the Hibernation server, primarily for authenticating and pushing Nix store paths.

You can install it from the binaries published to GitHub Releases, using Cargo:

```bash
cargo install --git https://github.com/ProfiiDev/hibernation hibernation
```

Or in you nix config:

```nix
{
  inputs = {
    ...
    hibernation.url = "github:profiidev/hibernation/latest";
  }
  ...
}
```

```nix
{ inputs, pkgs, ... }:
{
  environment.systemPackages = with pkgs; [
    inputs.hibernation.packages.${stdenv.hostPlatform.system}.default
  ];
}
```

### Pushing Nix Store Paths

1. To push a Nix store path to the Hibernation server you first need a signing key pair. You can generate a new secret key using the following command:

   ```bash
   nix key generate-secret --key-name <key-name> > secret.key
   ```

2. Create a public key from the secret key:

   ```bash
   nix key convert-secret-to-public < secret.key > public.key
   ```

3. Create a Cache in your Hibernation instance using the public key.

4. Use the following command from the cli to push paths (all missing configuration will be prompted for):
   ```bash
   hibernation push <cache-name> /nix/store/your-store-path
   ```

## Deployment

### Prerequisites

- [Docker](https://www.docker.com/)

### Components

The project consists of three main components:

- **Backend (`backend/`)**: Written in Rust using `axum`. It handles API requests, database interactions via `sea-orm`, and file storage.
- **Frontend (`frontend/`)**: A SvelteKit application providing a user-friendly interface.
- **CLI (`cli/`)**: A Rust-based command-line tool (`hibernation`) for interacting with the backend.
- **Shared (`shared/`)**: Contains shared types and utilities used by both the backend and CLI.

### Running with Docker Compose

The easiest way to run Hibernation is using Docker Compose. This will start the backend, frontend, and a PostgreSQL database.

```bash
docker compose up -d
```

Once running, the services will be available at:

- **Frontend**: `http://localhost:5173`
- **Backend**: `http://localhost:5173/api`
- **PostgreSQL**: `localhost:9301`

## TODO

- move url setting from frontend to env var and document in readme
- add support for virtual host based cache access (requires more restrictions on cache names)
- document action usage in readme
- add overview page in frontend
- nix store watch mode in cli + auto push + example in cache page
