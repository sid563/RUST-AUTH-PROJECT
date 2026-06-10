use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use chrono::{Duration, Utc};
use mongodb::bson::{doc, oid::ObjectId, DateTime};
use rand::Rng;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use futures::TryStreamExt;

use crate::{
    jwt::{decode_access_token, issue_access_token},
    models::{EmailLog, LoginChallenge, Task, TaskPriority, TaskStatus, User, UserRole},
    security::{hash_password, verify_password},
    state::{AppState, DevEmailEvent},
};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct Verify2faRequest {
    pub login_challenge_id: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct LatestEmailQuery {
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
}

#[derive(Debug, Deserialize)]
pub struct AssignTasksRequest {
    pub task_ids: Vec<String>,
    pub assignee_email: String,
}

#[derive(Debug, Clone)]
struct AuthUser {
    id: ObjectId,
    email: String,
    role: UserRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ViewTaskItem {
    id: String,
    title: String,
    status: String,
    priority: String,
    assigned_to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ViewMyTasksResponse {
    user: serde_json::Value,
    tasks: Vec<ViewTaskItem>,
    summary: serde_json::Value,
    cache: serde_json::Value,
}

fn status_as_str(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "todo",
        TaskStatus::InProgress => "inprogress",
        TaskStatus::Done => "done",
    }
}

fn priority_as_str(priority: &TaskPriority) -> &'static str {
    match priority {
        TaskPriority::High => "high",
        TaskPriority::Medium => "medium",
        TaskPriority::Low => "low",
    }
}

fn parse_role(role: &str) -> Option<UserRole> {
    match role {
        "admin" => Some(UserRole::Admin),
        "staff" => Some(UserRole::Staff),
        _ => None,
    }
}

fn extract_bearer_token(req: &HttpRequest) -> Option<String> {
    let header = req.headers().get("Authorization")?;
    let value = header.to_str().ok()?;
    value.strip_prefix("Bearer ").map(ToString::to_string)
}

fn auth_user_from_request(req: &HttpRequest, state: &AppState) -> Result<AuthUser, HttpResponse> {
    let token = extract_bearer_token(req).ok_or_else(|| {
        HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "missing or invalid authorization header"
        }))
    })?;

    let claims = decode_access_token(&token, &state.jwt_secret).map_err(|_| {
        HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "invalid access token"
        }))
    })?;

    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_| {
        HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "invalid token subject"
        }))
    })?;

    let role = parse_role(&claims.role).ok_or_else(|| {
        HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "invalid token role"
        }))
    })?;

    Ok(AuthUser {
        id: user_id,
        email: claims.email,
        role,
    })
}

async fn invalidate_user_tasks_cache(state: &AppState, user_id: &ObjectId) -> anyhow::Result<()> {
    let mut conn = state.redis_client.get_multiplexed_async_connection().await?;
    let key = format!("tasks:view:{}", user_id.to_hex());
    let _: usize = conn.del(key).await?;
    Ok(())
}

#[get("/health")]
pub async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

#[post("/seed/users")]
pub async fn seed_users(state: web::Data<AppState>) -> impl Responder {
    let users_collection = state.db.collection::<User>("users");

    let seed_inputs = [
        ("Admin", "admin@example.com", "Admin@123", UserRole::Admin),
        (
            "James Bond",
            "jamesbond@example.com",
            "Bond@123",
            UserRole::Staff,
        ),
    ];

    let mut inserted = Vec::new();

    for (full_name, email, raw_password, role) in seed_inputs {
        let existing = users_collection
            .find_one(doc! { "email": email })
            .await
            .map_err(|e| {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("db error checking existing user: {e}")
                }))
            });

        let Ok(existing) = existing else {
            return existing.err().unwrap();
        };

        if existing.is_some() {
            continue;
        }

        let password_hash = match hash_password(raw_password) {
            Ok(v) => v,
            Err(e) => {
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("hashing error: {e}")
                }));
            }
        };

        let now = DateTime::now();
        let user = User {
            id: None,
            full_name: full_name.to_string(),
            email: email.to_string(),
            password_hash,
            role,
            created_at: now,
            updated_at: now,
        };

        if let Err(e) = users_collection.insert_one(user).await {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("insert user error: {e}")
            }));
        }

        inserted.push(email);
    }

    HttpResponse::Ok().json(serde_json::json!({
        "message": "seed completed",
        "seeded_users": [
            {"email": "admin@example.com", "password": "Admin@123", "role": "admin"},
            {"email": "jamesbond@example.com", "password": "Bond@123", "role": "staff"}
        ],
        "newly_inserted": inserted
    }))
}

