import { extendZodWithOpenApi } from "@asteasolutions/zod-to-openapi";
import { z } from "zod";

extendZodWithOpenApi(z);

export type Task = z.infer<typeof TaskSchema>;
export const TaskSchema = z.object({
	value: z.number(),
});

export const TaskRequest = z.object({
	content: TaskSchema,
});

export type CompletedTask = z.infer<typeof CompletedTaskSchema>;
export const CompletedTaskSchema = z.object({
	value: z.number(),
	response: z.number(),
	completedAt: z.date(),
});

export type TaskResponse = z.infer<typeof TaskResponseSchema>;
export const TaskResponseSchema = z.object({
	completedTask: CompletedTaskSchema,
	publicKey: z.string(),
	signature: z.string(),
});
