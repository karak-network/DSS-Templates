import { Express } from "express";

import { healthzRouter, healthzPath } from "./healthz";
import { operatorRouter, operatorPath } from "./operator";

export default function mountRoutes(app: Express) {
	app.use(healthzPath, healthzRouter);
	app.use(operatorPath, operatorRouter);
}
