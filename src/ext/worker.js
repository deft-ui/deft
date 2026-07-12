import * as workerApi from "native";
import {EventBinder} from "deft:core";

export class Worker {

    #worker

    /**
     * @type EventBinder
     */
    #eventBinder;

    /**
     *
     * @param source {number | string}
     */
    constructor(source) {
        this.#worker = typeof source === "string" ? workerApi.create(source) : workerApi.bind(source);
        this.#eventBinder = new EventBinder(
            this.#worker,
            workerApi.bindJsEventListener,
            workerApi.removeJsEventListener,
            this
        );
    }

    postMessage(data) {
        workerApi.postMessage(this.#worker, JSON.stringify(data));
    }

    bindMessage(callback) {
        this.#eventBinder.bindEvent('message', e => {
            e.data = JSON.parse(e.detail.data);
            callback(e);
        });
    }

}
globalThis.Worker = Worker;