#[post("/auth/login")]
pub async fn login(state: web::Data<AppState>, body: web::Json<LoginRequest>) -> impl Responder {
    let users_collection = state.db.collection::<User>("users");
    let challenges_collection = state.db.collection::<LoginChallenge>("login_challenges");
    let email_logs_collection = state.db.collection::<EmailLog>("email_logs");

    let user = match users_collection
        .find_one(doc! { "email": &body.email.to_lowercase() })
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "invalid email or password"
            }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("db error: {e}")
            }));
        }
    };

    let is_valid_password = match verify_password(&body.password, &user.password_hash) {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("password verification error: {e}")
            }));
        }
    };

    if !is_valid_password {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "invalid email or password"
        }));
    }

    let mut rng = rand::rng();
    let code = format!("{:06}", rng.random_range(0..1_000_000));
    let code_hash = match hash_password(&code) {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("failed to hash verification code: {e}")
            }));
        }
    };

    let now = Utc::now();
    let expires_at = now + Duration::minutes(5);

    let Some(user_id) = user.id else {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "user id missing"
        }));
    };

    let challenge = LoginChallenge {
        id: None,
        user_id,
        code_hash,
        expires_at: DateTime::from_millis(expires_at.timestamp_millis()),
        used_at: None,
        created_at: DateTime::from_millis(now.timestamp_millis()),
    };

    let insert_result = match challenges_collection.insert_one(challenge).await {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("failed to create challenge: {e}")
            }));
        }
    };

    let Some(challenge_id) = insert_result.inserted_id.as_object_id() else {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "failed to read challenge id"
        }));
    };

    let masked_code = format!("****{}", &code[4..]);
    let email_log = EmailLog {
        id: None,
        to_email: user.email.clone(),
        purpose: "login_2fa".to_string(),
        masked_code,
        challenge_id,
        created_at: DateTime::from_millis(now.timestamp_millis()),
    };

    if let Err(e) = email_logs_collection.insert_one(email_log).await {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("failed to insert email log: {e}")
        }));
    }

    {
        let mut dev_events = state.dev_email_events.write().await;
        dev_events.insert(
            user.email.clone(),
            DevEmailEvent {
                to_email: user.email.clone(),
                code: code.clone(),
                challenge_id: challenge_id.to_hex(),
                created_at_iso: now.to_rfc3339(),
            },
        );
    }

    println!(
        "[DEV EMAIL] to={} purpose=login_2fa code={} challenge_id={}",
        user.email,
        code,
        challenge_id.to_hex()
    );

    HttpResponse::Ok().json(serde_json::json!({
        "message": "2fa challenge created",
        "login_challenge_id": challenge_id.to_hex(),
        "expires_in_seconds": 300
    }))
}

#[get("/dev/email-logs/latest")]
pub async fn dev_email_logs_latest(
    state: web::Data<AppState>,
    query: web::Query<LatestEmailQuery>,
) -> impl Responder {
    let dev_events = state.dev_email_events.read().await;

    if let Some(email) = &query.email {
        if let Some(event) = dev_events.get(email) {
            return HttpResponse::Ok().json(serde_json::json!({
                "to_email": event.to_email,
                "code": event.code,
                "challenge_id": event.challenge_id,
                "created_at": event.created_at_iso
            }));
        }

        return HttpResponse::NotFound().json(serde_json::json!({
            "error": "no dev email log found for the requested email"
        }));
    }

    let latest = dev_events.values().max_by(|a, b| a.created_at_iso.cmp(&b.created_at_iso));

    match latest {
        Some(event) => HttpResponse::Ok().json(serde_json::json!({
            "to_email": event.to_email,
            "code": event.code,
            "challenge_id": event.challenge_id,
            "created_at": event.created_at_iso
        })),
        None => HttpResponse::NotFound().json(serde_json::json!({
            "error": "no dev email logs available"
        })),
    }
}

