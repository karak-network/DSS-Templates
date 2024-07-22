import { Express } from "express";

import { healthzRouter, healthzPath } from "./healthz";
import { taskRouter, taskPath } from "./task";

export default function mountRoutes(app: Express) {
	app.use(healthzPath, healthzRouter);
	app.use(taskPath, taskRouter);
}
