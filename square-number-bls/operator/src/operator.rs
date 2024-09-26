use alloy::primitives::{keccak256, Address, U256};
use alloy::signers::local::PrivateKeySigner;
use alloy::sol_types::SolValue;
use axum::{extract::State, routing::post, Json, Router};
use chrono::{DateTime, Utc};
use eyre::Result;
use karak_rs::bls::keypair_signer::Signer;
use karak_rs::kms::keypair::traits::Keypair;
use karak_rs::{
    bls::{keypair_signer::KeypairSigner, signature::Signature},
    kms::keypair::bn254::PublicKey,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::contract::SquareNumberDSS;
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
    pub bls_pubkey: PublicKey,
    pub signature: Signature,
}

#[derive(Clone)]
pub struct TaskState {
    pub bls_signer: Arc<KeypairSigner>,
    pub wallet: Arc<PrivateKeySigner>,
    pub bls_pubkey: PublicKey,
}

impl TaskState {
    pub fn new(keypair: karak_rs::kms::keypair::bn254::Keypair, wallet: PrivateKeySigner) -> Self {
        let bls_signer = KeypairSigner::from(keypair.clone());
        let bls_pubkey = keypair.public_key();
        Self {
            bls_signer: Arc::new(bls_signer),
            wallet: Arc::new(wallet),
            bls_pubkey: bls_pubkey.clone(),
        }
    }

    pub async fn handle_task(&self, task: Task) -> Result<TaskResponse, TaskError> {
        let completed_task = self.run_task(task).await?;

        let signature = self.sign_object(&completed_task)?;

        Ok(TaskResponse {
            completed_task,
            public_key: self.wallet.address(),
            bls_pubkey: self.bls_pubkey.clone(),
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

    pub fn sign_object(&self, completed_task: &CompletedTask) -> Result<Signature, TaskError> {
        let task_response = SquareNumberDSS::TaskResponse {
            response: completed_task.response,
        };
        let encoded_msg = task_response.abi_encode();
        let msg_hash = keccak256(&encoded_msg);

        Ok(self.bls_signer.sign_message(msg_hash).unwrap())
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

pub fn operator_router(
    keypair: karak_rs::kms::keypair::bn254::Keypair,
    wallet: PrivateKeySigner,
) -> Router {
    Router::new()
        .route("/task", post(request_task))
        .with_state(TaskState::new(keypair, wallet))
}
