import { Address, Hex, verifyMessage } from "viem";

import { ServiceResponse } from "@/api/models";
import { Task, TaskRequest, TaskResponse } from "@/api/models/Task";
import { env } from "@/config";
import { logger } from "@/server";
import { registeredOperators } from "@/storage/operators";
import { dssContract } from "@/utils/contract/contract";
import { getOperatorStakeMapping } from "@/utils/contract/interactions/core";
import { readBlockNumberFromFile, writeBlockNumberToFile } from "@/utils/file";

export function startTaskServices() {
	logger.info("Listening for task request events");
	setInterval(watchForTaskEvents, env.HEARTBEAT);
}

async function watchForTaskEvents() {
	const nextBlockToCheck = await readBlockNumberFromFile(env.CONTRACTS_JSON);
	const events = await dssContract.getEvents.TaskRequestGenerated({ fromBlock: nextBlockToCheck });
	const taskRequests: TaskRequest[] = events.map((event: any) => {
		return {
			task: { value: Number(event["args"]["taskRequest"]["value"]) } as Task,
			blockNumber: Number(event["blockNumber"]),
		} as TaskRequest;
	});

	if (registeredOperators.length > 0) {
		taskRequests.forEach(async (taskRequest) => {
			const responses = await sendTaskToAllOperators(taskRequest.task as Task);
			const blockNumber = taskRequest.blockNumber!;
			const taskResponse = { response: responses };

			await dssContract.write.submitTaskResponse([taskRequest.task, taskResponse]);
			await writeBlockNumberToFile(env.CONTRACTS_JSON, blockNumber + 1);
		});
	} else {
		logger.error("task :: watchForTaskEvents :: no operators are registered");
	}
}

async function sendTaskToAllOperators(task: Task): Promise<number> {
	let operatorResponses: TaskResponse[] = await Promise.all(
		registeredOperators.map(async (operator) => {
			try {
				const response = await fetch(operator.url + "/operator/requestTask", {
					method: "POST",
					body: JSON.stringify(task),
					headers: { "Content-Type": "application/json" },
				});
				const responseJson = await response.json();
				const serviceResponse: any = (responseJson as ServiceResponse<TaskResponse>).responseObject;
				return {
					completedTask: serviceResponse.completedTask,
					publicKey: serviceResponse.publicKey,
					signature: serviceResponse.signature,
				} as TaskResponse;
			} catch (e) {
				logger.error(`task :: sendTaskToAllOperators :: error :: ${e}`);
				return undefined as unknown as TaskResponse;
			}
		})
	);
	operatorResponses = operatorResponses.filter(
		async (response) =>
			await verifyMessage({
				address: response.publicKey as Address,
				message: JSON.stringify(response.completedTask),
				signature: response.signature as Hex,
			})
	);

	const responseMap: Map<number, bigint> = new Map();
	const [operatorStakes, totalStake] = await getOperatorStakeMapping(
		operatorResponses.map((response) => response.publicKey!),
		0n
	);
	operatorResponses.forEach((operator) => {
		const operatorResponses = operator.completedTask!.response!;
		// get stake of each operator
		if (responseMap.get(operatorResponses) != undefined) {
			responseMap.set(operatorResponses, responseMap.get(operatorResponses)! + operatorStakes.get(operator.publicKey!)!);
		} else {
			responseMap.set(operatorResponses, operatorStakes.get(operator.publicKey!)!);
		}
	});
	const mostFrequentResponse = [...responseMap.entries()].reduce((a, b) => (b[1] > a[1] ? b : a));
	if (mostFrequentResponse[1] < totalStake / 2n) throw new Error("Majority not reached");
	return mostFrequentResponse[0];
}
