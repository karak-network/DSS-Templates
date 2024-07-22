import { readFileSync, writeFileSync } from "fs";

import { logger } from "@/server";

export const readBlockNumberFromFile = async (file: string): Promise<bigint> => {
	try {
		return BigInt((await JSON.parse(readFileSync(file, "utf-8"))).blockNumber);
	} catch (error) {
		console.log(file);
		logger.error("Error reading from file", error);
		return BigInt(0);
	}
};

export const writeBlockNumberToFile = async (file: string, val: number): Promise<void> => {
	try {
		const json = JSON.parse(readFileSync(file, "utf-8"));
		json.blockNumber = val;
		const jsonData = JSON.stringify(json, null, 2); // Indent with 2 spaces
		writeFileSync(file, jsonData, "utf-8");
	} catch (error) {
		logger.error("Error writing to file:", error);
	}
};
