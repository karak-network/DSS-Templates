import { extendZodWithOpenApi } from "@asteasolutions/zod-to-openapi";
import { z } from "zod";

extendZodWithOpenApi(z);

export type Task = z.infer<typeof TaskSchema>;
export const TaskSchema = z.object({
	value: z.number(),
});

export type TaskRequest = z.infer<typeof TaskRequestSchema>;
export const TaskRequestSchema = z.object({
	task: TaskSchema,
	blockNumber: z.number(),
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
