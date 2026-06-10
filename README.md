# Rust Auth Project (Actix + MongoDB + Redis)

Task Management API with:
- Email-password login + 2FA challenge
- JWT after 2FA verification
- Role-based access (`admin`, `staff`)
- Task assignment
- Per-user Redis caching for `GET /tasks/view-my-tasks`

## Stack
- Rust (edition 2024, compatible with 2021+ requirements)
- Actix Web
- MongoDB (Atlas/local)
- Redis (local via Docker Compose)
- Argon2 password hashing
- JWT via `jsonwebtoken`

## Implemented Endpoints
- `GET /health`
- `POST /seed/users`
- `POST /auth/login`
- `GET /dev/email-logs/latest`
- `POST /auth/verify-2fa`
- `POST /tasks` (admin only)
- `POST /tasks/assign` (admin only)
- `GET /tasks/view-my-tasks` (auth required, cache-aware)

## Data Model
Mongo collections:
- `users`
- `login_challenges`
- `email_logs`
- `tasks`

## Business Rules Covered
- Roles: `admin` and `staff`
- Only admin can create tasks
- Only admin can assign tasks
- Staff task creation returns `403`
- `view-my-tasks` returns only logged-in user assigned tasks
- 2FA code expires in 5 minutes
- 2FA code is one-time use
- 2FA code stored hashed (not plain text)
- Cache is per-user and invalidated on assignment changes

## Setup
1. Copy env:

```bash
cp .env.example .env
```

2. Edit `.env` with your Mongo Atlas values and JWT secret.

3. Start Redis:

```bash
docker compose up -d
```

4. Run API:

```bash
cargo run
```

Server default: `http://127.0.0.1:8080`

## Validation Flow (curl)

1. Seed users:

```bash
curl -s -X POST http://127.0.0.1:8080/seed/users | jq
```

2. Login as Admin (returns challenge id):

```bash
curl -s -X POST http://127.0.0.1:8080/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"admin@example.com","password":"Admin@123"}' | jq
```

3. Get dev 2FA code:

```bash
curl -s 'http://127.0.0.1:8080/dev/email-logs/latest?email=admin@example.com' | jq
```

4. Verify Admin 2FA (replace challenge id and code):

```bash
curl -s -X POST http://127.0.0.1:8080/auth/verify-2fa \
  -H 'Content-Type: application/json' \
  -d '{"login_challenge_id":"<ADMIN_CHALLENGE_ID>","code":"<ADMIN_CODE>"}' | jq
```

Save `access_token` as `ADMIN_TOKEN`.

5. Create exactly 5 tasks as Admin:

```bash
for p in high medium low high medium; do
  curl -s -X POST http://127.0.0.1:8080/tasks \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H 'Content-Type: application/json' \
    -d "{\"title\":\"Task-$p\",\"description\":\"desc\",\"status\":\"todo\",\"priority\":\"$p\"}" | jq
 done
```

Collect task ids from responses.

6. Assign exactly 3 tasks to James Bond:

```bash
curl -s -X POST http://127.0.0.1:8080/tasks/assign \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{
    "task_ids": ["<TASK_ID_1>", "<TASK_ID_2>", "<TASK_ID_3>"],
    "assignee_email": "jamesbond@example.com"
  }' | jq
```

7. Login as James Bond:

```bash
curl -s -X POST http://127.0.0.1:8080/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"jamesbond@example.com","password":"Bond@123"}' | jq
```

8. Get James Bond 2FA code:

```bash
curl -s 'http://127.0.0.1:8080/dev/email-logs/latest?email=jamesbond@example.com' | jq
```

9. Verify James Bond 2FA and save `JAMES_TOKEN`:

```bash
curl -s -X POST http://127.0.0.1:8080/auth/verify-2fa \
  -H 'Content-Type: application/json' \
  -d '{"login_challenge_id":"<JAMES_CHALLENGE_ID>","code":"<JAMES_CODE>"}' | jq
```

10. James tries to create task (must be 403):

```bash
curl -i -X POST http://127.0.0.1:8080/tasks \
  -H "Authorization: Bearer $JAMES_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"title":"Nope","description":"Nope","status":"todo","priority":"low"}'
```

11. James view tasks first time (cache miss):

```bash
curl -s -X GET http://127.0.0.1:8080/tasks/view-my-tasks \
  -H "Authorization: Bearer $JAMES_TOKEN" | jq
```

Expected: `summary.total_assigned_tasks = 3` and `cache.hit = false`

12. James view tasks second time (cache hit):

```bash
curl -s -X GET http://127.0.0.1:8080/tasks/view-my-tasks \
  -H "Authorization: Bearer $JAMES_TOKEN" | jq
```

Expected: `summary.total_assigned_tasks = 3` and `cache.hit = true`

## Environment Variables
See `.env.example`:
- `HOST`
- `PORT`
- `MONGO_URI`
- `MONGO_DB_NAME`
- `REDIS_URL`
- `JWT_SECRET`

## Notes
- This is local-development focused.
- `GET /dev/email-logs/latest` is a dev-only helper endpoint.
- 2FA codes are also printed to console for local verification.

## Tests
Run fast unit tests:

```bash
cargo test
```

Run full assignment e2e validation test (requires running API + Mongo + Redis):

```bash
cargo test -- --ignored
```

## Final Validation Response
Paste your final `GET /tasks/view-my-tasks` James Bond response here after local run.
