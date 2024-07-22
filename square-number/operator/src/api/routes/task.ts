import { OpenAPIRegistry } from "@asteasolutions/zod-to-openapi";
import express, { Request, Response, Router } from "express";

import { handleTask } from "@/api/controllers/task";
import { createApiResponse } from "@/api/docs/openAPIResponseBuilders";
import { handleServiceResponse } from "@/api/handlers";
import { ServiceResponse } from "@/api/models";
import { Task, TaskResponseSchema, TaskRequest } from "@/api/models/Task";

export const taskPath = "/task";
export const taskRegistry = new OpenAPIRegistry();
export const taskRouter: Router = express.Router();

taskRegistry.registerPath({
	method: "post",
	path: taskPath,
	tags: ["Task"],
	request: { body: { content: { "application/json": { schema: TaskRequest.shape.content } } } },
	responses: createApiResponse(TaskResponseSchema, "Success"),
});

taskRouter.post("/", async (req: Request, res: Response) => {
	try {
		const task: Task = req.body;
		const taskResponse = await handleTask(task);
		handleServiceResponse(ServiceResponse.success("Task completed successfully", taskResponse), res);
	} catch (error) {
		handleServiceResponse(ServiceResponse.failure(`router :: POST :: /task :: failed with error ${error}`, null), res);
	}
});
