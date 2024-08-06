import * as PromoClient from 'prom-client';
import {Router} from "express";
import {Counter, Gauge} from "prom-client";

class Prometheus {
    public collectDefaultMetrics: typeof PromoClient.collectDefaultMetrics;
    public testCounter: Counter;
    public testGauge: Gauge;

    constructor() {
        this.collectDefaultMetrics = PromoClient.collectDefaultMetrics;
        this.testCounter= new Counter({
            name: 'test_counter',
            help: 'Example of a counter',
            labelNames: ['code'],
        });

        this.testGauge =  new Gauge({
            name: 'test_gauge',
            help: 'Example of a gauge',
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