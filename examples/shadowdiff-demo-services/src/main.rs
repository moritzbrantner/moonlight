use axum::{extract::State, http::StatusCode, routing::any, Json, Router};
use serde_json::{json, Value};
use std::{env, net::SocketAddr, time::Duration};
use tokio::net::TcpListener;
use uuid::Uuid;

#[derive(Clone, Copy)]
enum Role {
    Primary,
    Candidate,
    Secondary,
}

impl Role {
    fn from_arg(value: &str) -> anyhow::Result<Self> {
        match value {
            "primary" => Ok(Self::Primary),
            "candidate" => Ok(Self::Candidate),
            "secondary" => Ok(Self::Secondary),
            other => anyhow::bail!(
                "unknown demo service role {other}; use primary, candidate, or secondary"
            ),
        }
    }

    fn port(self) -> u16 {
        match self {
            Self::Primary => 3001,
            Self::Candidate => 3002,
            Self::Secondary => 3003,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Candidate => "candidate",
            Self::Secondary => "secondary",
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let role = env::args()
        .nth(1)
        .map(|value| Role::from_arg(&value))
        .transpose()?
        .unwrap_or(Role::Primary);
    let port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| role.port());
    let addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;
    let app = Router::new().fallback(any(handler)).with_state(role);
    let listener = TcpListener::bind(addr).await?;
    println!(
        "shadowdiff demo {} listening on http://{addr}",
        role.as_str()
    );
    axum::serve(listener, app).await?;
    Ok(())
}

async fn handler(State(role): State<Role>, uri: axum::http::Uri) -> (StatusCode, Json<Value>) {
    match uri.path() {
        "/success" => ok(payload(
            role,
            "success",
            json!({ "value": 42, "stable": true }),
        )),
        "/regression" => {
            let value = if matches!(role, Role::Candidate) {
                43
            } else {
                42
            };
            ok(payload(
                role,
                "regression",
                json!({ "value": value, "stable": true }),
            ))
        }
        "/noise" => {
            let region = if matches!(role, Role::Secondary) {
                "eu-secondary"
            } else {
                "eu-primary"
            };
            ok(payload(
                role,
                "noise",
                json!({ "value": 42, "region": region }),
            ))
        }
        "/noisy-regression" => {
            let region = if matches!(role, Role::Secondary) {
                "eu-secondary"
            } else {
                "eu-primary"
            };
            let total = if matches!(role, Role::Candidate) {
                99
            } else {
                42
            };
            ok(payload(
                role,
                "noisy-regression",
                json!({ "total": total, "region": region }),
            ))
        }
        "/status-regression" if matches!(role, Role::Candidate) => (
            StatusCode::BAD_REQUEST,
            Json(payload(
                role,
                "status-regression",
                json!({ "error": "candidate status changed" }),
            )),
        ),
        "/status-regression" => ok(payload(role, "status-regression", json!({ "ok": true }))),
        "/slow-candidate" if matches!(role, Role::Candidate) => {
            tokio::time::sleep(Duration::from_millis(650)).await;
            ok(payload(
                role,
                "slow-candidate",
                json!({ "value": 42, "slow": true }),
            ))
        }
        "/slow-candidate" => ok(payload(
            role,
            "slow-candidate",
            json!({ "value": 42, "slow": true }),
        )),
        _ => (
            StatusCode::NOT_FOUND,
            Json(json!({ "role": role.as_str(), "error": "not found", "path": uri.path() })),
        ),
    }
}

fn ok(value: Value) -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(value))
}

fn payload(role: Role, endpoint: &str, mut body: Value) -> Value {
    let map = body.as_object_mut().expect("demo payloads are objects");
    map.insert("endpoint".to_string(), json!(endpoint));
    map.insert("timestamp".to_string(), json!("2026-06-13T07:44:00Z"));
    map.insert("requestId".to_string(), json!(Uuid::new_v4().to_string()));
    if matches!(role, Role::Secondary) && endpoint == "noise" {
        map.insert("servedBy".to_string(), json!("secondary-pool"));
    }
    body
}
