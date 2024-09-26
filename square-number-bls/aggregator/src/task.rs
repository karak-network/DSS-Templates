use std::{
    collections::{HashMap, HashSet},
    fs,
    sync::{Arc, PoisonError},
    time::Duration,
};

use alloy::{
    primitives::{keccak256, Address, Uint, U256},
    providers::Provider,
    rpc::types::{BlockNumberOrTag, Filter},
    sol_types::{SolEvent, SolValue},
    transports::http::Client,
};
use ark_ff::PrimeField;
use chrono::{DateTime, Utc};
use eyre::Result;
use karak_rs::{
    bls::{keypair_signer::verify_signature, signature::Signature},
    kms::keypair::bn254::{
        algebra::{g1::G1Point, g2::G2Point},
        PublicKey,
    },
};
use serde::{Deserialize, Serialize};
use tokio::{
    signal,
    time::{self},
};
use tracing::{error, info};

use crate::{
    aggregator::{Operator, OperatorState},
    contract::{ContractManager, SquareNumberDSS},
    Config, TaskError,
};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Task {
    pub value: U256,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskRequest {
    pub task: Task,
    pub block_number: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletedTask {
    pub value: U256,
    pub response: U256,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskResponse {
    pub completed_task: CompletedTask,
    pub public_key: Address,
    pub bls_pubkey: PublicKey,
    pub signature: Signature,
}

#[derive(Serialize, Deserialize)]
pub struct BlockNumberData {
    pub block_number: u64,
}

pub struct TaskService {
    contract_manager: ContractManager,
    operator_state: Arc<OperatorState>,
    square_number_address: Address,
    block_number_store: String,
    block_number: u64,
    client: Client,
    heartbeat_interval: Duration,
}

impl From<G1Point> for SquareNumberDSS::G1Point {
    fn from(karak_point: G1Point) -> Self {
        let x = karak_point.0.x.into_bigint().0;
        let y = karak_point.0.y.into_bigint().0;

        let x_uint = Uint::<256, 4>::from(U256::from_limbs(x));
        let y_uint = Uint::<256, 4>::from(U256::from_limbs(y));

        SquareNumberDSS::G1Point {
            X: x_uint,
            Y: y_uint,
        }
    }
}

impl From<G2Point> for SquareNumberDSS::G2Point {
    fn from(karak_point: G2Point) -> Self {
        let x = karak_point.0.x;
        let y = karak_point.0.y;

        let x0_uint = Uint::<256, 4>::from(U256::from_limbs(x.c0.into_bigint().0));
        let x1_uint = Uint::<256, 4>::from(U256::from_limbs(x.c1.into_bigint().0));
        let y0_uint = Uint::<256, 4>::from(U256::from_limbs(y.c0.into_bigint().0));
        let y1_uint = Uint::<256, 4>::from(U256::from_limbs(y.c1.into_bigint().0));

        SquareNumberDSS::G2Point {
            X: [x1_uint, x0_uint], // Reversed x coordinates
            Y: [y1_uint, y0_uint], // Reversed y coordinates
        }
    }
}

impl TaskService {
    pub fn new(operator_state: Arc<OperatorState>, config: Config) -> Result<Self> {
        let contract_manager = ContractManager::new(&config)?;
        let square_number_address = config.square_number_dss_address;
        let block_number_store = config.block_number_store.clone();
        let block_number: u64 = config.load_block_number()?;
        let heartbeat_interval = Duration::from_millis(config.heartbeat);
        let client = Client::new();
        Ok(Self {
            contract_manager,
            operator_state,
            square_number_address,
            block_number_store,
            block_number,
            client,
            heartbeat_interval,
        })
    }

    pub async fn start(self: Arc<Self>) {
        info!("Listening for task request events");

        let heartbeat_interval = self.heartbeat_interval;

        tokio::spawn(async move {
            let mut interval = time::interval(heartbeat_interval);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = self.watch_for_task_events().await {
                            error!("Failed to watch for task events: {e}");
                        }
                    }
                    _ = signal::ctrl_c() => {
                        info!("Received shutdown signal. Stopping the aggregator...");
                        break;
                    }
                }
            }

            info!("Aggregator service stopped gracefully.");
        });
    }

    async fn watch_for_task_events(&self) -> Result<()> {
        let square_number_address = self.square_number_address;
        let next_block_to_check: u64 = self.block_number;
        let filter = Filter::new()
            .address(square_number_address)
            .from_block(BlockNumberOrTag::Number(next_block_to_check));

        let operators = match self.operator_state.operators.read() {
            Ok(guard) => guard.clone(),
            Err(PoisonError { .. }) => {
                error!("Failed to acquire read lock on operator state");
                return Err(eyre::anyhow!(
                    "Failed to acquire read lock on operator state"
                ));
            }
        };
        let logs = self.contract_manager.provider.get_logs(&filter).await?;
        let mut new_last_checked_block = next_block_to_check;

        for log in logs {
            if let Some(&SquareNumberDSS::TaskRequestGenerated::SIGNATURE_HASH) = log.topic0() {
                let SquareNumberDSS::TaskRequestGenerated {
                    sender: _,
                    taskRequest,
                } = log.log_decode()?.inner.data;
                let task = Task {
                    value: taskRequest.value,
                };
                let block_number = log.block_number.expect("Invalid block number");
                let task_request = TaskRequest { task, block_number };

                if !operators.is_empty() {
                    let (response, non_signing_operators, agg_pubkey, agg_sign) = self
                        .send_task_to_all_operators_and_aggregate(task_request.task, &operators)
                        .await?;

                    let task_response = SquareNumberDSS::TaskResponse { response };
                    let dss_task_request = SquareNumberDSS::TaskRequest {
                        value: task_request.task.value,
                    };

                    let non_signing_operators_converted = non_signing_operators
                        .into_iter()
                        .map(SquareNumberDSS::G1Point::from)
                        .collect();

                    match self
                        .contract_manager
                        .submit_task_response(
                            dss_task_request,
                            task_response,
                            non_signing_operators_converted,
                            agg_pubkey.into(),
                            agg_sign.into(),
                        )
                        .await
                    {
                        Ok(tx) => info!("Transaction sent: {:?}", tx),
                        Err(e) => error!("Failed to send transaction: {:?}", e),
                    }
                    new_last_checked_block =
                        new_last_checked_block.max(task_request.block_number + 1);
                } else {
                    info!("No operators are registered or no task requests were found.");
                }
            }
        }
        let _ = self
            .write_block_number_to_file(&self.block_number_store, new_last_checked_block)
            .await;

        Ok(())
    }

    fn aggregate_points_g1(&self, point_one: G1Point, point_two: G1Point) -> G1Point {
        point_one + point_two
    }

    fn aggregate_points_g2(&self, point_one: G2Point, point_two: G2Point) -> G2Point {
        point_one + point_two
    }

    async fn send_task_to_all_operators_and_aggregate(
        &self,
        task: Task,
        operators: &HashSet<Operator>,
    ) -> Result<(U256, Vec<G1Point>, G2Point, G1Point), TaskError> {
        let mut operator_responses = Vec::new();
        let mut signing_operators = HashSet::new();

        // Collect responses from operators
        for operator in operators.iter() {
            let operator = operator.clone();
            let res = self
                .client
                .post(format!("{}operator/task", operator.url()))
                .header("Content-Type", "application/json")
                .json(&task)
                .send()
                .await;

            match res {
                Ok(response) => {
                    if let Ok(body) = response.text().await {
                        let response_json: Result<TaskResponse, _> = serde_json::from_str(&body);
                        if let Ok(response_json) = response_json {
                            operator_responses.push((operator.clone(), response_json));
                        } else {
                            error!("Error parsing JSON from {}.", operator.url());
                        }
                    } else {
                        error!("Error reading response body from {}.", operator.url());
                    }
                }
                Err(e) => error!("Error sending task to {}: {:?}", operator.url(), e),
            }
        }

        let mut verified_responses = Vec::new();
        for (_operator, response) in &operator_responses {
            let public_key = response.public_key;
            if self
                .verify_message(response)
                .await
                .map_err(|_| TaskError::TaskVerificationFailed)?
            {
                verified_responses.push(response);
                signing_operators.insert(public_key);
            }
        }

        let non_signing_public_keys: Vec<G1Point> = operators
            .iter()
            .filter_map(|operator| {
                if !signing_operators.contains(operator.public_key()) {
                    operator_responses
                        .iter()
                        .find(|(_, response)| response.public_key == *operator.public_key())
                        .map(|(_, response)| response.bls_pubkey.g1)
                } else {
                    None
                }
            })
            .collect();

        let mut response_map: HashMap<U256, Vec<G1Point>> = HashMap::new();
        let mut g2_points = Vec::new();

        for response in &verified_responses {
            let response_value: U256 = response.completed_task.response;

            let signature = response.signature;

            g2_points.push(response.bls_pubkey.g2);

            response_map
                .entry(response_value)
                .or_default()
                .push(signature);
        }

        let (response_value, signatures) = response_map
            .into_iter()
            .max_by_key(|(_, sigs)| sigs.len())
            .ok_or(TaskError::TaskVerificationFailed)?;

        if signatures.len() <= operators.len() / 2 {
            return Err(TaskError::MajorityNotReached);
        }

        let aggregated_signature = signatures
            .into_iter()
            .reduce(|acc, sig| self.aggregate_points_g1(acc, sig))
            .ok_or(TaskError::TaskVerificationFailed)?;

        let aggregated_g2_points = g2_points
            .into_iter()
            .reduce(|acc, sig| self.aggregate_points_g2(acc, sig))
            .ok_or(TaskError::TaskVerificationFailed)?;

        Ok((
            response_value,
            non_signing_public_keys,
            aggregated_g2_points,
            aggregated_signature,
        ))
    }

    async fn verify_message(&self, task_response: &TaskResponse) -> Result<bool> {
        let encoded_msg = SquareNumberDSS::TaskResponse {
            response: task_response.completed_task.response,
        }
        .abi_encode();
        let msg_hash = keccak256(&encoded_msg);
        Ok(verify_signature(
            &task_response.bls_pubkey.g2,
            &task_response.signature,
            msg_hash,
        )
        .is_ok())
    }

    async fn write_block_number_to_file(&self, file: &str, val: u64) -> Result<()> {
        let data = BlockNumberData { block_number: val };

        let json_data = serde_json::to_string_pretty(&data)?;
        fs::write(file, json_data)?;

        Ok(())
    }
}
