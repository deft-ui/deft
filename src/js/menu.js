import * as standardMenuItemApi from "deft:core:standardmenuitem";
import * as menuApi from "deft:core:menu";
export class StandardMenuItem {
    #handle;
    constructor(label, callback) {
        this.#handle = standardMenuItemApi.jsNew(label, callback);
    }

    /**
     *
     * @param value {boolean}
     */
    set disabled(value) {
        standardMenuItemApi.setDisabled(this.#handle, value);
    }

    /**
     *
     * @returns {boolean}
     */
    get disabled() {
        return standardMenuItemApi.getDisabled(this.#handle);
    }

    get handle() {
        return this.#handle;
    }
}

export class Menu {
    #handle

    constructor() {
        this.#handle = menuApi.create();
    }

    /**
     *
     * @param item {StandardMenuItem}
     */
    addStandardItem(item) {
        menuApi.addStandardItem(this.#handle, item.handle);
    }

    addSeparator() {
        menuApi.addSeparator(this.#handle);
    }

    get handle() {
        return this.#handle;
    }

}