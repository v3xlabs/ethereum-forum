# Contributing

This repository includes a Vite TypeScript frontend and a Rust backend connected to a Postgres database.

## Layout

```toml
 - /app # rust server (poem, async-std)
 - /web # frontend (vite, react)
```

## Getting Started

There are multiple ways you can run the project.
If you wish to modify the backend ensure you have read the `app/` section.

For running the `frontend` connected to production you can:

```bash
cd ./web && pnpm dev:prod
```

For local development, clone the repo, install dependencies, and follow these instructions:

### Dependencies

- [pnpm](https://pnpm.io/) and [node](https://nodejs.org/) for the frontend
- [Rust](https://www.rust-lang.org/tools/install)/cargo for running the backend in development
- [Docker](https://www.docker.com/) for running the database and Meilisearch

## app/

To run the backend in development mode, first set up the environment by copying the example .env file and modifying it to your liking.

```bash
cd ./app

# Setup .env file
cp .env.example .env

# Start docker (ensure you are in the `app` directory)
docker compose up -d

# Setup database (development-only)
cargo sqlx migrate run
cargo sqlx prepare
```

This spins up a postgres database & meilisearch instance & ensures the tables are up to date.

### Running the backend

```bash
cargo run
```

## web/

Install dependencies using pnpm, then run the `dev` script.

```bash
cd ./web
pnpm install
pnpm dev
```

If you wish to run the frontend connected to the production backend you can:

```bash
cd ./web
pnpm dev:prod
```

### API Types

The typescript types in the frontend ([schema.gen.ts](./web/src/api/schema.gen.ts)) are *generated automatically* when the dev server is running.

If you wish to generate the types manually you can:

```bash
cd ./web
pnpm api-schema
```
