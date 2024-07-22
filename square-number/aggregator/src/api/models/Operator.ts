import { extendZodWithOpenApi } from "@asteasolutions/zod-to-openapi";
import { z } from "zod";

extendZodWithOpenApi(z);

export type Operator = z.infer<typeof OperatorSchema>;
export const OperatorSchema = z.object({
	publicKey: z.string(),
	url: z.string(),
});

export const NewOperatorRequest = z.object({
	content: OperatorSchema,
});

export const GetOperatorRequest = z.object({
	query: z.object({ address: z.string() }),
});
