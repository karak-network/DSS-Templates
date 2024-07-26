import dotenv from "dotenv";
import { cleanEnv, host, num, port, str, testOnly } from "envalid";

dotenv.config();

export const env = cleanEnv(process.env, {
	// Server Config
	NODE_ENV: str({ devDefault: testOnly("test"), choices: ["development", "production", "test"] }),
	HOST: host({ devDefault: testOnly("localhost") }),
	PORT: port(),
	RPC_URL: str(),
	DOMAIN_URL: str(),

	// CORS Config
	CORS_ORIGIN: str(),

	// Rate Limiting Config
	COMMON_RATE_LIMIT_MAX_REQUESTS: num({ devDefault: testOnly(1000) }),
	COMMON_RATE_LIMIT_WINDOW_MS: num({ devDefault: testOnly(10000) }),

	// Aggregator Config
	AGGREGATOR_URL: str(),
	HEARTBEAT: num(),

	// User Config
	PRIVATE_KEY: str(),
});
