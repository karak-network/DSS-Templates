import { Response } from "express";

import { ServiceResponse } from "@/api/models";
import { logger } from "@/server";

export function handleServiceResponse(serviceResponse: ServiceResponse<any>, response: Response) {
	return response.status(serviceResponse.statusCode).send(serviceResponse);
}

export async function postRequest<T>(url: string, data: any): Promise<T> {
	const response = await fetch(url, {
		method: "POST",
		body: JSON.stringify(data),
		headers: { "Content-Type": "application/json" },
	});
	const responseJson = await response.json();
	let responseObject: T;
	if (response.status === 200) {
		responseObject = (responseJson as ServiceResponse<T>).responseObject;
		logger.info(`getServiceResponseObject :: got response ${responseObject}`);
		return responseObject;
	} else {
		logger.error("getServiceResponseObject :: could not get response");
		throw new Error(`could not get response from ${url}`);
	}
}

export async function getRequest<T>(url: string): Promise<T> {
	const response = await fetch(url, {
		method: "POST",
		headers: { "Content-Type": "application/json" },
	});
	const responseJson = await response.json();
	let responseObject: T;
	if (response.status === 200) {
		responseObject = (responseJson as ServiceResponse<T>).responseObject;
		logger.info(`getServiceResponseObject :: got response ${responseObject}`);
		return responseObject;
	} else {
		logger.error("getServiceResponseObject :: could not get response");
		throw new Error(`could not get response from ${url}`);
	}
}
