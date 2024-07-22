import { CompletedTask, Task, TaskResponse } from "@/api/models/Task";
import { completedTasks } from "@/storage/task";
import { client } from "@/utils/contract/contract";

export async function signObject(completedTask: CompletedTask): Promise<string> {
	const dataString = JSON.stringify(completedTask);
	const signature = await client.signMessage({ message: dataString });
	return signature as string;
}

export async function handleTask(task: Task): Promise<TaskResponse> {
	const completedTask = await runTask(task);
	const signature = await signObject(completedTask);
	return {
		completedTask: completedTask,
		publicKey: client.account.address,
		signature: signature,
	} as TaskResponse;
}

export async function runTask(task: Task): Promise<CompletedTask> {
	const taskResponse = taskComputation(task.value);
	const completedTaskVal = {
		value: task.value,
		response: taskResponse,
		completedAt: new Date(),
	} as CompletedTask;
	completedTasks.push(completedTaskVal);
	return completedTaskVal;
}

const taskComputation = (val: number) => {
	return val ** 2;
};