#[post("/auth/verify-2fa")]
pub async fn verify_2fa(
    state: web::Data<AppState>,
    body: web::Json<Verify2faRequest>,
) -> impl Responder {
    let users_collection = state.db.collection::<User>("users");
    let challenges_collection = state.db.collection::<LoginChallenge>("login_challenges");

    let challenge_id = match ObjectId::parse_str(&body.login_challenge_id) {
        Ok(v) => v,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "invalid login_challenge_id"
            }));
        }
    };

    let challenge = match challenges_collection.find_one(doc! {"_id": challenge_id}).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "invalid or expired challenge"
            }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("db error loading challenge: {e}")
            }));
        }
    };

    if challenge.used_at.is_some() {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "verification code already used"
        }));
    }

    if challenge.expires_at < DateTime::now() {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "verification code expired"
        }));
    }

    let is_valid_code = match verify_password(&body.code, &challenge.code_hash) {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("code verification error: {e}")
            }));
        }
    };

    if !is_valid_code {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "incorrect verification code"
        }));
    }

    let now = DateTime::now();
    if let Err(e) = challenges_collection
        .update_one(
            doc! {
                "_id": challenge_id,
                "used_at": mongodb::bson::Bson::Null
            },
            doc! {
                "$set": {"used_at": now}
            },
        )
        .await
    {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("failed to update challenge: {e}")
        }));
    }

    let user = match users_collection
        .find_one(doc! {"_id": challenge.user_id})
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "user not found for challenge"
            }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("db error loading user: {e}")
            }));
        }
    };

    let Some(user_id) = user.id else {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "user id missing"
        }));
    };

    let token = match issue_access_token(&user_id, &user.email, &user.role, &state.jwt_secret) {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("jwt issue error: {e}")
            }));
        }
    };

    HttpResponse::Ok().json(serde_json::json!({
        "access_token": token,
        "token_type": "Bearer",
        "expires_in_seconds": 86400,
        "user": {
            "email": user.email,
            "role": user.role
        }
    }))
}

#[post("/tasks")]
pub async fn create_task(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreateTaskRequest>,
) -> impl Responder {
    let auth_user = match auth_user_from_request(&req, &state) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    if auth_user.role != UserRole::Admin {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "forbidden"
        }));
    }

    let tasks_collection = state.db.collection::<Task>("tasks");
    let now = DateTime::now();
    let task = Task {
        id: None,
        title: body.title.clone(),
        description: body.description.clone(),
        status: body.status.clone(),
        priority: body.priority.clone(),
        created_by_id: auth_user.id,
        assigned_to_id: None,
        created_at: now,
        updated_at: now,
    };

    let insert_result = match tasks_collection.insert_one(task).await {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("failed to create task: {e}")
            }));
        }
    };

    let task_id = insert_result
        .inserted_id
        .as_object_id()
        .map(|id| id.to_hex())
        .unwrap_or_default();

    HttpResponse::Ok().json(serde_json::json!({
        "message": "task created",
        "task_id": task_id
    }))
}

