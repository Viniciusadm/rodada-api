# Rodada API

Rodada runs live quizzes and games for groups: a host builds a match, shows it on a big screen and players join from their phones. This repository holds the API: accounts, the question bank, match templates, matches and the realtime channel. The web app lives in its own repository.

| Part | What it is |
| --- | --- |
| [`rodada-api`](https://github.com/Viniciusadm/rodada-api) | This repository. Rust API over MySQL. |
| [`rodada-web`](https://github.com/Viniciusadm/rodada-web) | React app for hosts and players. |

## What it does

- Accounts with Argon2 password hashes and JWT access tokens (`/api/v1/auth`).
- A question bank with true/false, multiple choice, free text and buzzer questions, and CSV import.
- Match templates with the game mode (`quiz` or `buzzer`), the scoring mode (`classic`, `initial_points`, `elimination`) and when to show the ranking.
- Matches: players join a room with a code, the host starts the match, draws rounds and ends it, and answers are graded on the server.
- Notifications and notification preferences.
- The big screen: each template can carry a [Ribalta](https://github.com/Viniciusadm/ribalta) document, which the API compiles and checks with the `ribalta` crate (`/api/v1/screens`).
- A WebSocket endpoint (`/ws`) that pushes match events to the host, the players and the big screen.

The code follows a layered layout: `domain` holds the rules and types, `application` the use cases and their ports, `infrastructure` the MySQL repositories, auth and clock, and `api` the axum routes. `realtime` is the in-process hub behind the WebSocket.

## Requirements

- Rust 1.94+ (the `ribalta` crate requires it; this crate uses edition 2021)
- MySQL 8

Stack: axum 0.7, sqlx 0.8 (MySQL, rustls), jsonwebtoken 9, argon2 0.5, tokio.

## Running locally

Copy the example environment and set `DATABASE_URL` and `JWT_SECRET`:

```bash
cp .env.example .env
```

Start the API. It listens on `127.0.0.1:3001` (`HTTP_HOST` and `HTTP_PORT` change it), and runs the migrations in [`migrations`](migrations) on startup unless `RUN_MIGRATIONS=false`:

```bash
cargo run
```

Without `DATABASE_URL` the server still starts, but the endpoints that need persistence return a configuration error. `DEMO_SEEDS=true` adds a sample organization, an admin user (`DEMO_ADMIN_EMAIL`, `DEMO_ADMIN_PASSWORD`), players, questions and a finished match.

## Tests

```bash
cargo test
```

The tests cover the use cases with in-memory fakes (auth, questions, matches), the HTTP routes, the realtime hub, the answer grading and the big screen compilation. None of them need a database.
