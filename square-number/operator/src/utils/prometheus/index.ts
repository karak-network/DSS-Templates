import { Counter, collectDefaultMetrics, register } from 'prom-client';
import {Router} from "express";
import {logger} from "@/server";

class Prometheus {
    public collectDefaultMetrics: typeof collectDefaultMetrics;
    public isRegisteredOnChain: Counter;
    public isRegisteredToAggregator: Counter;
    public taskReceived: Counter;
    public taskCompleted: Counter;
    public taskErrored: Counter;
    constructor() {
        this.collectDefaultMetrics = collectDefaultMetrics;
        this.isRegisteredOnChain= new Counter({
            name: 'chain_registration_success',
            help: 'Boolean flag indicating if operator is registered to DSS contract',
            labelNames: ['code'],
        });

        this.isRegisteredToAggregator= new Counter({
            name: 'aggregator_registration_success',
            help: 'Boolean flag indicating if operator is registered to Aggregator',
        });

        this.taskReceived = new Counter({
            name: 'task_received',
            help: 'Counter for task received successfully',
        });

        this.taskErrored = new Counter({
            name: 'task_errored',
            help: 'Counter for task that errored',
        });

        this.taskCompleted = new Counter({
            name: 'task_completed',
            help: 'Counter of successful task completions',
        });

    }

    public addMetricRoute(app: Router) {
        this.collectDefaultMetrics();
        app.get('/metrics', async (req, res) => {
            try {
                res.set('Content-Type', register.contentType);
                res.end(await register.metrics());
            } catch (ex) {
                logger.error('Error fetching metrics:', ex);
                res.status(500).end('Error fetching metrics');
            }
        });
    }
}

export const pm = new Prometheus();