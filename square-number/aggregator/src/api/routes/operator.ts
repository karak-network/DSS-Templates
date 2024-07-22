import { OpenAPIRegistry } from "@asteasolutions/zod-to-openapi";
import express, { Request, Response, Router } from "express";

import { isOperatorRegistered, registerOperator } from "@/api/controllers/operator";
import { createApiResponse } from "@/api/docs/openAPIResponseBuilders";
import { handleServiceResponse } from "@/api/handlers";
import { ServiceResponse } from "@/api/models";
import { GetOperatorRequest, NewOperatorRequest, Operator, OperatorSchema } from "@/api/models/Operator";
import { logger } from "@/server";

export const operatorPath = "/operator";
export const operatorRegistry = new OpenAPIRegistry();
export const operatorRouter: Router = express.Router();

operatorRegistry.registerPath({
	method: "post",
	path: operatorPath,
	tags: ["Operator"],
	request: { body: { content: { "application/json": { schema: NewOperatorRequest.shape.content } } } },
	responses: createApiResponse(OperatorSchema, "Success"),
});

operatorRouter.post("/", async (req: Request, res: Response) => {
	try {
		const operator: Operator = req.body;
		await registerOperator(operator);
		logger.info(`Operator ${operator.publicKey} registered successfully`);
		handleServiceResponse(ServiceResponse.success("Operator registered successfully", null), res);
	} catch (error) {
		handleServiceResponse(ServiceResponse.failure(`router :: POST :: /operator :: failed with error ${error}`, null), res);
	}
});

operatorRegistry.registerPath({
	method: "get",
	path: operatorPath,
	tags: ["Operator"],
	request: { query: GetOperatorRequest.shape.query },
	responses: createApiResponse(OperatorSchema, "Success"),
});

operatorRouter.get("/", async (req: Request, res: Response) => {
	try {
		const isRegistered = isOperatorRegistered(req.query.address as Operator);
		handleServiceResponse(ServiceResponse.success(`Operator ${req.body} is registered: ${isRegistered}`, isRegistered), res);
	} catch (error) {
		handleServiceResponse(ServiceResponse.failure(`router :: GET :: /operator :: failed with error ${error}`, null), res);
	}
});
