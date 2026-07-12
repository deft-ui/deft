import * as workerContextApi from "native";
import {EventBinder} from "deft:core";

export class WorkerContext {
    #workerContext;
    /**
     * @type {EventBinder}
     */
    #eventBinder;
    constructor() {
        this.#workerContext = workerContextApi.get();
        this.#eventBinder = new EventBinder(
            this.#workerContext,
            workerContextApi.bindJsEventListener,
            workerContextApi.removeJsEventListener,
            this
        )
    }
    postMessage(data) {
        workerContextApi.postMessage(this.#workerContext, JSON.stringify(data));
    }
    bindMessage(callback) {
        this.#eventBinder.bindEvent('message', e => {
            e.data = JSON.parse(e.detail.data);
            callback(e);
        });
    }

}

globalThis.workerContext = new WorkerContext();