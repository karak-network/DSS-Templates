import { startServices } from "@/api/services";
import { env } from "@/config";
import { app, logger } from "@/server";
import { client } from "@/utils/contract/contract";

const server = app.listen(env.PORT, () => {
	const { NODE_ENV, HOST, PORT } = env;
	const operatorUrl = `http://${HOST}:${PORT}`;
	logger.info(`Server (${NODE_ENV}) running on port ${operatorUrl}`);

	// Start background services
	startServices(env.AGGREGATOR_URL, client.account.address, operatorUrl);
});

const onCloseSignal = () => {
	logger.info("sigint received, shutting down");
	server.close(() => {
		logger.info("server closed");
		process.exit();
	});
	setTimeout(() => process.exit(1), 10000).unref(); // Force shutdown after 10s
};

process.on("SIGINT", onCloseSignal);
process.on("SIGTERM", onCloseSignal);
