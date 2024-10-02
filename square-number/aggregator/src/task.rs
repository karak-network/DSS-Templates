use std::{
    collections::{HashMap, HashSet},
    fs,
    sync::{Arc, PoisonError},
    time::Duration,
};

use alloy::{
    primitives::{Address, Uint, U256},
    providers::Provider,
    rpc::types::{BlockNumberOrTag, Filter},
    signers::Signature,
    sol_types::SolEvent,
    transports::http::Client,
};
use chrono::{DateTime, Utc};
use eyre::Result;
use serde::{Deserialize, Serialize};
use tokio::{
    signal,
    time::{self},
};
use tracing::{error, info};
use url::Url;

use crate::{
    aggregator::{Operator, OperatorState},
    contract::{ContractManager, SquareNumberDSS, VaultContract},
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
    dss_address: Address,
    block_number_store: String,
    block_number: u64,
    rpc_url: Url,
    private_key: alloy::signers::local::PrivateKeySigner,
    client: Client,
    heartbeat_interval: Duration,
}

impl TaskService {
    pub fn new(operator_state: Arc<OperatorState>, config: Config) -> Result<Self> {
        let contract_manager = ContractManager::new(&config)?;
        let square_number_address = config.square_number_dss_address;
        let dss_address = config.square_number_dss_address;
        let block_number_store = config.block_number_store.clone();
        let block_number: u64 = config.load_block_number()?;
        let rpc_url = config.get_rpc_url()?;
        let private_key = config.get_private_key()?;
        let heartbeat_interval = Duration::from_millis(config.heartbeat);
        let client = Client::new();
        Ok(Self {
            contract_manager,
            operator_state,
            square_number_address,
            dss_address,
            block_number_store,
            block_number,
            rpc_url,
            private_key,
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
                    let response = self
                        .send_task_to_all_operators(task_request.task, &operators)
                        .await?;

                    let task_response = SquareNumberDSS::TaskResponse { response };
                    let dss_task_request = SquareNumberDSS::TaskRequest {
                        value: task_request.task.value,
                    };
                    match self
                        .contract_manager
                        .submit_task_response(dss_task_request, task_response)
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

    async fn get_operator_stake_normalized_eth(
        &self,
        operator: Address,
    ) -> Result<U256, TaskError> {
        let vaults = self
            .contract_manager
            .fetch_vaults_staked_in_dss(operator, self.dss_address)
            .await?;

        let mut stake = Uint::from(0u64);

        for vault in vaults {
            let total_assets =
                VaultContract::new(self.rpc_url.clone(), self.private_key.clone(), vault)?
                    .vault_instance
                    .totalAssets()
                    .call()
                    .await
                    .map_err(|_| TaskError::ContractCallError)?
                    ._0;

            // TODO: Normalize total assets to ETH
            stake += total_assets;
        }

        Ok(stake)
    }

    async fn get_operator_stake_mapping(
        &self,
        operators: Vec<Address>,
        min_acceptable_stake: U256,
    ) -> Result<(HashMap<Address, U256>, U256), TaskError> {
        let mut stake_mapping = HashMap::new();
        let mut total_stake = Uint::from(0u64);

        for operator in operators {
            let stake = self
                .get_operator_stake_normalized_eth(operator)
                .await
                .map_err(TaskError::from)?;

            if stake > min_acceptable_stake {
                stake_mapping.insert(operator, stake);
                total_stake += stake;
            }
        }

        Ok((stake_mapping, total_stake))
    }

    async fn send_task_to_all_operators(
        &self,
        task: Task,
        operators: &HashSet<Operator>,
    ) -> Result<U256, TaskError> {
        let mut operator_responses = Vec::new();

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
                    let body_result = response.text().await;

                    match body_result {
                        Ok(body) => {
                            let response_json_result: Result<TaskResponse, serde_json::Error> =
                                serde_json::from_str(&body);
                            match response_json_result {
                                Ok(response_json) => {
                                    operator_responses.push(response_json);
                                }
                                Err(e) => {
                                    error!(
                                        "Error parsing JSON response from {}: {:?}",
                                        operator.url(),
                                        e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            error!(
                                "Error reading response body from {}: {:?}",
                                operator.url(),
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    error!("Error sending task to {}: {:?}", operator.url(), e);
                }
            }
        }

        info!("op_RES_LEN: {}", operator_responses.len());

        let mut verified_responses = Vec::new();

        for response in operator_responses {
            let is_verified = self.verify_message(&response).await.map_err(|e| {
                error!("Error verifying message: {:?}", e);
                TaskError::TaskVerificationFailed
            })?;

            if is_verified {
                verified_responses.push(response);
            } else {
                error!("Task response verification failed.");
            }
        }

        let mut response_map = HashMap::new();

        let addresses: Vec<Address> = verified_responses
            .iter()
            .map(|r| Ok(r.public_key))
            .collect::<Result<_, TaskError>>()?;

        let (operator_stakes, total_stake) = self
            .get_operator_stake_mapping(addresses, Uint::from(0u64))
            .await?;

        let default_stake = Uint::from(0u64);

        for response in verified_responses.iter() {
            let response_value = Uint::from(response.completed_task.response);
            let public_key = response.public_key;
            let stake = operator_stakes.get(&public_key).unwrap_or(&default_stake);

            *response_map.entry(response_value).or_insert(default_stake) += *stake;
        }

        info!("Finished mapping responses to stakes.");

        let most_frequent_response = response_map
            .into_iter()
            .max_by_key(|&(_, stake)| stake)
            .ok_or(TaskError::TaskVerificationFailed)?;

        if most_frequent_response.1 < total_stake / Uint::from(2u64) {
            error!("Majority not reached. Expected at least half of total stake.");
            return Err(TaskError::MajorityNotReached);
        }

        Ok(most_frequent_response.0)
    }

    async fn verify_message(&self, task_response: &TaskResponse) -> Result<bool> {
        let address: Address = task_response.public_key;
        let signature: Signature = task_response.signature;
        let message = serde_json::to_string(&task_response.completed_task)?;
        let recovered_address = signature.recover_address_from_msg(message)?;
        Ok(recovered_address == address)
    }

    async fn write_block_number_to_file(&self, file: &str, val: u64) -> Result<()> {
        let data = BlockNumberData { block_number: val };

        let json_data = serde_json::to_string_pretty(&data)?;
        fs::write(file, json_data)?;

        Ok(())
    }
}
