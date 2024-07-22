import { Operator } from "@/api/models/Operator";
import { registeredOperators } from "@/storage/operators";
import { dssContract } from "@/utils/contract/contract";

export async function registerOperator(operator: Operator) {
	const isRegisteredInDSS = await isOperatorRegisteredInDSS(operator);
	if (!registeredOperators.includes(operator) && isRegisteredInDSS) {
		registeredOperators.push(operator);
	} else if (!isRegisteredInDSS) {
		throw new Error("not registered in DSS");
	} else {
		throw new Error("operator already registered");
	}
}

export function isOperatorRegistered(operator: Operator) {
	const isRegistered = registeredOperators.some((op) => {
		return operator.publicKey == op.publicKey;
	});
	return isRegistered;
}

export async function isOperatorRegisteredInDSS(operatorInfo: Operator): Promise<boolean> {
	const isRegistered = (await dssContract.read.isOperatorRegistered([operatorInfo.publicKey])) as boolean;
	return isRegistered;
}
