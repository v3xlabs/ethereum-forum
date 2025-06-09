# Ethereum Forum

## MCP

This project exposes mcp tools via the following endpoint:

```url
https://ethereum.forum/mcp
```

You can easily add it to your favourite editor by adding the following to your `settings.json`:

```json
{
  "mcpServers": {
    "ethereum-forum": {
      "url": "https://ethereum.forum/mcp"
    }
  }
}
```

## Data Sources

- [x] Protocol Calendar (ical)
- [x] ethereum-magicians.org (Discourse)
- [ ] ethresear.ch (Discourse) (TODO)
- [ ] github/ethereum/pm (TODO)
  - issues & comments
- [ ] github/ethereum/eips (TODO)
  - data & prs
- [ ] github/ethereum/ercs (TODO)
  - data & prs
- [ ] blog.ethereum.org (TODO)

## Development and self-hosting

This repository includes TS frontend and a Rust backend connecting to a postgress database.
For local development, clone the repo, install dependencies and follow these intructions:

### Dependencies 
- [pnpm](https://pnpm.io/) and [node](https://nodejs.org/) for the frontend
- [Rust](https://www.rust-lang.org/tools/install)/cargo compiling backend
- [Docker](https://www.docker.com/) for running the database and meilisearch

### Building

#### Backend

To set up the backend, first edit the example enviroment variables and save it as `.env`

```
cd app
nano .env.example
cp .env.example .env
```

Run postgres db and melisearch backend using provided docker compose

```
docker compose up -d
```

Compile and run the rust backend, it's served on port defined in the enviroment (3000 by default)
```
cargo run
```

#### Frotnend

Install dependencies using pnpm manager 
```
cd web
pnpm install
```
Start the frontend dev server, served on port 5173:

```
pnpm dev
```
Or to build for production
```
pnpm build
```

