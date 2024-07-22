import { OpenAPIRegistry } from "@asteasolutions/zod-to-openapi";
import express, { Request, Response, Router } from "express";
import { z } from "zod";

import { createApiResponse } from "@/api/docs/openAPIResponseBuilders";
import { handleServiceResponse } from "@/api/handlers";
import { ResponseStatus, ServiceResponse } from "@/api/models";

export const healthzPath = "/healthz";
export const healthzRegistry = new OpenAPIRegistry();
export const healthzRouter: Router = express.Router();

healthzRegistry.registerPath({
	method: "get",
	path: healthzPath,
	tags: ["Health Check"],
	responses: createApiResponse(z.null(), "Success"),
});

healthzRouter.get("/", (_req: Request, res: Response) => {
	const serviceResponse = ServiceResponse.success("Service is healthy", null);
	handleServiceResponse(serviceResponse, res);
});
