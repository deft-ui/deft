import * as systemTrayApi from "native";
import {EventRegistry} from "deft:core";

export class SystemTray {
    /**
     * @type EventRegistry
     */
    #eventRegistry;

    #menuUserCallback;

    tray;
    constructor() {
        this.tray = systemTrayApi.create("Deft");
        this.#eventRegistry = new EventRegistry(this.tray, systemTrayApi.bindEvent, systemTrayApi.removeEventListener, this);
    }

    set title(title) {
        systemTrayApi.setTitle(this.tray, title);
    }

    set icon(icon) {
        systemTrayApi.setIcon(this.tray, icon);
    }

    /**
     *
     * @param menus {TrayMenu[]}
     */
    setMenus(menus) {
        const list = [];
        const menuHandlers = new Map();
        for (const m of menus) {
            const {id, label, checked, enabled} = m;
            const kind = m.kind || "standard";
            if (m.handler) {
                menuHandlers.set(m.id, m.handler);
            }
            list.push({id, label, kind, checked, enabled});
        }
        const menuHandler = (e) => {
            const id = e.detail;
            const handler = menuHandlers.get(id);
            if (handler) {
                handler();
            }
            if (this.#menuUserCallback) {
                this.#menuUserCallback(e);
            }
        }
        systemTrayApi.setMenus(this.tray, list);
        this.#eventRegistry.bindEvent("menuclick", menuHandler)
    }

    setShowMenuOnLeftClick(value) {
        systemTrayApi.setShowMenuOnLeftClick(this.tray, value);
    }

    bindActivate(callback) {
        this.#eventRegistry.bindEvent("activate", callback);
    }

    bindMenuClick(callback) {
        this.#menuUserCallback = callback;
    }

}