#[post("/tasks/assign")]
pub async fn assign_tasks(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<AssignTasksRequest>,
) -> impl Responder {
    let auth_user = match auth_user_from_request(&req, &state) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    if auth_user.role != UserRole::Admin {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "forbidden"
        }));
    }

    if body.task_ids.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "task_ids cannot be empty"
        }));
    }

    let users_collection = state.db.collection::<User>("users");
    let tasks_collection = state.db.collection::<Task>("tasks");

    let assignee = match users_collection
        .find_one(doc! {"email": body.assignee_email.to_lowercase()})
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "assignee user not found"
            }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("db error loading assignee: {e}")
            }));
        }
    };

    let Some(assignee_id) = assignee.id else {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "assignee id missing"
        }));
    };

    let task_object_ids = {
        let mut ids = Vec::with_capacity(body.task_ids.len());
        for raw_id in &body.task_ids {
            let parsed = match ObjectId::parse_str(raw_id) {
                Ok(v) => v,
                Err(_) => {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "error": format!("invalid task id: {raw_id}")
                    }));
                }
            };
            ids.push(parsed);
        }
        ids
    };

    let cursor = match tasks_collection
        .find(doc! {"_id": {"$in": &task_object_ids}})
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("db error loading tasks: {e}")
            }));
        }
    };

    let existing_tasks = match cursor.try_collect::<Vec<Task>>().await {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("failed to decode tasks: {e}")
            }));
        }
    };

    if existing_tasks.len() != task_object_ids.len() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "one or more task_ids do not exist"
        }));
    }

    let now = DateTime::now();
    if let Err(e) = tasks_collection
        .update_many(
            doc! {"_id": {"$in": &task_object_ids}},
            doc! {
                "$set": {
                    "assigned_to_id": assignee_id,
                    "updated_at": now,
                }
            },
        )
        .await
    {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("failed to assign tasks: {e}")
        }));
    }

    let mut users_to_invalidate = vec![assignee_id];
    for task in &existing_tasks {
        if let Some(prev_user_id) = &task.assigned_to_id {
            if !users_to_invalidate.contains(prev_user_id) {
                users_to_invalidate.push(*prev_user_id);
            }
        }
    }

    for user_id in &users_to_invalidate {
        if let Err(e) = invalidate_user_tasks_cache(&state, user_id).await {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("failed to invalidate cache: {e}")
            }));
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "message": "tasks assigned",
        "assigned_to": body.assignee_email,
        "task_count": task_object_ids.len()
    }))
}

#[get("/tasks/view-my-tasks")]
pub async fn view_my_tasks(req: HttpRequest, state: web::Data<AppState>) -> impl Responder {
    let auth_user = match auth_user_from_request(&req, &state) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let cache_key = format!("tasks:view:{}", auth_user.id.to_hex());
    let mut conn = match state.redis_client.get_multiplexed_async_connection().await {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("redis connection error: {e}")
            }));
        }
    };

    let cached_payload: Option<String> = match conn.get(&cache_key).await {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("redis get error: {e}")
            }));
        }
    };

    if let Some(cached) = cached_payload {
        let mut response: ViewMyTasksResponse = match serde_json::from_str(&cached) {
            Ok(v) => v,
            Err(e) => {
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("failed to decode cached payload: {e}")
                }));
            }
        };
        response.cache = serde_json::json!({"hit": true});
        return HttpResponse::Ok().json(response);
    }

    let tasks_collection = state.db.collection::<Task>("tasks");
    let cursor = match tasks_collection
        .find(doc! {"assigned_to_id": auth_user.id})
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("db error loading tasks: {e}")
            }));
        }
    };

    let mut tasks = match cursor.try_collect::<Vec<Task>>().await {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("failed to decode tasks: {e}")
            }));
        }
    };

    tasks.sort_by_key(|t| t.created_at);

    let response_tasks = tasks
        .iter()
        .map(|task| ViewTaskItem {
            id: task.id.map(|v| v.to_hex()).unwrap_or_default(),
            title: task.title.clone(),
            status: status_as_str(&task.status).to_string(),
            priority: priority_as_str(&task.priority).to_string(),
            assigned_to: auth_user.email.clone(),
        })
        .collect::<Vec<_>>();

    let response = ViewMyTasksResponse {
        user: serde_json::json!({
            "email": auth_user.email,
            "role": match auth_user.role {
                UserRole::Admin => "admin",
                UserRole::Staff => "staff",
            }
        }),
        tasks: response_tasks,
        summary: serde_json::json!({
            "total_assigned_tasks": tasks.len()
        }),
        cache: serde_json::json!({"hit": false}),
    };

    let encoded = match serde_json::to_string(&response) {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("failed to encode cache payload: {e}")
            }));
        }
    };

    let set_result: redis::RedisResult<()> = conn.set_ex(&cache_key, encoded, 300).await;
    if let Err(e) = set_result {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("redis set error: {e}")
        }));
    }

    HttpResponse::Ok().json(response)
}
