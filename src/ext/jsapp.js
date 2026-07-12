import * as jsAppApi from "native";
import {EventBinder} from "deft:core";

class DeftApp {

    /**
     * @var {EventBinder}
     */
    #eventBinder;

    constructor() {
        this.#eventBinder = new EventBinder(null, jsAppApi.bindJsEventListener, jsAppApi.unbindJsEventListener, this);
    }

    /**
     *
     * @param callback {(event: IAppReopenEvent) => void}
     */
    bindReopen(callback) {
        this.#eventBinder.bindEvent("reopen", callback);
    }

}

navigator.app = new DeftApp();