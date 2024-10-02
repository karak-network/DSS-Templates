use alloy::{primitives::Address, signers::local::PrivateKeySigner};
use axum::{routing::get, Router};
use base64::Engine;
use karak_rs::kms::keypair::bn254::Keypair;
use serde::Deserialize;
use std::{net::IpAddr, str::FromStr};
use thiserror::Error;
use url::Url;

pub mod contract;
pub mod error;
pub mod health;
pub mod operator;
pub mod register;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub host: IpAddr,
    pub port: u16,
    pub rpc_url: Url,
    #[serde(deserialize_with = "deserialize_private_key")]
    pub private_key: alloy::signers::local::PrivateKeySigner,
    pub domain_url: Url,
    pub aggregator_url: Url,
    pub square_number_dss_address: Address,
    pub core_address: Address,
    pub heartbeat: u64,
    #[serde(deserialize_with = "deserialize_bls_keypair")]
    pub bls_keypair: Keypair,
}

fn deserialize_bls_keypair<'de, D>(deserializer: D) -> Result<Keypair, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let base64_str: String = Deserialize::deserialize(deserializer)?;
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(&base64_str)
        .map_err(|e| serde::de::Error::custom(format!("Base64 decoding error: {}", e)))?;

    Keypair::from_bytes(&decoded_bytes).map_err(serde::de::Error::custom)
}

fn deserialize_private_key<'de, D>(deserializer: D) -> Result<PrivateKeySigner, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    PrivateKeySigner::from_str(&s).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
pub struct ContractAddresses {
    pub square_number_dss: Address,
    pub core: Address,
    pub block_number: u64,
}

#[derive(Debug, Error)]
pub enum TaskError {
    #[error("Contract call error")]
    ContractCallError,

    #[error("Operator not found")]
    OperatorNotFound,

    #[error("Task verification failed")]
    TaskVerificationFailed,

    #[error("Majority not reached")]
    MajorityNotReached,

    #[error("Signature Conversion error")]
    SignatureConversionError,

    #[error("JSON parsing error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Custom URL error: {0}")]
    CustomUrlError(String),

    #[error("Signing error: {0}")]
    SigningError(#[from] alloy::signers::Error),
}

pub fn routes(keypair: karak_rs::kms::keypair::bn254::Keypair, wallet: PrivateKeySigner) -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .nest("/operator", operator::operator_router(keypair, wallet))
}
