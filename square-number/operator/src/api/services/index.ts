import { registerOperator } from "@/api/services/registration";

export const startServices = async (aggregatorURL: string, operatorPubkey: string, operatorUrl: string) => {
	registerOperator(aggregatorURL, operatorPubkey, operatorUrl);
};
