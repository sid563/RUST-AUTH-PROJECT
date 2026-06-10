# Local Validation Run

End-to-end validation of the README flow (steps 1–12), run locally against
MongoDB Atlas + a local Redis. All steps pass.

| Step | Action | Expected | Result |
|------|--------|----------|--------|
| 5  | Create 5 tasks (admin) | 5 task ids | ✅ 5 created |
| 6  | Assign 3 tasks to James | `task_count: 3` | ✅ assigned + cache invalidated |
| 7–9 | James login → 2FA → JWT | token, role `staff` | ✅ |
| 10 | James creates a task | **403 forbidden** | ✅ `HTTP 403` |
| 11 | view-my-tasks (1st) | `total=3`, `cache.hit=false` | ✅ |
| 12 | view-my-tasks (2nd) | `total=3`, `cache.hit=true` | ✅ |

---

## Step 5 — Create exactly 5 tasks (admin)

```json
{"message":"task created","task_id":"6a293b10c121b0a6bfc817ac"}   // Task-high
{"message":"task created","task_id":"6a293b10c121b0a6bfc817ad"}   // Task-medium
{"message":"task created","task_id":"6a293b11c121b0a6bfc817ae"}   // Task-low
{"message":"task created","task_id":"6a293b11c121b0a6bfc817af"}   // Task-high
{"message":"task created","task_id":"6a293b11c121b0a6bfc817b0"}   // Task-medium
```

## Step 6 — Assign 3 tasks to James Bond

```json
{"message":"tasks assigned","assigned_to":"jamesbond@example.com","task_count":3}
```

Assigned task ids:
`6a293b10c121b0a6bfc817ac`, `6a293b10c121b0a6bfc817ad`, `6a293b11c121b0a6bfc817ae`

## Step 7 — Login as James Bond

```json
{"message":"2fa challenge created","login_challenge_id":"6a293bcaa558de5218daef5e","expires_in_seconds":300}
```

## Step 8 — Fetch James Bond 2FA code (dev endpoint)

```json
{"to_email":"jamesbond@example.com","code":"557278","challenge_id":"6a293bcaa558de5218daef5e","created_at":"2026-06-10T10:26:18.375893+00:00"}
```

## Step 9 — Verify James Bond 2FA

```json
{"access_token":"<JAMES_TOKEN>","token_type":"Bearer","expires_in_seconds":86400,"user":{"email":"jamesbond@example.com","role":"staff"}}
```

## Step 10 — James tries to create a task (must be 403)

```
HTTP 403
{"error":"forbidden"}
```

## Step 11 — James view-my-tasks (cache miss)

```json
{
  "user": {"email": "jamesbond@example.com", "role": "staff"},
  "tasks": [
    {"id":"6a293b10c121b0a6bfc817ac","title":"Task-high","status":"todo","priority":"high","assigned_to":"jamesbond@example.com"},
    {"id":"6a293b10c121b0a6bfc817ad","title":"Task-medium","status":"todo","priority":"medium","assigned_to":"jamesbond@example.com"},
    {"id":"6a293b11c121b0a6bfc817ae","title":"Task-low","status":"todo","priority":"low","assigned_to":"jamesbond@example.com"}
  ],
  "summary": {"total_assigned_tasks": 3},
  "cache": {"hit": false}
}
```

## Step 12 — James view-my-tasks again (cache hit)

```json
{
  "user": {"email": "jamesbond@example.com", "role": "staff"},
  "tasks": [
    {"id":"6a293b10c121b0a6bfc817ac","title":"Task-high","status":"todo","priority":"high","assigned_to":"jamesbond@example.com"},
    {"id":"6a293b10c121b0a6bfc817ad","title":"Task-medium","status":"todo","priority":"medium","assigned_to":"jamesbond@example.com"},
    {"id":"6a293b11c121b0a6bfc817ae","title":"Task-low","status":"todo","priority":"low","assigned_to":"jamesbond@example.com"}
  ],
  "summary": {"total_assigned_tasks": 3},
  "cache": {"hit": true}
}
```

---

## Local Environment Notes

- **API**: Actix Web on `127.0.0.1:8080`
- **MongoDB**: Atlas cluster, db `task_auth_db`
- **Redis**: local container on port `6380` (no auth) — `REDIS_URL=redis://127.0.0.1:6380`
- Secrets live in `.env` (gitignored).
