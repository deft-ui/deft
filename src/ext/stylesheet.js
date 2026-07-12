import * as stylesheetApi from "native";

export class StylesheetItem {
    id;
    constructor(id) {
        this.id = id;
    }

    update(code) {
        stylesheetApi.update(this.id, code);
    }
}

export class Stylesheet {
    /**
     *
     * @param code {string}
     * @returns {StylesheetItem}
     */
    append(code) {
        const id = stylesheetApi.add(code);
        return new StylesheetItem(id);
    }

    /**
     *
     * @param stylesheet {StylesheetItem}
     */
    remove(stylesheet) {
        stylesheetApi.remove(stylesheet.id);
    }
}

navigator.stylesheet = new Stylesheet();