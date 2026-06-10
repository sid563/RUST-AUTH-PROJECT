use reqwest::StatusCode;
use serde_json::{json, Value};

fn api_base_url() -> String {
    std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string())
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

async fn post_json(client: &reqwest::Client, url: &str, payload: Value) -> reqwest::Response {
    client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .expect("request should succeed")
}

#[tokio::test]
#[ignore = "Requires running API, MongoDB/Atlas config, and Redis. Run with: cargo test -- --ignored"]
async fn validation_flow_admin_and_james_bond() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let seed_resp = post_json(&client, &format!("{base}/seed/users"), json!({})).await;
    assert_eq!(seed_resp.status(), StatusCode::OK);

    let admin_login_resp = post_json(
        &client,
        &format!("{base}/auth/login"),
        json!({"email": "admin@example.com", "password": "Admin@123"}),
    )
    .await;
    assert_eq!(admin_login_resp.status(), StatusCode::OK);
    let admin_login_json: Value = admin_login_resp
        .json()
        .await
        .expect("admin login json should parse");
    let admin_challenge_id = admin_login_json["login_challenge_id"]
        .as_str()
        .expect("admin challenge id should exist")
        .to_string();

    let admin_code_resp = client
        .get(format!("{base}/dev/email-logs/latest?email=admin@example.com"))
        .send()
        .await
        .expect("admin code request should succeed");
    assert_eq!(admin_code_resp.status(), StatusCode::OK);
    let admin_code_json: Value = admin_code_resp
        .json()
        .await
        .expect("admin code json should parse");
    let admin_code = admin_code_json["code"]
        .as_str()
        .expect("admin code should exist")
        .to_string();

    let admin_verify_resp = post_json(
        &client,
        &format!("{base}/auth/verify-2fa"),
        json!({
            "login_challenge_id": admin_challenge_id,
            "code": admin_code
        }),
    )
    .await;
    assert_eq!(admin_verify_resp.status(), StatusCode::OK);
    let admin_verify_json: Value = admin_verify_resp
        .json()
        .await
        .expect("admin verify json should parse");
    let admin_token = admin_verify_json["access_token"]
        .as_str()
        .expect("admin token should exist")
        .to_string();

    let mut created_task_ids = Vec::new();
    let priorities = ["high", "medium", "low", "high", "medium"];
    for (idx, priority) in priorities.iter().enumerate() {
        let create_resp = client
            .post(format!("{base}/tasks"))
            .header("Authorization", bearer(&admin_token))
            .json(&json!({
                "title": format!("Validation Task {}", idx + 1),
                "description": "Created from e2e test",
                "status": "todo",
                "priority": priority,
            }))
            .send()
            .await
            .expect("task create request should succeed");
        assert_eq!(create_resp.status(), StatusCode::OK);
        let create_json: Value = create_resp
            .json()
            .await
            .expect("task create json should parse");
        let task_id = create_json["task_id"]
            .as_str()
            .expect("task id should exist")
            .to_string();
        created_task_ids.push(task_id);
    }

    let assign_resp = client
        .post(format!("{base}/tasks/assign"))
        .header("Authorization", bearer(&admin_token))
        .json(&json!({
            "task_ids": created_task_ids[0..3].to_vec(),
            "assignee_email": "jamesbond@example.com"
        }))
        .send()
        .await
        .expect("task assign request should succeed");
    assert_eq!(assign_resp.status(), StatusCode::OK);

    let james_login_resp = post_json(
        &client,
        &format!("{base}/auth/login"),
        json!({"email": "jamesbond@example.com", "password": "Bond@123"}),
    )
    .await;
    assert_eq!(james_login_resp.status(), StatusCode::OK);
    let james_login_json: Value = james_login_resp
        .json()
        .await
        .expect("james login json should parse");
    let james_challenge_id = james_login_json["login_challenge_id"]
        .as_str()
        .expect("james challenge id should exist")
        .to_string();

    let james_code_resp = client
        .get(format!("{base}/dev/email-logs/latest?email=jamesbond@example.com"))
        .send()
        .await
        .expect("james code request should succeed");
    assert_eq!(james_code_resp.status(), StatusCode::OK);
    let james_code_json: Value = james_code_resp
        .json()
        .await
        .expect("james code json should parse");
    let james_code = james_code_json["code"]
        .as_str()
        .expect("james code should exist")
        .to_string();

    let james_verify_resp = post_json(
        &client,
        &format!("{base}/auth/verify-2fa"),
        json!({
            "login_challenge_id": james_challenge_id,
            "code": james_code
        }),
    )
    .await;
    assert_eq!(james_verify_resp.status(), StatusCode::OK);
    let james_verify_json: Value = james_verify_resp
        .json()
        .await
        .expect("james verify json should parse");
    let james_token = james_verify_json["access_token"]
        .as_str()
        .expect("james token should exist")
        .to_string();

    let forbidden_create_resp = client
        .post(format!("{base}/tasks"))
        .header("Authorization", bearer(&james_token))
        .json(&json!({
            "title": "Should fail",
            "description": "staff cannot create",
            "status": "todo",
            "priority": "low"
        }))
        .send()
        .await
        .expect("staff create task call should succeed");
    assert_eq!(forbidden_create_resp.status(), StatusCode::FORBIDDEN);

    let first_view_resp = client
        .get(format!("{base}/tasks/view-my-tasks"))
        .header("Authorization", bearer(&james_token))
        .send()
        .await
        .expect("first view-my-tasks should succeed");
    assert_eq!(first_view_resp.status(), StatusCode::OK);
    let first_view_json: Value = first_view_resp
        .json()
        .await
        .expect("first view json should parse");
    assert_eq!(first_view_json["summary"]["total_assigned_tasks"], 3);
    assert_eq!(first_view_json["cache"]["hit"], false);

    let second_view_resp = client
        .get(format!("{base}/tasks/view-my-tasks"))
        .header("Authorization", bearer(&james_token))
        .send()
        .await
        .expect("second view-my-tasks should succeed");
    assert_eq!(second_view_resp.status(), StatusCode::OK);
    let second_view_json: Value = second_view_resp
        .json()
        .await
        .expect("second view json should parse");
    assert_eq!(second_view_json["summary"]["total_assigned_tasks"], 3);
    assert_eq!(second_view_json["cache"]["hit"], true);
}
