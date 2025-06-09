This repository includes a Vite TypeScript frontend and a Rust backend connected to a Postgres database.
For local development, clone the repo, install dependencies, and follow these instructions:

# Dependencies

- [pnpm](https://pnpm.io/) and [node](https://nodejs.org/) for the frontend
- [Rust](https://www.rust-lang.org/tools/install)/cargo for running the backend in development
- [Docker](https://www.docker.com/) for running the database and Meilisearch

# Building

## Backend

To run the backend in development mode, first set up the environment by copying the example .env file and modifying it to your liking.

```
cp app/.env.example app/.env
```

Run the Postgres database and Meilisearch backend using the provided Docker Compose file:

```
docker compose up -d
```

Compile and run the Rust backend. It is served on the port defined in the environment (3000 by default):

```
cargo run
```

## Frontend

Install dependencies using pnpm, then run the `dev` script.
