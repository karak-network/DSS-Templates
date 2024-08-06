import { getRequest, postRequest } from "@/api/handlers";
import { Operator } from "@/api/models/Operator";
import { env } from "@/config";
import { logger } from "@/server";
import { coreContract, dssContract, dssContractAddress } from "@/utils/contract/contract";
import {pm} from "@/utils/prometheus";

export async function registerOperator(aggregatorURL: string, operatorPubkey: string, operatorUrl: string) {
	const registeredInDSS = await isRegisteredInDSS(operatorPubkey);
	if (!registeredInDSS) await registerInDSS();

	setInterval(async () => {
		await registerInDSSAndAggregator(aggregatorURL, operatorPubkey, operatorUrl);
		pm.testCounter.inc(1);
		pm.testGauge.set({ code: 200 }, 5);
	}, env.HEARTBEAT);
}

async function registerInDSSAndAggregator(aggregatorURL: string, operatorPubkey: string, operatorUrl: string) {
	const registeredWithAggregator = await isRegisteredWithAggregator(aggregatorURL, operatorPubkey);
	if (!registeredWithAggregator) await registerOperatorWithAggregator(aggregatorURL, operatorPubkey, operatorUrl);
}

async function isRegisteredInDSS(operatorAddress: string): Promise<boolean> {
	const isRegistered = (await dssContract.read.isOperatorRegistered([operatorAddress])) as boolean;
	logger.info(`operatorService :: isRegisteredInDSS :: got response ${isRegistered} from DSS`);

	return isRegistered;
}

async function registerInDSS() {
	await coreContract.write.registerOperatorToDSS([dssContractAddress, "0x"]);
	logger.info("operatorService :: registerInDSS :: operator registered successfully in the DSS");
}

async function isRegisteredWithAggregator(aggregatorURL: string, operatorPubkey: string): Promise<boolean> {
	try {
		const isRegistered: boolean = await getRequest(aggregatorURL + "/operator?address=" + operatorPubkey);
		logger.info(`operatorService :: isRegisteredWithAggregator :: got response ${isRegistered}`);
		return isRegistered;
	} catch (error) {
		logger.error("operatorService :: isOperatorRegistered :: api request failed", error);
		return false;
	}
}

async function registerOperatorWithAggregator(aggregatorURL: string, operatorPubkey: string, operatorUrl: string) {
	const operator: Operator = { publicKey: operatorPubkey, url: operatorUrl };
	try {
		await postRequest(aggregatorURL + "/operator", operator);
		logger.info(`operatorService :: registerOperatorWithAggregator :: successfully registered operator`);
	} catch (e) {
		logger.error(`operatorService ::registerWithAggregator :: api request failed`);
	}
}
