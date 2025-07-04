use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{common::scoped_task::ScopedTask, tunnel::TunnelConnector};

pub mod controller;
pub mod session;

pub struct WebClient {
    controller: Arc<controller::Controller>,
    tasks: ScopedTask<()>,
    sessions: Arc<Mutex<Vec<Arc<session::Session>>>>,
}

impl WebClient {
    pub fn new<T: TunnelConnector + 'static, S: ToString, H: ToString>(
        connector: T,
        token: S,
        hostname: H,
    ) -> Self {
        let controller = Arc::new(controller::Controller::new(
            token.to_string(),
            hostname.to_string(),
        ));

        let controller_clone = controller.clone();
        let sessions = Arc::new(Mutex::new(Vec::new()));
        let sessions_clone = sessions.clone();
        let tasks = ScopedTask::from(tokio::spawn(async move {
            Self::routine(
                controller_clone,
                Box::new(connector),
                sessions_clone,
            )
            .await;
        }));

        WebClient {
            controller,
            tasks,
            sessions,
        }
    }

    async fn routine(
        controller: Arc<controller::Controller>,
        mut connector: Box<dyn TunnelConnector>,
        sessions: Arc<Mutex<Vec<Arc<session::Session>>>>,
    ) {
        loop {
            let conn = match connector.connect().await {
                Ok(conn) => conn,
                Err(e) => {
                    println!(
                        "Failed to connect to the server ({}), retrying in 5 seconds...",
                        e
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }
            };

            println!("Successfully connected to {:?}", conn.info());

            let session = Arc::new(session::Session::new(conn, controller.clone()));

            sessions.lock().await.push(session.clone());

            session.wait().await;
        }
    }

    pub async fn shutdown(&self) {
        let sessions = self.sessions.lock().await;
        for session in sessions.iter() {
            session.shutdown().await;
        }
    }
}
