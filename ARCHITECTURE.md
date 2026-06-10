# Architecture

A domain-driven, layered Actix Web service. The codebase is small today but is
structured so new domains/modules slot in without reshaping existing layers.

---

## Layered overview

```
                      HTTP request
                          │
                          ▼
   web_server/          ── handlers (thin): extract → validate → call compute → respond
     │   middlewares/   ── RateLimit (scope), SessionMiddleware (per resource)
     ▼
   request_validations/ ── field validation, collects all errors before compute
     │
     ▼
   compute/             ── business logic; orchestrates queries + utils
     │
     ▼
   queries/             ── MongoDB access only (find/insert/update). No logic.
     │
     ▼
   models/              ── pure data structs (DB documents + DTOs). No side effects.
     │
     ▼
   applications/        ── infra connections (Mongo, Redis) + shared AppState
```

Cross-cutting layers used by all of the above:

- **`utils/`** — JWT, password/code hashing, Redis cache helpers, constants.
- **`errors.rs`** — one `ApiError` enum; implements actix `ResponseError` so any
  layer can `?`-propagate and the HTTP status + JSON body are produced centrally.
- **`config.rs`** — environment-based configuration loader.
- **`traits.rs`** — placeholder for store traits/mocks as the data layer grows.

**Rule of thumb**
- `web_server` handlers extract request data, run validation, and call `compute` — nothing else.
- `compute` runs logic, calls `queries`, talks to Redis via `utils::cache`.
- `queries` only talk to MongoDB — no business rules.
- `models` are pure data — no methods with side effects.

---

## Directory layout

```
src/
├── main.rs                          # startup: config → connections → server (CORS + routes)
├── config.rs                        # AppConfig::from_env
├── errors.rs                        # ApiError + ResponseError + From conversions
├── traits.rs                        # (placeholder) store-trait abstractions
│
├── applications/                    # infrastructure connections + shared state
│   ├── application_store.rs         #   AppState { db, jwt_secret, redis_client, dev_email_events, rate_limit_per_second }
│   ├── connect_mongo.rs             #   MongoDB connection factory
│   └── connect_redis.rs             #   Redis client factory
│
├── models/                          # pure data structures (+ root re-exports)
│   ├── user.rs                      #   User, UserRole (+ as_str/from_str)
│   ├── auth.rs                      #   LoginChallenge, EmailLog
│   ├── task.rs                      #   Task, TaskStatus, TaskPriority (+ as_str)
│   ├── session.rs                   #   JwtClaims, AuthUser
│   └── dtos.rs                      #   request/response DTOs + compute outcomes
│
├── queries/                         # MongoDB access layer
│   ├── users.rs                     #   find_by_email / find_by_id / insert
│   ├── auth.rs                      #   challenges + email logs
│   └── tasks.rs                     #   insert / find_by_ids / find_assigned_to / assign_to
│
├── compute/                         # business logic
│   ├── authorization.rs             #   authenticate(token) -> AuthUser; require_admin
│   ├── auth/
│   │   ├── seed_users.rs            #   idempotent demo seeding
│   │   ├── login.rs                 #   verify creds, create 2FA challenge
│   │   └── verify_2fa.rs            #   validate code, issue JWT
│   └── tasks/
│       ├── create_task.rs
│       ├── assign_tasks.rs          #   assign + cache invalidation
│       └── view_my_tasks.rs         #   cache-aware read
│
├── request_validations/             # input validation (Result<(), Vec<String>>)
│   ├── common.rs                    #   shared validators (email)
│   ├── auth.rs
│   └── tasks.rs
│
├── utils/                           # shared helpers
│   ├── jwt.rs                       #   issue_access_token / decode_access_token
│   ├── security.rs                  #   Argon2 hash/verify
│   ├── cache.rs                     #   Redis get/set/del + incr_with_ttl
│   └── constants.rs                 #   collection names, cache keys/TTLs, token lifetimes
│
└── web_server/                      # HTTP layer
    ├── create_connections.rs        #   THE router (.configure target)
    ├── health_check.rs
    ├── middlewares/
    │   ├── rate_limit.rs            #   RateLimit (per-identity, per-second bucket)
    │   └── session.rs               #   SessionMiddleware (authn) + AuthUser extractor
    ├── auth/
    │   ├── seed_users.rs · login.rs · verify_2fa.rs · dev_email_logs.rs
    └── tasks/
        ├── create_task.rs · assign_tasks.rs · view_my_tasks.rs
```

