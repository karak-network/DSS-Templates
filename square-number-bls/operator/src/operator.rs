use alloy::primitives::{Address, U256};
use alloy::signers::Signature;
use alloy::signers::{local::PrivateKeySigner, Signer};
use axum::{extract::State, routing::post, Json, Router};
use chrono::{DateTime, Utc};
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::error::AppError;
use crate::TaskError;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub value: U256,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompletedTask {
    pub value: U256,
    pub response: U256,
    pub completed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskResponse {
    pub completed_task: CompletedTask,
    pub public_key: Address,
    pub signature: Signature,
}

#[derive(Clone, Debug)]
pub struct TaskState {
    pub wallet: Arc<PrivateKeySigner>,
}

impl TaskState {
    pub fn new(wallet: PrivateKeySigner) -> Self {
        Self {
            wallet: Arc::new(wallet),
        }
    }

    pub async fn handle_task(&self, task: Task) -> Result<TaskResponse, TaskError> {
        let completed_task = self.run_task(task).await?;

        let signature = self.sign_object(&completed_task).await?;

        Ok(TaskResponse {
            completed_task,
            public_key: self.wallet.address(),
            signature,
        })
    }

    async fn run_task(&self, task: Task) -> Result<CompletedTask, TaskError> {
        let task_response = task_computation(task.value);
        let completed_at = Utc::now();

        info!("Task completed at: {}", completed_at.to_rfc3339());

        Ok(CompletedTask {
            value: task.value,
            response: task_response,
            completed_at,
        })
    }

    pub async fn sign_object(
        &self,
        completed_task: &CompletedTask,
    ) -> Result<Signature, TaskError> {
        let data_string = serde_json::to_string(completed_task).unwrap();
        Ok(self.wallet.sign_message(data_string.as_bytes()).await?)
    }
}

fn task_computation(val: U256) -> U256 {
    val * val
}

pub async fn request_task(
    State(task_state): State<TaskState>,
    Json(task): Json<Task>,
) -> Result<Json<TaskResponse>, AppError> {
    info!("Handling task request: {:?}", task);

    match task_state.handle_task(task).await {
        Ok(task_response) => Ok(Json(task_response)),
        Err(err) => {
            error!("Error handling task request: {err}");
            Err(AppError::from(eyre::Report::new(err)))
        }
    }
}

pub fn operator_router(wallet: PrivateKeySigner) -> Router {
    Router::new()
        .route("/task", post(request_task))
        .with_state(TaskState::new(wallet))
}
