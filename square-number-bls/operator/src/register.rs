use crate::{contract::SquareNumberDSS::SquareNumberDSSInstance, Config};
use alloy::{
    network::{Ethereum, EthereumWallet},
    primitives::{Address, Bytes, FixedBytes, TxHash},
    providers::{
        fillers::{FillProvider, JoinFill, RecommendedFiller, WalletFiller},
        ProviderBuilder, ReqwestProvider,
    },
    transports::http::{reqwest, ReqwestTransport},
};
use eyre::Result;
use karak_rs::bls::keypair_signer::Signer;
use karak_rs::{
    bls::{keypair_signer::KeypairSigner, registration::OperatorRegistration},
    contracts::Core::CoreInstance,
    kms::keypair::bn254::{algebra::g1::G1Point, Keypair},
};
use serde::Serialize;
use tokio::time::{self, Duration};
use tracing::{error, info};
use url::Url;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AddressPayload {
    public_key: Address,
    url: Url,
}

pub type RecommendedProvider = FillProvider<
    JoinFill<RecommendedFiller, WalletFiller<EthereumWallet>>,
    ReqwestProvider,
    ReqwestTransport,
    Ethereum,
>;

pub struct RegistrationService {
    dss_instance: SquareNumberDSSInstance<ReqwestTransport, RecommendedProvider>,
    core_instance: CoreInstance<ReqwestTransport, RecommendedProvider>,
    operator_address: Address,
    dss_address: Address,
    aggregator_url: Url,
    domain_url: Url,
    reqwest_client: reqwest::Client,
    heartbeat_interval: Duration,
    bls_keypair: Keypair,
}

impl RegistrationService {
    pub fn new(config: Config) -> Result<Self> {
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(EthereumWallet::from(config.private_key.clone()))
            .on_http(config.rpc_url);
        let dss_address = config.square_number_dss_address;
        let dss_instance = SquareNumberDSSInstance::new(dss_address, provider.clone());
        let core_instance = CoreInstance::new(config.core_address, provider);
        let heartbeat_interval = Duration::from_millis(config.heartbeat);
        let bls_keypair = config.bls_keypair;
        Ok(Self {
            dss_instance,
            core_instance,
            operator_address: config.private_key.address(),
            dss_address,
            aggregator_url: config.aggregator_url,
            domain_url: config.domain_url,
            reqwest_client: reqwest::Client::new(),
            heartbeat_interval,
            bls_keypair,
        })
    }

    pub async fn start(&self) {
        loop {
            let registered_in_dss = match self.is_registered_in_dss().await {
                Ok(registered) => registered,
                Err(e) => {
                    error!("Failed to check registration status: {e}");
                    continue;
                }
            };

            if !registered_in_dss {
                info!("Operator not registered in DSS. Registering...");
                match self.register_in_dss().await {
                    Ok(receipt) => {
                        info!("operatorService :: register_in_dss :: operator registered successfully in the DSS :: {receipt}");
                        break;
                    }
                    Err(e) => {
                        error!("Failed to register operator in DSS: {e}");
                        continue;
                    }
                }
            } else {
                info!("Operator already registered in DSS");
                break;
            }
        }
        info!("Operator registration service started");

        let mut interval = time::interval(self.heartbeat_interval);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.heartbeat_check().await {
                        error!("Heartbeat failed: {e}");
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("Received shutdown signal. Stopping the service...");
                    break;
                }
            }
        }

        info!("Operator service stopped gracefully.");
    }

    async fn heartbeat_check(&self) -> Result<()> {
        let registered_with_aggregator = self.is_registered_with_aggregator().await?;
        if !registered_with_aggregator {
            self.register_operator_with_aggregator().await?;
        }
        Ok(())
    }

    async fn is_registered_in_dss(&self) -> Result<bool> {
        Ok(self
            .dss_instance
            .isOperatorRegistered(self.operator_address)
            .call()
            .await?
            ._0)
    }

    async fn get_registration_msg_hash(&self) -> FixedBytes<32> {
        self.dss_instance
            .REGISTRATION_MESSAGE_HASH()
            .call()
            .await
            .unwrap()
            ._0
    }

    async fn register_in_dss(&self) -> Result<TxHash> {
        let bls_pubkey = karak_rs::kms::keypair::traits::Keypair::public_key(&self.bls_keypair);
        let msg_hash = self.get_registration_msg_hash().await;
        let signature = self.sign(&self.bls_keypair.clone(), msg_hash.into())?;
        let registration = karak_rs::bls::registration::BlsRegistration {
            g1_pubkey: bls_pubkey.g1,
            g2_pubkey: bls_pubkey.g2,
            signature,
            msg_hash,
        };
        let receipt = self
            .core_instance
            .register_operator_to_dss_with_bls(self.dss_address, &registration)
            .await?;

        Ok(receipt)
    }

    fn sign(&self, bls_keypair: &Keypair, msg_hash: Bytes) -> Result<G1Point, eyre::Error> {
        let mut hash_buffer = [0u8; 32];
        hash_buffer.copy_from_slice(&msg_hash);
        let keypair_signer = KeypairSigner::from(bls_keypair.clone());
        let signature = keypair_signer.sign_message(hash_buffer)?;
        Ok(signature)
    }
    async fn is_registered_with_aggregator(&self) -> Result<bool> {
        let url = self
            .aggregator_url
            .join("aggregator/isOperatorRegistered")?;

        let payload = AddressPayload {
            public_key: self.operator_address,
            url: self.domain_url.clone(),
        };

        Ok(self
            .reqwest_client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?
            .json::<bool>()
            .await?)
    }

    pub async fn register_operator_with_aggregator(&self) -> Result<()> {
        let url = self.aggregator_url.join("aggregator/registerOperator")?;
        let operator = AddressPayload {
            public_key: self.operator_address,
            url: self.domain_url.clone(),
        };

        self.reqwest_client.post(url).json(&operator).send().await?;

        Ok(())
    }
}
