# Instrument — task API

A small REST API for a to-do list, written in **Rust** with **Axum** and
**Postgres**. It is the backend for [VOCO-todo-frontend](https://github.com/rasmus-antsi/VOCO-todo-frontend),
a React client that talks to it over `/api`.

The whole server is one file — [`src/main.rs`](src/main.rs), about 130 lines.

## Stack

| Piece        | What it does                                                        |
| ------------ | ------------------------------------------------------------------- |
| **axum**     | HTTP routing and JSON extraction                                    |
| **sqlx**     | Async Postgres queries, checked at runtime; also runs the migrations |
| **tokio**    | The async runtime everything sits on                                |
| **serde**    | Turns structs into JSON and back                                    |
| **dotenvy**  | Loads `.env` in development                                         |
| **tower-http** | The CORS layer, needed once the frontend is on another host       |

## Running it locally

You need [Rust](https://rustup.rs) and Docker.

```bash
# 1. Start Postgres (docker-compose.yml maps it to localhost:5433)
docker compose up -d

# 2. Point the app at it
cp .env.example .env

# 3. Run. Migrations apply automatically on boot.
cargo run
```

The API is then on **http://localhost:3000**. Check it:

```bash
curl localhost:3000/api/health          # -> ok
curl localhost:3000/api/tasks           # -> []
```

To run the frontend against it, see that repo's README — its dev server
proxies `/api` here, so no CORS setup is needed while developing.

## Endpoints

Everything is under `/api`. A task is
`{ "id": 1, "user_id": 1, "title": "Buy milk", "done": false }`.

| Method   | Path              | Body              | Returns                     |
| -------- | ----------------- | ----------------- | --------------------------- |
| `GET`    | `/api/health`     | —                 | `ok`                        |
| `GET`    | `/api/tasks`      | —                 | `Task[]`, ordered by `id`   |
| `POST`   | `/api/tasks`      | `{ "title": … }`  | the created `Task`          |
| `PATCH`  | `/api/tasks/{id}` | `{ "done": … }`   | the updated `Task`          |
| `DELETE` | `/api/tasks/{id}` | —                 | the deleted `Task`          |

`PATCH` and `DELETE` return **404** if the id does not exist. Anything that
goes wrong in the database is logged and returned as a **500** — the client
never sees the SQL error text.

Try it:

```bash
curl -X POST localhost:3000/api/tasks \
  -H 'Content-Type: application/json' \
  -d '{"title":"Buy milk"}'

curl -X PATCH localhost:3000/api/tasks/1 \
  -H 'Content-Type: application/json' \
  -d '{"done":true}'

curl -X DELETE localhost:3000/api/tasks/1
```

## How it works

**`main` wires four things together** and then serves:

1. `dotenvy` loads `.env`, and `DATABASE_URL` is read out of the environment.
2. `PgPool::connect` opens a connection pool. The pool is the router's
   **state**, so every handler gets a connection by asking for
   `State(pool): State<PgPool>` in its signature.
3. `sqlx::migrate!()` applies anything in `migrations/` that has not run yet.
   The macro embeds the SQL files into the binary at compile time, so a
   deployed build carries its own schema and there is no separate migrate step.
4. The routes are nested under `/api` and wrapped in the CORS layer.

**Handlers return `Result<Json<T>, AppError>`.** `AppError` is a two-variant
enum — `NotFound` and `Db` — and it implements two traits that do the work:

- `From<sqlx::Error>`, which is what lets a handler write `.await?` and have a
  database failure become an `AppError` on its own;
- `IntoResponse`, which is where the enum turns into an actual status code.

That pairing is why the handlers have no error handling in them. The happy path
is the only path written out; `?` handles the rest.

**Queries use `query_as::<_, Task>`**, which maps result columns onto the
`Task` struct via its `FromRow` derive. Values are always `.bind()`ed rather
than formatted into the string, so user input cannot alter the query — that is
what keeps SQL injection out.

`fetch_optional` is the distinction that produces the 404: an `UPDATE … RETURNING`
that matches no row is not an error, it is `None`, and `.ok_or(AppError::NotFound)?`
turns that into the right status.

## Database

Two tables, created by `migrations/20260919151307_init.sql`:

```
users                       tasks
─────                       ─────
id            BIGSERIAL PK  id          BIGSERIAL PK
email         TEXT UNIQUE   user_id     BIGINT → users(id) ON DELETE CASCADE
password_hash TEXT          title       TEXT
created_at    TIMESTAMPTZ   done        BOOLEAN default false
                            created_at  TIMESTAMPTZ
```

**There is no auth yet**, so every query hardcodes `user_id = 1`. The schema is
already built for real users — the foreign key and the cascade are there — but
nothing issues sessions, so the second migration seeds a single user with
`id = 1` for the app to attach its tasks to. Adding login means adding a
sessions or token layer and replacing that literal `1` with the id it resolves.

Migrations run at startup, but `sqlx-cli` is still handy for adding one:

```bash
cargo install sqlx-cli --no-default-features --features rustls,postgres
sqlx migrate add <name>      # creates migrations/<timestamp>_<name>.sql
```

## Configuration

| Variable       | Required | Default | Notes                                    |
| -------------- | -------- | ------- | ---------------------------------------- |
| `DATABASE_URL` | yes      | —       | Postgres connection string               |
| `PORT`         | no       | `3000`  | Hosting platforms usually set this        |

`.env` is gitignored; `.env.example` is the committed template.

## Deploying

The `Dockerfile` builds a release binary and copies it into a slim Debian
image, so the deployed container is the binary plus CA certificates.

On a platform like Railway: create a Postgres instance, deploy this repo, and
set `DATABASE_URL` to the database's connection string. `PORT` is injected by
the platform, and migrations run themselves on the first boot.

The CORS layer currently allows **any** origin, which is the right default
while the frontend's URL is still changing. Once it is fixed, narrow it to that
one origin in `main.rs`.

## What is missing

Honest list, since this is a school project:

- **No authentication.** `user_id = 1` everywhere.
- **No validation.** An empty title is accepted.
- **No tests.**
- **`title` cannot be edited** — `PATCH` only accepts `done`.
- **No pagination.** `GET /api/tasks` returns everything.