---

## Request lifecycle

```
request
  → CORS (App-level, Cors::permissive for local dev)
  → RateLimit middleware            (scope-level; per-identity per-second bucket)
  → SessionMiddleware               (resource-level; only on protected routes)
        decodes JWT → AuthUser into request extensions, or short-circuits 401
  → handler (web_server/…)
        AuthUser extractor (from extensions)
        require_admin(...)           (admin-only routes)
        validate_*(...)              (request_validations/)
        compute::…                   (business logic)
            queries::…               (MongoDB)
            utils::cache::…          (Redis)
  → ApiError (if any) → ResponseError → HTTP status + JSON
```

**Middleware order** (outer → inner): `CORS → RateLimit → SessionMiddleware → handler`.

---

## Routes

| Method | Path | Auth | Notes |
|---|---|---|---|
| GET | `/health` | public | liveness |
| POST | `/seed/users` | public | idempotent demo seeding |
| POST | `/auth/login` | public | verify creds → 2FA challenge |
| GET | `/dev/email-logs/latest` | public | **dev only**: latest 2FA code |
| POST | `/auth/verify-2fa` | public | validate code → JWT |
| POST | `/tasks` | session + admin | create task |
| POST | `/tasks/assign` | session + admin | assign tasks + invalidate cache |
| GET | `/tasks/view-my-tasks` | session | cache-aware assigned-tasks view |

Public routes are registered at the top level; protected routes live inside a
rate-limited `web::scope("")`, each `web::resource(...)` wrapped with
`SessionMiddleware` (see `web_server/create_connections.rs`).

---

## Authentication & authorization

- **Authentication** is a **middleware** (`web_server::middlewares::session::SessionMiddleware`),
  wrapped per protected resource. It reads `Authorization: Bearer <jwt>`, calls
  `compute::authorization::authenticate`, and inserts the resulting `AuthUser`
  into the request extensions (or returns `401`).
- Handlers receive the caller via the **`AuthUser` extractor** (`impl FromRequest`),
  which reads from those extensions — so authentication is enforced at the route
  boundary, not inside handlers.
- **Authorization** is the fine-grained `require_admin(&auth_user)` check inside
  admin-only handlers (`view-my-tasks` is any authenticated user, so it has none).

### 2FA login flow
1. `POST /auth/login` — verify password (Argon2). Generate a 6-digit code, store
   it **hashed** in a `LoginChallenge` (5-min expiry, `used_at = null`). Write a
   masked `EmailLog`. (Dev: stash the real code in memory + print to console.)
2. `POST /auth/verify-2fa` — load challenge; reject if used/expired; verify the
   code against its hash; atomically mark it used (`used_at: null` filter →
   one-time use); issue a 24h JWT.

---

## Rate limiting

`web_server::middlewares::rate_limit::RateLimit` — one Redis bucket per identity
per second:

```
key:   rate_limit:user:<id>:<unix_second>   (authenticated)
       rate_limit:ip:<addr>:<unix_second>   (fallback when no valid token)
logic: INCR key; on first hit EXPIRE; if count > limit → 429
```

The limit is `AppState::rate_limit_per_second` (env `RATE_LIMIT_PER_SECOND`,
default `60`, `0` disables). The middleware **fails open** — a Redis error never
blocks traffic.

---

## Caching

Per-user task-view cache in Redis (`utils::cache`):

