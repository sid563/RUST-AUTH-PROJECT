# AI Usage Disclosure

## Tools Used
- GitHub Copilot Chat

## How AI Was Used
- Generated and refined API scaffolding for Actix Web.
- Helped draft MongoDB models and endpoint handlers.
- Assisted with JWT, Argon2 hashing, and Redis cache integration.
- Helped produce README validation steps and environment setup notes.

## Manual Changes and Verification
- Manually reviewed endpoint behavior against assignment requirements.
- Manually validated route wiring, role checks, and 2FA workflow logic.
- Manually verified compile success with `cargo build`.
- Manual local end-to-end verification is required with real `.env` credentials.

## Candidate Understanding
- The candidate is expected to explain:
  - Why login returns challenge id before JWT
  - How code hashing and one-time challenge usage work
  - How RBAC is enforced for admin-only routes
  - How per-user task cache is stored and invalidated
