import cors from "cors";
import express, { type Express } from "express";
import helmet from "helmet";
import { pino } from "pino";

import { env } from "@/config";
import mountRoutes from "./api/routes";
import { openAPIRouter } from "./api/docs/openAPIRouter";
import errorHandler from "@/middleware/errorHandler";
import rateLimiter from "@/middleware/rateLimiter";
import requestLogger from "@/middleware/requestLogger";
import {pm} from "@/utils/prometheus";

const logger = pino({ name: "server start" });
const app: Express = express();

// Set the application to trust the reverse proxy
app.set("trust proxy", true);

// Middlewares
app.use(cors({ origin: env.CORS_ORIGIN, credentials: true }));
app.use(helmet());
app.use(rateLimiter);
app.use(express.json());
app.use(express.urlencoded({ extended: true }));

// Request logging
app.use(requestLogger);

pm.addMetricRoute(app);

// Routes
mountRoutes(app);

// Swagger UI
app.use(openAPIRouter);

// Error handlers
app.use(errorHandler());

export { app, logger };