- **Key:** `tasks:view:<user_id>`, TTL 300s.
- **Read** (`view_my_tasks`): on hit, decode and return with `cache.hit = true`;
  on miss, query Mongo, build payload, cache it, return `cache.hit = false`.
- **Invalidation** (`assign_tasks`): after assigning, delete the cache key for
  the new assignee and every previous assignee of the affected tasks.

---

## Error handling

`ApiError` (`errors.rs`) is the single error type returned by `queries`,
`compute`, and handlers:

| Variant | HTTP |
|---|---|
| `BadRequest` / `Validation(Vec<String>)` | 400 |
| `Unauthorized` | 401 |
| `Forbidden` | 403 |
| `NotFound` | 404 |
| `Internal` | 500 |

It implements `ResponseError`, so handlers return `Result<HttpResponse, ApiError>`
and actix renders the body (`{"error": "...", "details": [...]}`). `From` impls
convert `mongodb`, `redis`, `serde_json`, and `anyhow` errors into `Internal`.

---

## Data model (MongoDB collections)

| Collection | Struct | Purpose |
|---|---|---|
| `users` | `User` | accounts; Argon2 `password_hash`, `role` |
| `login_challenges` | `LoginChallenge` | one-time 2FA codes (hashed), expiry, `used_at` |
| `email_logs` | `EmailLog` | audit of 2FA "sends" (masked code only) |
| `tasks` | `Task` | tasks; `created_by_id`, `assigned_to_id` |

No transactions are used, so a single-node MongoDB (local or Atlas) is sufficient.

---

## Configuration

Loaded once at startup (`config.rs`, via `dotenvy`). See `.env.example`.

| Var | Default | Purpose |
|---|---|---|
| `HOST` | `127.0.0.1` | bind host |
| `PORT` | `8080` | bind port |
| `MONGO_URI` | `mongodb://localhost:27017` | MongoDB connection string |
| `MONGO_DB_NAME` | `task_auth_db` | database name |
| `REDIS_URL` | `redis://127.0.0.1:6379` | Redis connection |
| `JWT_SECRET` | `change-me-in-env` | HS256 signing secret |
| `RATE_LIMIT_PER_SECOND` | `60` | per-identity request cap (`0` = off) |

CORS is `Cors::permissive()` for local development (the Next.js client at
`:3000` calls the API at `:8080`). Tighten the allowed origin before any
non-local deployment.

---

## Conventions

| Layer | File naming | Function shape |
|---|---|---|
| `web_server` | `<verb>_<resource>.rs` | `Result<HttpResponse, ApiError>` |
| `compute` | `<action>.rs` | `Result<T, ApiError>` |
| `queries` | `<entity>.rs` (grouped) | `Result<T, ApiError>`, take `&Database` |
| `models` | `<entity>.rs` | pure structs |
| `request_validations` | `<domain>.rs` | `Result<(), Vec<String>>` |

---

## Extending the service

- **New endpoint:** add `queries/` fn → `compute/` fn → `web_server/` handler →
  register in `create_connections.rs`. Add request/response types to `models/dtos.rs`.
- **New protected route:** wrap its `web::resource` with `SessionMiddleware`;
  add `require_admin` (or a future role check) in the handler if needed.
- **New middleware:** add under `web_server/middlewares/` and `.wrap()` it at the
  appropriate scope/resource in `create_connections.rs`.
- **Mockable data layer:** define store traits in `traits.rs`, implement for the
  real Mongo store and an in-memory mock, and have `compute` depend on the trait.

---

## Tests

- Unit tests live beside the code (`utils::jwt`, `utils::security`).
- `tests/e2e_validation_flow.rs` drives the full HTTP flow end-to-end
  (`#[ignore]`; run with `cargo test -- --ignored` against a live API + Mongo + Redis).
- `examples/clear_tasks.rs` wipes `tasks`/`login_challenges`/`email_logs` for a
  clean e2e run (`cargo run --example clear_tasks`).
