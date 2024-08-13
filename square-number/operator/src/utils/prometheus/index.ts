import * as PromoClient from 'prom-client';
import {Router} from "express";
import {Counter} from "prom-client";

class Prometheus {
    public collectDefaultMetrics: typeof PromoClient.collectDefaultMetrics;
    public isChainRegistered: Counter;
    public isRegisteredToAggregator: Counter;
    public taskReceived: Counter;
    public taskCompleted: Counter;
    public taskErrored: Counter;
    constructor() {
        this.collectDefaultMetrics = PromoClient.collectDefaultMetrics;
        this.isChainRegistered= new Counter({
            name: 'chain_registration_success',
            help: 'Boolean flag to find if the operator is registered to contract or not',
            labelNames: ['code'],
        });

        this.isRegisteredToAggregator= new Counter({
            name: 'aggregator_registration_success',
            help: 'Boolean flag to find if the operator is registered to aggregator server or not',
            labelNames: ['code'],
        });

        this.taskReceived = new Counter({
            name: 'task_received',
            help: 'Counter for task received successfully',
            labelNames: ['code'],
        });

        this.taskErrored = new Counter({
            name: 'task_errored',
            help: 'Counter for task errored in api call',
            labelNames: ['code'],
        });

        this.taskCompleted = new Counter({
            name: 'task_completed',
            help: 'Counter for successful response for api call for sending task',
            labelNames: ['code'],
        });

    }

    public addMetricRoute(app: Router) {
        this.collectDefaultMetrics();
        app.get('/metrics', async (req, res) => {
            try {
                res.set('Content-Type', PromoClient.register.contentType);
                res.end(await PromoClient.register.metrics());
            } catch (ex) {
                res.status(500).end(ex);
            }
        });
    }
}

export const pm = new Prometheus();