//! Method dispatcher.

use tract_proto::{Method, Request, Response, ResultBody, PROTO_VERSION};

use crate::state::AppState;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn dispatch(state: &AppState, req: Request) -> Response {
    let id = req.id;
    match req.method {
        Method::Ping => Response::ok(
            id,
            ResultBody::Pong {
                version: VERSION.to_string(),
                proto: PROTO_VERSION,
            },
        ),
        Method::Status => {
            let uptime_secs = state.started_at().elapsed().as_secs();
            let cache_entries = state.orchestrator().cache_count().await;
            Response::ok(
                id,
                ResultBody::Status {
                    version: VERSION.to_string(),
                    proto: PROTO_VERSION,
                    uptime_secs,
                    cache_entries,
                },
            )
        }
        Method::Fetch(params) => match state
            .orchestrator()
            .fetch(&params.url, params.options)
            .await
        {
            Ok(result) => Response::ok(id, ResultBody::Fetch(result)),
            Err(err) => Response::err(id, err),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tract_fetcher::{Cache, HttpClient};
    use tract_proto::{ErrorCode, FetchParams, Method};
    use uuid::Uuid;

    use crate::orchestrator::Orchestrator;

    fn state() -> AppState {
        AppState::new(Orchestrator::new(
            Cache::in_memory().unwrap(),
            HttpClient::new().unwrap(),
        ))
    }

    #[tokio::test]
    async fn ping_returns_pong() {
        let resp = dispatch(
            &state(),
            Request {
                id: Uuid::nil(),
                method: Method::Ping,
            },
        )
        .await;
        assert!(resp.ok);
        match resp.result {
            Some(ResultBody::Pong { proto, .. }) => assert_eq!(proto, PROTO_VERSION),
            _ => panic!("expected Pong"),
        }
    }

    #[tokio::test]
    async fn status_returns_cache_count() {
        let resp = dispatch(
            &state(),
            Request {
                id: Uuid::nil(),
                method: Method::Status,
            },
        )
        .await;
        assert!(resp.ok);
        match resp.result {
            Some(ResultBody::Status { cache_entries, .. }) => assert_eq!(cache_entries, 0),
            _ => panic!("expected Status"),
        }
    }

    #[tokio::test]
    async fn fetch_invalid_url_returns_typed_error() {
        let resp = dispatch(
            &state(),
            Request {
                id: Uuid::nil(),
                method: Method::Fetch(FetchParams {
                    url: "not a url at all".into(),
                    ..Default::default()
                }),
            },
        )
        .await;
        assert!(!resp.ok);
        assert_eq!(resp.error.unwrap().code, ErrorCode::InvalidUrl);
    }
}
