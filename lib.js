const VT_CONTAINER = "container"
const VT_LABEL = "label"
const VT_BUTTON = "button"
const VT_ENTRY = "entry"
const VT_GROUP = "group"
const VT_PROGRESS_BAR = "progressbar"
const VT_SCROLL = "scroll"
const VT_TEXT_INPUT = "text-input"
const VT_TEXT_EDIT = "text-edit"
const VT_IMAGE = "image"
const VT_BODY = "body"
const VT_PARAGRAPH = "paragraph"
const VT_CHECKBOX = "checkbox"
const VT_RADIO = "radio"
const VT_RADIO_GROUP = "radio-group"
const VT_RICH_TEXT = "rich-text"
const VT_SELECT = "select"
const VT_DIALOG = "dialog";
const VT_DIALOG_TITLE = "dialog-title";


class Clipboard {
    /**
     *
     * @returns {Promise<string>}
     */
    async readText() {
        return Clipboard_read_text();
    }

    /**
     *
     * @param text {string}
     * @returns {Promise<void>}
     */
    async writeText(text) {
        Clipboard_write_text(text);
    }
}

class StylesheetItem {
    id;
    constructor(id) {
        this.id = id;
    }

    update(code) {
        stylesheet_update(this.id, code);
    }
}

class Stylesheet {
    /**
     *
     * @param code {string}
     * @returns {StylesheetItem}
     */
    append(code) {
        const id = stylesheet_add(code);
        return new StylesheetItem(id);
    }

    /**
     *
     * @param stylesheet {StylesheetItem}
     */
    remove(stylesheet) {
        stylesheet_remove(stylesheet.id);
    }
}

export class Navigator {

    /**
     * @var {Clipboard}
     */
    clipboard;
    /**
     * @var {Stylesheet}
     */
    stylesheet;
    constructor() {
        this.clipboard = new Clipboard();
        this.stylesheet = new Stylesheet();
    }
}

export class Process {
    /**
     *
     * @param code {number}
     */
    exit(code) {
        process_exit(code);
    }

    /**
     *
     * @returns {string[]}
     */
    get argv() {
        return process_argv();
    }

    /**
     *
     * @returns {boolean}
     */
    get isMobilePlatform() {
        return process_is_mobile_platform();
    }

    /**
     *
     * @returns {string}
     */
    get platform() {
        return process_platform();
    }

    /**
     *
     * @param handler {Function}
     */
    setPromiseRejectionTracker(handler) {
        process_set_promise_rejection_tracker(handler);
    }
}


export class FileDialog {
    /**
     *
     * @param options {ShowFileDialogOptions}
     * @returns {Promise<string[]>}
     */
    show(options) {
        return new Promise((resolve, reject) => {
            dialog_show_file_dialog({
                dialogType: options.dialogType,
            }, options.window?.handle, (result, data) => {
                if (result) {
                    resolve(data);
                } else {
                    reject(data);
                }
            })
        })

    }
}

export class Page {
    handle
    constructor(handle) {
        this.handle = handle;
    }
    close() {
        Page_close(this.handle);
    }
}

export class Popup {
    handle
    constructor(handle) {
        this.handle = handle;
    }
    close() {
        Popup_close(this.handle);
    }
}

export class StandardMenuItem {
    #handle;
    constructor(label, callback) {
        this.#handle = StandardMenuItem_js_new(label, callback);
    }

    /**
     *
     * @param value {boolean}
     */
    set disabled(value) {
        StandardMenuItem_set_disabled(this.#handle, value);
    }

    /**
     *
     * @returns {boolean}
     */
    get disabled() {
        return StandardMenuItem_get_disabled(this.#handle);
    }

    get handle() {
        return this.#handle;
    }
}

export class Menu {
    #handle

    constructor() {
        this.#handle = Menu_new();
    }

    /**
     *
     * @param item {StandardMenuItem}
     */
    addStandardItem(item) {
        Menu_add_standard_item(this.#handle, item.handle);
    }

    addSeparator() {
        Menu_add_separator(this.#handle);
    }

    get handle() {
        return this.#handle;
    }

}

/**
 * @typedef {IEvent<ResizeDetail>} IResizeEvent
 */
export class Window {

    /**
     * @type EventRegistry
     */
    #eventRegistry;

    /**
     * @type EventBinder
     */
    #eventBinder;

    #windowHandle;

    #body;

    /**
     *
     * @param attrs {WindowAttrs}
     */
    constructor(attrs = null) {
        attrs = attrs || {};
        attrs.preferredRenderers = [].concat(attrs.preferredRenderers || [])
        this.#windowHandle = Window_create(attrs);
        this.#eventBinder = new EventBinder(this.#windowHandle, Window_bind_js_event_listener, Window_unbind_js_event_listener, this);
        Window_set_js_context(this.#windowHandle, this);
        this.#body = new BodyElement();
        Window_set_body(this.#windowHandle, this.#body.handle);
    }

    /**
     *
     * @param windowHandle
     * @returns {Window}
     */
    static fromHandle(windowHandle) {
        return Window_get_js_context(windowHandle);
    }

    static supportMultipleWindows() {
        return Window_support_multiple_windows();
    }

    get handle() {
        return this.#windowHandle
    }

    /**
     *
     * @returns {BodyElement}
     */
    get body() {
        return this.#body;
    }


    /**
     *
     * @returns {{width: number, height: number}}
     */
    get innerSize() {
        const [width, height] = Window_get_inner_size(this.handle);
        return {width, height}
    }

    /**
     *
     * @param content {Element}
     * @param x {number}
     * @param y {number}
     * @return {Page}
     */
    createPage(content, x, y) {
        x = x ?? Number.NaN;
        y = y ?? Number.NaN;
        const page = Window_create_page(this.#windowHandle, content.handle, x, y);
        return new Page(page);
    }

    /**
     *
     * @param content {Element}
     * @param target {{x: number, y: number, width?: number, height?: number}}
     * @return {Popup}
     */
    popup(content, target) {
        const rect = {
            x: target.x,
            y: target.y,
            width: target.width || 0,
            height: target.height || 0,
        }
        const handle = Window_popup(this.handle, content.handle, rect);
        return new Popup(handle);
    }

    /**
     *
     * @param menu {Menu}
     * @param x {number}
     * @param y {number}
     */
    popupMenu(menu, x, y) {
        Window_popup_menu(this.#windowHandle, menu.handle, x, y);
    }

    /**
     *
     * @param message {string | Element}
     * @param options {AlertOptions}
     */
    showAlert(message, options = {}) {
        options = options || {};
        this.showConfirm(message, {
            ...options,
            hideCancel: true,
        }).finally(() => {
            options?.callback && options.callback();
        });
    }

    /**
     *
     * @param message {string | Element}
     * @param options {ConfirmOptions}
     * @returns {Promise<boolean>}
     */
    showConfirm(message, options = {}) {
        options = options || {};
        function createBtn(label) {
            const btnLabel = new LabelElement();
            btnLabel.text = label;
            const btn = new ButtonElement();
            btn.style = {
                minWidth: '4em',
                flexDirection: 'row',
                justifyContent: 'center',
            }
            btn.addChild(btnLabel);
            return btn;
        }
        return new Promise((resolve) => {
            if (!(message instanceof Element)) {
                const label = new LabelElement();
                label.text = message;
                label.style = {
                    padding: '2em',
                }
                message = label;
            }

            const footer = new ContainerElement();
            footer.style = {
                flexDirection: 'row',
                justifyContent: 'center',
                padding: '10px',
                gap: '2em',
            }
            const btn = createBtn(options?.confirmBtnText ?? "OK");
            footer.addChild(btn);

            let cancelBtn;
            if (!options.hideCancel) {
                cancelBtn = createBtn(options?.cancelBtnText ?? "Cancel");
                footer.addChild(cancelBtn);
            }

            const wrapper = new ContainerElement();
            wrapper.style = {
                minWidth: 200,
                alignItems: 'center',
            }
            wrapper.addChild(message);
            wrapper.addChild(footer);
            const dialog = this.showDialog(wrapper, options?.title);
            btn.bindClick(() => {
                resolve(true);
                dialog.close();
            });
            cancelBtn?.bindClick(() => {
                resolve(false);
                dialog.close();
            });
        })
    }

    /**
     *
     * @param content {Element}
     * @param title {string}
     * @returns {{close(): void}}
     */
    showDialog(content, title) {
        if (Window.supportMultipleWindows()) {
            const window = new Window({
                resizable: false,
                preferredRenderers: "SoftBuffer",
                minimizable: false,
                closable: false,
            });
            window.title = title ?? this.title ?? "";
            window.body.addChild(content);
            window.setModal(this);
            window.bindResize((e) => {
                const parentPos = this.outerPosition;
                const parentSize = this.innerSize;
                const x = parentPos.x + (parentSize.width - e.detail.width) / 2.0;
                const y = parentPos.y + (parentSize.height - e.detail.height) / 2.0;
                window.outerPosition = {x, y};
            });
            return {
                close() {
                    window.close();
                }
            }
        } else {
            const wrapper = new DialogElement();
            if (title) {
                const titleEl = new DialogTitleElement();
                const titleLabelEl = new LabelElement();
                titleLabelEl.text = title ?? window.title ?? "";
                titleEl.addChild(titleLabelEl);
                wrapper.addChild(titleEl);
            }
            wrapper.addChild(content);
            const page = this.createPage(wrapper, NaN, NaN);
            return {
                close() {
                    page.close();
                }
            }
        }
    }

    /**
     *
     * @param title {string}
     */
    set title(title) {
        Window_set_title(this.#windowHandle, title);
    }

    /**
     *
     * @returns {string}
     */
    get title() {
        return Window_get_title(this.#windowHandle);
    }

    /**
     *
     * @returns {{width: number, height: number}}
     */
    get monitorSize() {
        const [width, height] = Window_get_monitor_size(this.#windowHandle);
        return {
            width,
            height
        }
    }

    /**
     *
     * @param size {Size}
     */
    resize(size) {
        Window_resize(this.#windowHandle, size);
    }

    drag() {
        Window_drag(this.#windowHandle);
    }

    /**
     *
     * @returns {boolean}
     */
    get minimized() {
        return Window_is_minimized(this.#windowHandle);
    }

    /**
     *
     * @param minimized {boolean}
     */
    set minimized(minimized) {
        Window_set_minimized(this.#windowHandle, minimized);
    }

    /**
     *
     * @returns {boolean}
     */
    get maximized() {
        return Window_is_maximized(this.#windowHandle);
    }

    /**
     *
     * @param maximized {boolean}
     */
    set maximized(maximized) {
        Window_set_maximized(this.#windowHandle, maximized);
    }

    /**
     *
     * @param owner {Window}
     */
    setModal(owner) {
        Window_set_modal(this.#windowHandle, owner.#windowHandle)
    }

    /**
     *
     * @param value {{x: number, y: number}}
     */
    set outerPosition(value) {
        Window_set_outer_position(this.handle, value.x, value.y);
    }

    /**
     *
     * @returns {{x: number, y: number}}
     */
    get outerPosition() {
        const [x, y] = Window_get_outer_position(this.handle);
        return {x, y}
    }

    close() {
        Window_close(this.#windowHandle);
    }

    /**
     *
     * @param visible {boolean}
     */
    set visible(visible) {
        Window_set_visible(this.#windowHandle, visible);
    }

    /**
     *
     * @returns {boolean}
     */
    get visible() {
        return Window_is_visible(this.#windowHandle);
    }

    requestFullscreen() {
        Window_request_fullscreen(this.#windowHandle);
    }

    exitFullscreen() {
        Window_exit_fullscreen(this.#windowHandle);
    }

    get fullscreen() {
        return Window_is_fullscreen(this.#windowHandle);
    }

    /**
     *
     * @param callback {(event: IResizeEvent) => void}
     */
    bindResize(callback) {
        this.#eventBinder.bindEvent("resize", callback);
    }

    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindClose(callback) {
        this.bindEvent("close", callback);
    }

    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindFocus(callback) {
        this.bindEvent("focus", callback);
    }

    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindBlur(callback) {
        this.bindEvent("blur", callback);
    }

    bindEvent(type, callback) {
        this.#eventBinder.bindEvent(type, callback);
    }

    /**
     * @typedef {("resize", event)} addEventListener
     * @param type
     * @param callback
     */
    addEventListener(type, callback) {
        this.#eventBinder.addEventListener(type, callback);
    }

    removeEventListener(type, callback) {
        this.#eventBinder.removeEventListener(type, callback);
    }

}

/**
 * @template D
 * @template E
 */
export class EventObject {
    _propagationCancelled = false
    _preventDefault = false
    type;
    /**
     * @type {D}
     */
    detail;
    /**
     * @type {E}
     */
    target;

    /**
     * @type {E}
     */
    currentTarget;

    constructor(type, detail, target, currentTarget) {
        this.type = type;
        this.detail = detail;
        this.target = target;
        this.currentTarget = currentTarget;
    }

    stopPropagation() {
        this._propagationCancelled = true;
    }

    preventDefault() {
        this._preventDefault = true;
    }

    result() {
        return {
            propagationCancelled: this._propagationCancelled,
            preventDefault: this._preventDefault,
        }
    }

}

export class EventRegistry {
    eventListeners = Object.create(null);
    _id;
    _remove_api;
    _add_api;
    #contextGetter;
    #self;

    constructor(id, addApi, removeApi, self, contextGetter) {
        this._id = id;
        this._add_api = addApi;
        this._remove_api = removeApi;
        this.#contextGetter = contextGetter;
        this.#self = self;
    }

    bindEvent(type, callback) {
        type = type.toLowerCase();
        if (typeof callback !== "function") {
            throw new Error("invalid callback");
        }
        let oldListenerId = this.eventListeners[type];
        if (oldListenerId) {
            this._remove_api(this._id, type, oldListenerId);
        }

        const getJsContext = (target) => {
            if (target && this.#contextGetter) {
                return this.#contextGetter(target);
            }
            return target;
        }

        const self = this.#self;

        /**
         *
         * @param type {string}
         * @param detail {object}
         * @param target {unknown}
         * @returns {{propagationCancelled: boolean, preventDefault: boolean}}
         * @private
         */
        function eventCallback(type, detail, target) {
            const event = new EventObject(type, detail, getJsContext(target), self);
            try {
                callback && callback(event);
            } catch (error) {
                console.error(`${type} event handling error, detail=`, detail ,error.message || error);
            }
            return event.result();
        }

        this.eventListeners[type] = this._add_api(this._id, type, eventCallback);
    }
}

export class EventBinder {
    #eventListeners = Object.create(null);
    #target;
    #removeEventListenerApi;
    #addEventListenerApi;
    #contextGetter;
    #self;
    #allEventListeners = Object.create(null);

    constructor(target, addApi, removeApi, self, contextGetter) {
        this.#target = target;
        this.#addEventListenerApi = addApi;
        this.#removeEventListenerApi = removeApi;
        this.#contextGetter = contextGetter;
        this.#self = self;
    }

    bindEvent(type, callback) {
        type = type.toLowerCase();
        if (typeof callback !== "function") {
            throw new Error("invalid callback");
        }
        let oldListener = this.#eventListeners[type];
        if (oldListener) {
            this.removeEventListener(type, oldListener);
        }
        this.addEventListener(type, callback);
        this.#eventListeners[type] = callback;
    }
    addEventListener(type, callback) {
        const getJsContext = (target) => {
            try {
                if (target && this.#contextGetter) {
                    return this.#contextGetter(target);
                }
                return target;
            } catch (error) {
                return target;
            }
        }

        const self = this.#self;

        /**
         *
         * @param detail {object}
         * @param target {unknown}
         * @returns {{propagationCancelled: boolean, preventDefault: boolean}}
         * @private
         */
        function eventCallback(detail, target) {
            const event = new EventObject(type, detail, getJsContext(target), self);
            try {
                callback && callback(event);
            } catch (error) {
                console.error(`${type} event handling error, detail=`, detail ,error.message || error);
            }
            return event.result();
        }
        if (!this.#allEventListeners[type]) {
            this.#allEventListeners[type] = new Map();
        }
        const id = this.#addEventListenerApi(this.#target, type, eventCallback);
        this.#allEventListeners[type].set(callback, id);
        return id;
    }

    removeEventListener(type, callback) {
        /**
         * @type {Map}
         */
        const map = this.#allEventListeners[type];
        const id = map.get(callback);
        if (id) {
            map.delete(callback);
            this.#removeEventListenerApi(this.#target, id);
        }
    }

}

export class SystemTray {
    /**
     * @type EventRegistry
     */
    #eventRegistry;

    #menuUserCallback;

    tray;
    constructor() {
        this.tray = SystemTray_create("Test");
        this.#eventRegistry = new EventRegistry(this.tray, SystemTray_bind_event, SystemTray_remove_event_listener, this);
    }

    set title(title) {
        SystemTray_set_title(this.tray, title);
    }

    set icon(icon) {
        SystemTray_set_icon(this.tray, icon);
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
        SystemTray_set_menus(this.tray, list);
        this.#eventRegistry.bindEvent("menuclick", menuHandler)
    }

    setShowMenuOnLeftClick(value) {
        SystemTray_set_show_menu_on_left_click(this.tray, value);
    }

    bindActivate(callback) {
        this.#eventRegistry.bindEvent("activate", callback);
    }

    bindMenuClick(callback) {
        this.#menuUserCallback = callback;
    }

}
export class Element {
    /**
     * @type {ContainerBasedElement}
     */
    _parent
    /**
     * @type number
     */
    handle

    /**
     * @type {StyleProps}
     */
    #style

    /**
     * @type {StyleProps}
     */
    #hoverStyle

    /**
     * @type EventBinder
     */
    #eventBinder;

    /**
     *
     * @param el {any}
     * @param context {object}
     */
    constructor(el, context) {
        const myContext = this;
        if (typeof el === "string") {
            this.handle = Element_create_by_tag(el, myContext);
        } else {
            Element_set_js_context(el, myContext);
            this.handle = el;
        }
        if (!this.handle) {
            throw new Error("Failed to create element:" + el)
        }
        this.#eventBinder = new EventBinder(this.handle, Element_add_js_event_listener, Element_remove_js_event_listener, this, (target) => {
            return Element.fromHandle(target);
        });
    }

    static fromHandle(elementHandle) {
        if (elementHandle) {
            return Element_get_js_context(elementHandle) || null;
        }
        return null;
    }

    createEventBinder(target, addEventListenerApi, removeEventListenerApi) {
        if (!removeEventListenerApi) {
            removeEventListenerApi = (_t, listenerId) => {
                Element_remove_js_event_listener(this.handle, listenerId);
            }
        }
        return new EventBinder(target, addEventListenerApi, removeEventListenerApi, this, (target) => {
            return Element.fromHandle(target);
        });
    }

    /**
     * Get the eid of the element
     * @returns {number}
     */
    get eid() {
        return Element_get_eid(this.handle)
    }

    /**
     *
     * @param clazz {string}
     */
    set class(clazz) {
        Element_set_class(this.handle, clazz);
    }

    /**
     *
     * @returns {string}
     */
    get class() {
        return Element_get_class(this.handle);
    }

    /**
     *
     * @param clazz {string}
     */
    set className(clazz) {
        Element_set_class(this.handle, clazz);
    }

    /**
     *
     * @returns {string}
     */
    get className() {
        return Element_get_class(this.handle);
    }

    /**
     * Get the parent of element
     * @returns {Element | null}
     */
    get parent() {
        const eh = Element_get_parent(this.handle);
        return Element.fromHandle(eh);
    }

    /**
     * Make element focusable or not
     * @param focusable {boolean}
     */
    set focusable(focusable) {
        Element_set_focusable(this.handle, focusable);
    }

    /**
     * Whether element is focusable
     * @returns {boolean}
     */
    get focusable() {
        return Element_is_focusable(this.handle);
    }

    /**
     * Get the root of current element
     * @returns {Element}
     */
    get rootElement() {
        let p = this.getParent();
        if (p == null) {
            return this;
        } else {
            return p.getRootElement();
        }
    }

    /**
     * Request focus on the current element
     */
    focus() {
        Element_focus(this.handle);
    }

    set tooltip(text) {
        Element_set_tooltip(this.handle, text);
    }

    get tooltip() {
        return Element_get_tooltip(this.handle);
    }

    /**
     * Get the window of element
     * @returns {Window}
     */
    get window() {
        const windowHandle = Element_get_window(this.handle);
        return Window.fromHandle(windowHandle);
    }

    /**
     * Set element style
     * @param style {StyleProps | string}
     */
    set style(style) {
        this.#style = style;
        Element_set_style(this.handle, style);
    }

    /**
     * Get element style
     * @returns {StyleProps}
     */
    get style() {
        return Element_get_style(this.handle);
    }

    /**
     * Set element style in hover state
     * @param style {StyleProps | string}
     */
    set hoverStyle(style) {
        this.#hoverStyle = style;
        Element_set_hover_style(this.handle, style);
    }

    /**
     * Get element style in hover state
     * @returns {StyleProps}
     */
    get hoverStyle() {
        return this.#hoverStyle;
    }

    /**
     * The scrollTop property gets or sets the number of pixels by which an element's content is scrolled from its top edge.
     * @param value {number}
     */
    set scrollTop(value) {
        Element_set_scroll_top(this.handle, value);
    }

    /**
     * The scrollTop property gets or sets the number of pixels by which an element's content is scrolled from its top edge.
     * @returns {number}
     */
    get scrollTop() {
        return Element_get_scroll_top(this.handle);
    }

    /**
     * The scrollLeft property gets or sets the number of pixels by which an element's content is scrolled from its left edge.
     * @param value {number}
     */
    set scrollLeft(value) {
        Element_set_scroll_left(this.handle, value);
    }

    /**
     * The scrollLeft property gets or sets the number of pixels by which an element's content is scrolled from its left edge.
     * @returns {number}
     */
    get scrollLeft() {
        return Element_get_scroll_left(this.handle);
    }

    /**
     * Make element draggable
     * @param value {boolean}
     */
    set draggable(value) {
        Element_set_draggable(this.handle, value);
    }

    /**
     * Whether element is draggable or not
     * @returns {*}
     */
    get draggable() {
        return Element_get_draggable(this.handle);
    }

    /**
     * Set the cursor in hover state
     * @param value {string}
     */
    set cursor(value) {
        Element_set_cursor(this.handle, value);
    }

    /**
     * Get the size of element
     * @returns {[number, number]}
     */
    get size() {
        return Element_get_size(this.handle);
    }

    /**
     *
     * @returns {[number, number]}
     */
    get contentSize() {
        return Element_get_real_content_size(this.handle);
    }

    /**
     *
     * @returns {ElementRect}
     */
    getBoundingClientRect() {
        return Element_get_bounding_client_rect(this.handle);
    }

    /**
     * The scrollWidth read-only property is a measurement of the height of an element's content, including content not visible on the screen due to overflow.
     * @returns {number}
     */
    get scrollHeight() {
        return Element_get_scroll_height(this.handle);
    }

    /**
     * The scrollWidth read-only property is a measurement of the width of an element's content, including content not visible on the screen due to overflow.
     * @returns {number}
     */
    get scrollWidth() {
        return Element_scroll_width(this.handle);
    }

    setAttribute(key, value) {
        Element_set_attribute(this.handle, key, value);
    }

    getAttribute(key) {
        return Element_get_attribute(this.handle, key);
    }

    removeAttribute(key) {
        Element_remove_attribute(this.handle, key);
    }

    /**
     *
     * @param callback {(event: IBoundsChangeEvent) => void}
     */
    bindBoundsChange(callback) {
        this.bindEvent("boundschange", callback);
    }

    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindFocus(callback) {
        this.bindEvent("focus", callback);
    }

    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindBlur(callback) {
        this.bindEvent("blur", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindClick(callback) {
        this.#eventBinder.bindEvent("click", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindContextMenu(callback) {
        this.#eventBinder.bindEvent("contextmenu", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseDown(callback) {
        this.#eventBinder.bindEvent("mousedown", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseUp(callback) {
        this.#eventBinder.bindEvent("mouseup", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseMove(callback) {
        this.#eventBinder.bindEvent("mousemove", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseEnter(callback) {
        this.#eventBinder.bindEvent("mouseenter", callback);
    }

    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseLeave(callback) {
        this.#eventBinder.bindEvent("mouseleave", callback);
    }

    /**
     *
     * @param callback {(e: IKeyEvent) => void}
     */
    bindKeyDown(callback) {
        this.#eventBinder.bindEvent("keydown", callback);
    }

    /**
     *
     * @param callback {(e: IKeyEvent) => void}
     */
    bindKeyUp(callback) {
        this.#eventBinder.bindEvent("keyup", callback);
    }

    bindSizeChanged(callback) {
        this.#eventBinder.bindEvent("sizechange", callback);
    }

    bindScroll(callback) {
        this.#eventBinder.bindEvent("scroll", callback);
    }

    bindMouseWheel(callback) {
        this.#eventBinder.bindEvent("mousewheel", callback);
    }

    bindDragStart(callback) {
        this.#eventBinder.bindEvent("dragstart", callback);
    }

    bindDragOver(callback) {
        this.#eventBinder.bindEvent("dragover", callback);
    }

    bindDrop(callback) {
        this.#eventBinder.bindEvent("drop", callback);
    }

    bindTouchStart(callback) {
        this.#eventBinder.bindEvent("touchstart", callback);
    }

    bindTouchMove(callback) {
        this.#eventBinder.bindEvent("touchmove", callback);
    }

    bindTouchEnd(callback) {
        this.#eventBinder.bindEvent("touchend", callback);
    }

    bindTouchCancel(callback) {
        this.#eventBinder.bindEvent("touchcancel", callback);
    }

    /**
     *
     * @param callback {(e: IDroppedFileEvent) => void}
     */
    bindDroppedFile(callback) {
        this.#eventBinder.bindEvent("droppedfile", callback);
    }

    /**
     *
     * @param callback {(e: IHoveredFileEvent) => void}
     */
    bindHoveredFile(callback) {
        this.#eventBinder.bindEvent("hoveredfile", callback);
    }

    bindEvent(type, callback) {
        this.#eventBinder.bindEvent(type, callback);
    }

    /**
     *
     * @param value {boolean}
     */
    set autoFocus(value) {
        Element_set_auto_focus(this.handle, value);
    }

    /**
     *
     * @returns {boolean}
     */
    get autoFocus() {
        return Element_get_auto_focus(this.handle);
    }


    toString() {
        return this.handle + "@" + this.constructor.name
    }

}

export class Audio {
    context;
    #eventRegistry;
    id;
    constructor(config) {
        this.id = Audio_create(config || {})
        this.#eventRegistry = new EventRegistry(this.id, Audio_add_event_listener, Audio_remove_event_listener, this);
    }

    play() {
        Audio_play(this.id);
    }

    pause() {
        Audio_pause(this.id);
    }

    stop() {
        Audio_stop(this.id);
    }

    bindLoad(callback) {
        this.#eventRegistry.bindEvent('load', callback);
    }

    bindTimeUpdate(callback) {
        this.#eventRegistry.bindEvent("timeupdate", callback);
    }

    bindEnd(callback) {
        this.#eventRegistry.bindEvent("end", callback);
    }

    bindPause(callback) {
        this.#eventRegistry.bindEvent("pause", callback);
    }

    bindStop(callback) {
        this.#eventRegistry.bindEvent("stop", callback);
    }

    bindCurrentChange(callback) {
        this.#eventRegistry.bindEvent("currentchange", callback);
    }

    bindEvent(type, callback) {
        this.#eventRegistry.bindEvent(type, callback);
    }

}

export class LabelElement extends Element {
    constructor() {
        super(VT_LABEL);
    }

    /**
     *
     * @param text {string}
     */
    set text(text) {
        Label_set_text(this.handle, text);
    }

}

export class CheckboxElement extends Element {
    constructor() {
        super(VT_CHECKBOX);
    }

    /**
     *
     * @param text {string}
     */
    set label(text) {
        Checkbox_set_label(this.handle, text);
    }

    /**
     *
     * @returns {string}
     */
    get label() {
        return Checkbox_get_label(this.handle);
    }

    /**
     *
     * @param value {boolean}
     */
    set checked(value) {
        Checkbox_set_checked(this.handle, value);
    }

    /**
     *
     * @returns {boolean}
     */
    get checked() {
        return Checkbox_is_checked(this.handle);
    }

    /**
     *
     * @returns {boolean}
     */
    get disabled() {
        return Element_is_disabled(this.handle);
    }

    /**
     *
     * @param value {boolean}
     */
    set disabled(value) {
        Element_set_disabled(this.handle, value);
    }

    /**
     *
     * @param callback {(e: IVoidEvent) => void}
     */
    bindChange(callback) {
        this.bindEvent("change", callback);
    }

}

export class RadioElement extends Element {
    constructor() {
        super(VT_RADIO);
    }

    /**
     *
     * @param text {string}
     */
    set label(text) {
        Radio_set_label(this.handle, text);
    }

    /**
     *
     * @returns {string}
     */
    get label() {
        return Radio_get_label(this.handle);
    }

    /**
     *
     * @param value {boolean}
     */
    set checked(value) {
        Radio_set_checked(this.handle, value);
    }

    /**
     *
     * @returns {boolean}
     */
    get checked() {
        return Radio_is_checked(this.handle);
    }

    /**
     *
     * @returns {boolean}
     */
    get disabled() {
        return Element_is_disabled(this.handle);
    }

    /**
     *
     * @param value {boolean}
     */
    set disabled(value) {
        Element_set_disabled(this.handle, value);
    }

    /**
     *
     * @param callback {(e: IVoidEvent) => void}
     */
    bindChange(callback) {
        this.bindEvent("change", callback);
    }

}

export class SelectElement extends Element {
    constructor() {
        super(VT_SELECT);
    }

    /**
     *
     * @param value {string}
     */
    set value(value) {
        Select_set_value(this.handle, value + "");
    }


    /**
     *
     * @returns {string}
     */
    get value() {
        return Select_get_value(this.handle);
    }

    /**
     *
     * @param options {SelectOption[]}
     */
    set options(options) {
        Select_set_options(this.handle, options);
    }

    /**
     *
     * @returns {SelectOption[]}
     */
    get options() {
        return Select_get_options(this.handle);
    }

    /**
     *
     * @param placeholder {string}
     */
    set placeholder(placeholder) {
        Select_set_placeholder(this.handle, placeholder);
    }

    /**
     *
     * @returns {string}
     */
    get placeholder() {
        return Select_get_placeholder(this.handle);
    }

    /**
     *
     * @returns {boolean}
     */
    get disabled() {
        return Element_is_disabled(this.handle);
    }

    /**
     *
     * @param value {boolean}
     */
    set disabled(value) {
        Element_set_disabled(this.handle, value);
    }

    bindChange(callback) {
        this.bindEvent("change", callback);
    }

}

/**
 * @typedef {{
 *   type: "text",
 *   text: string,
 *   weight ?: string,
 *   textDecorationLine ?: string,
 *   fontFamilies ?: string[],
 *   fontSize ?: number,
 *   color ?: string,
 *   backgroundColor ?: string
 * }} TextUnit
 */
export class RichTextElement extends Element {
    constructor() {
        super(VT_RICH_TEXT);
    }

    /**
     *
     * @param units {TextUnit[]}
     */
    addLine(units) {
        RichText_add_line(this.handle, units);
    }

    /**
     *
     * @param index {number}
     * @param units {TextUnit[]}
     */
    insertLine(index, units) {
        RichText_insert_line(this.handle, index, units);
    }


    /**
     *
     * @param index {number}
     */
    deleteLine(index) {
        RichText_delete_line(this.handle, index);
    }

    /**
     *
     * @param index {number}
     * @param units {TextUnit[]}
     */
    updateLine(index, units) {
        RichText_update_line(this.handle, index, units);
    }

    clear() {
        RichText_clear(this.handle);
    }

    /**
     *
     * @param units {TextUnit[]}
     * @return {[number, number]}
     */
    measureLine(units) {
        return RichText_measure_line(this.handle, units);
    }

    /**
     *
     * @returns {string | undefined}
     */
    get selectionText() {
        return RichText_get_selection_text(this.handle);
    }

}

export class ImageElement extends Element {
    constructor() {
        super(VT_IMAGE);
    }
    set src(src) {
        Image_set_src(this.handle, src);
    }
}

export class TextInputElement extends Element {

    constructor() {
        super(VT_TEXT_INPUT);
    }

    /**
     *
     * @param text {string}
     */
    set text(text) {
        TextInput_set_text(this.handle, text);
    }

    /**
     *
     * @param placeholder {string}
     */
    set placeholder(placeholder) {
        TextInput_set_placeholder(this.handle, placeholder);
    }

    get placeholder() {
        return TextInput_get_placeholder(this.handle);
    }

    /**
     *
     * @param type {"text"|"password"}
     */
    set type(type) {
        TextInput_set_type(this.handle, type);
    }

    /**
     *
     * @returns {"text" | "password"}
     */
    get type() {
        return TextInput_get_type(this.handle);
    }

    /**
     *
     * @returns {string}
     */
    get text() {
        return TextInput_get_text(this.handle);
    }

    /**
     *
     * @returns {boolean}
     */
    get disabled() {
        return Element_is_disabled(this.handle);
    }

    /**
     *
     * @param value {boolean}
     */
    set disabled(value) {
        Element_set_disabled(this.handle, value);
    }


    /**
     *
     * @param callback {(e: ITextEvent) => void}
     */
    bindTextChange(callback) {
        this.bindEvent("textchange", callback);
    }

    /**
     *
     * @param callback {(e: ICaretEvent) => void}
     */
    bindCaretChange(callback) {
        this.bindEvent("caretchange", callback);
    }

}

export class TextEditElement extends Element {
    constructor() {
        super(VT_TEXT_EDIT);
    }

    /**
     *
     * @param text {string}
     */
    set text(text) {
        TextEdit_set_text(this.handle, text);
    }

    /**
     *
     * @param placeholder {string}
     */
    set placeholder(placeholder) {
        TextEdit_set_placeholder(this.handle, placeholder);
    }

    get placeholder() {
        return TextEdit_get_placeholder(this.handle);
    }

    /**
     *
     * @param start {number}
     * @param end {number}
     */
    setSelectionByCharOffset(start, end) {
        TextEdit_set_selection_by_char_offset(this.handle, start, end)
    }

    /**
     *
     * @param charOffset {number}
     */
    setCaretByCharOffset(charOffset) {
        TextEdit_set_caret_by_char_offset(this.handle, charOffset);
    }

    /**
     *
     * @returns {string}
     */
    get text() {
        return TextEdit_get_text(this.handle);
    }

    /**
     *
     * @returns {boolean}
     */
    get disabled() {
        return Element_is_disabled(this.handle);
    }

    /**
     *
     * @param value {boolean}
     */
    set disabled(value) {
        Element_set_disabled(this.handle, value);
    }


    /**
     *
     * @param callback {(e: ITextEvent) => void}
     */
    bindTextChange(callback) {
        this.bindEvent("textchange", callback);
    }

    /**
     *
     * @param callback {(e: ICaretEvent) => void}
     */
    bindCaretChange(callback) {
        this.bindEvent("caretchange", callback);
    }

}

class ContainerBasedElement extends Element {
    #children = [];
    
    /**
     *
     * @param child {Element}
     * @param index {number}
     */
    addChild(child, index= -1) {
        if (child._parent === this) {
            const oldIndex = this.#children.indexOf(child);
            if (oldIndex === index) {
                return;
            }
            index -= oldIndex < index ? 1 : 0;
            this.removeChild(child);
            this.addChild(child, index);
            return;
        }
        if (child._parent) {
            child._parent.removeChild(child);
        }
        child._parent = this;
        if (typeof index === "number" && index >= 0 && index < this.#children.length) {
            Element_add_child(this.handle, child.handle, index);
            this.#children.splice(index, 0, child);
        } else {
            Element_add_child(this.handle, child.handle, -1);
            this.#children.push(child);
        }
    }

    /**
     *
     * @param newNode {Element}
     * @param referenceNode {Element}
     */
    addChildBefore(newNode, referenceNode) {
        const index = this.#children.indexOf(referenceNode);
        this.addChild(newNode, index);
    }

    /**
     *
     * @param newNode {Element}
     * @param referenceNode {Element}
     */
    addChildAfter(newNode, referenceNode) {
        const index = this.#children.indexOf(referenceNode);
        if (index >= 0) {
            this.addChild(newNode, index + 1);
        } else {
            this.addChild(newNode);
        }
    }

    /**
     *
     * @param child {Element}
     */
    removeChild(child) {
        const index = this.#children.indexOf(child);
        if (index >= 0) {
            child._parent = null;
            Element_remove_child(this.handle, index);
            this.#children.splice(index, 1);
        } else {
            console.log("remove child failed")
        }
    }

    /**
     *
     * @returns {Element[]}
     */
    get children() {
        return this.#children.slice();
    }

}

export class ButtonElement extends ContainerBasedElement {
    constructor() {
        super(VT_BUTTON);
    }

    /**
     *
     * @returns {boolean}
     */
    get disabled() {
        return Element_is_disabled(this.handle);
    }

    /**
     *
     * @param value {boolean}
     */
    set disabled(value) {
        Element_set_disabled(this.handle, value);
    }

}

export class ContainerElement extends ContainerBasedElement {
    constructor() {
        super(VT_CONTAINER);
    }
}

export class DialogElement extends ContainerBasedElement {
    constructor() {
        super(VT_DIALOG);
    }
}

export class DialogTitleElement extends ContainerBasedElement {
    constructor() {
        super(VT_DIALOG_TITLE);
    }
}

export class BodyElement extends ContainerBasedElement {
    constructor() {
        super(VT_BODY);
    }
}

export class ScrollElement extends ContainerBasedElement {
    constructor() {
        super(VT_SCROLL);
    }

    /**
     *
     * @param value {"auto"|"always"|"never"}
     */
    set scrollX(value) {
        Scroll_set_scroll_x(this.handle, value);
    }

    /**
     *
     * @param value {"auto"|"always"|"never"}
     */
    set scrollY(value) {
        Scroll_set_scroll_y(this.handle, value);
    }

    scrollBy(value) {
        value.x = value.x || 0;
        value.y = value.y || 0;
        Element_scroll_by(this.handle, value);
    }

}

export class RadioGroupElement extends ContainerBasedElement {
    constructor() {
        super(VT_RADIO_GROUP);
    }
}

export class WebSocket {

    client;

    listeners;

    onopen;

    onclose;

    onmessage;

    onping;

    onpong;

    onerror;

    #closed = false;

    constructor(url) {
        this.listeners = Object.create(null);
        this.#connect(url);
    }

    addEventListener(name, callback) {
        if (!this.listeners[name]) {
            this.listeners[name] = [];
        }
        const listeners = this.listeners[name]
        listeners.push(callback);
    }

    async send(data) {
        try {
            await WsConnection_send_str(this.client, data + "");
        } catch (error) {
            this.#emit('error', error);
        }
    }

    close() {
        if (!this.#closed) {
            this.#closed = true;
            this.#emit("close");
            WsConnection_close(this.client);
        }
    }

    async #connect(url) {
        try {
            this.client = await WsConnection_connect(url);
            this.#emit("open");
            this.#doRead();
        } catch (error) {
            this.#emit("error", error);
        }

    }

    async #doRead() {
        try {
            loop:
            for (;;) {
                let [type, data] = await WsConnection_read(this.client);
                // console.log("read message", type, data);
                switch (type) {
                    case "text":
                        this.#emit("message", data);
                        break;
                    case "binary":
                        this.#emit("message", ArrayBuffer.from(data));
                        break;
                    case "ping":
                        this.#emit("ping", data);
                        break;
                    case "pong":
                        this.#emit("pong", data);
                        break;
                    case "close":
                        break loop;
                    case "frame":
                        this.#emit("frame", data);
                        break;
                }
            }
            //TODO maybe half-close?
            this.close();
        } catch (error) {
            console.error(error);
            this.#emit("error");
            this.close();
        }
    }

    #emit(name, data) {
        // console.log("emit", name, data);
        /**
         * @type {Event}
         */
        let event = {
            bubbles: false,
            cancelBubble: false,
            cancelable: false,
            composed: false,
            currentTarget: null,
            eventPhase: 0,
            isTrusted: true,
            returnValue: false,
            srcElement: null,
            target: null,
            timeStamp: new Date().getTime(),
            type: name,
            data,
        };
        const key = `on${name}`;
        if (this[key]) {
            try {
                this[key](event)
            } catch (error) {
                console.error(error);
            }
        }
        for (const listener of this.listeners[name] || []) {
            try {
                listener(event);
            } catch (error) {
                console.error(error);
            }
        }
    }

}

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
        this.#worker = typeof source === "string" ? Worker_create(source) : Worker_bind(source);
        this.#eventBinder = new EventBinder(
            this.#worker,
            Worker_bind_js_event_listener,
            Worker_remove_js_event_listener,
            this
        );
    }

    postMessage(data) {
        Worker_post_message(this.#worker, JSON.stringify(data));
    }

    bindMessage(callback) {
        this.#eventBinder.bindEvent('message', e => {
            e.data = JSON.parse(e.detail.data);
            callback(e);
        });
    }

}

export class WorkerContext {
    #workerContext;
    /**
     * @type {EventBinder}
     */
    #eventBinder;
    constructor() {
        this.#workerContext = WorkerContext_get();
        this.#eventBinder = new EventBinder(
            this.#workerContext,
            WorkerContext_bind_js_event_listener,
            WorkerContext_remove_js_event_listener,
            this
        )
    }
    postMessage(data) {
        WorkerContext_post_message(this.#workerContext, JSON.stringify(data));
    }
    bindMessage(callback) {
        this.#eventBinder.bindEvent('message', e => {
            e.data = JSON.parse(e.detail.data);
            callback(e);
        });
    }

    static create() {
        if (globalThis.WorkerContext_get) {
            return new WorkerContext();
        }
        return null;
    }
}

export class SqliteConn {
    #conn;
    constructor(conn) {
        this.#conn = conn;
    }

    /**
     *
     * @param sql {string}
     * @param params {*[]}
     * @returns {Promise<number>}
     */
    async execute(sql, params= []) {
        return await SqliteConn_execute(this.#conn, sql, params);
    }

    /**
     *
     * @param sql {string}
     * @param params {*[]}
     * @returns {Promise<Object[]>}
     */
    async query(sql, params = []) {
        const [columnNames, rows] = await SqliteConn_query(this.#conn, sql, params);
        return rows.map(it => {
            const map = {};
            for (let i = 0; i < columnNames.length; i++) {
                map[columnNames[i]] = it[i];
            }
            return map;
        });
    }

}

export class Sqlite {

    /**
     *
     * @param path {string}
     * @returns {Promise<SqliteConn>}
     */
    static async open(path) {
        const conn = SqliteConn_create();
        await SqliteConn_open(conn, path);
        return new SqliteConn(conn);
    }

}

function collectCircleRefInfo(value, visited, circleRefList, level) {
    if (level >= 3) {
        return;
    }
    if (value && typeof value === "object") {
        if (visited.includes(value)) {
            circleRefList.push(value);
            return;
        } else {
            visited.push(value);
        }
        Object.entries(value).forEach(([k, v]) => {
            collectCircleRefInfo(v, visited, circleRefList, level + 1);
        })
    }
}

function log(...values) {
    values.forEach((value, index) => {
        const visited = [];
        const circleRefList = [];
        collectCircleRefInfo(value, visited, circleRefList, 0);
        printObj(value, "", circleRefList, [], 0);
        if (index < values.length - 1) {
            printObj(",")
        }
    })
    Console_print("\n");
}

function printObj(value, padding, circleRefList, printedList, level) {
    let type = typeof value;
    if (value instanceof Error) {
        console.log(`[Error(${value.name})]` + value.message);
        if (value.stack) {
            console.log(value.stack);
        }
    } else if (type === "object" && value != null) {
        const refIdx = circleRefList.indexOf(value);
        if (refIdx >= 0 && printedList.includes(value)) {
            Console_print("[Circular *" + refIdx + "]");
        } else {
            const entries = Object.entries(value);
            if (level >= 2) {
                return "{...}"
            }
            if (!entries.length) {
                Console_print("{}");
            } else {
                const prefix = refIdx >= 0 ? ("<ref *" + refIdx + ">") : "";
                Console_print(prefix + "{\n");
                printedList.push(value);
                entries.forEach(([k, v], index) => {
                    Console_print(padding + "  " + k + ":");
                    printObj(v, padding + "  ", circleRefList, printedList, level + 1);
                    if (index < entries.length - 1) {
                        Console_print(",\n");
                    }
                });
                Console_print("\n" + padding + "}");
            }
        }
    } else if (type === "symbol") {
        console.log("[Symbol]")
    } else if (type === "function") {
        console.log("[Function]")
    } else {
        Console_print(value + "");
    }
}
globalThis.console = {
    trace: log,
    debug: log,
    log,
    info: log,
    warn: log,
    error: log,
}

/**
 *
 * @type {LocalStorage}
 */
const localStorage = {
    getItem(key) {
        return localstorage_get(key)
    },
    setItem(key, value) {
        localstorage_set(key, value);
    }
}
globalThis.localStorage = localStorage;

export const workerContext = WorkerContext.create();
if (workerContext) {
    globalThis.workerContext = workerContext;
}

export class FetchResponse {
    _resp;

    constructor(resp, status) {
        this._resp = resp;
        this.status = status;
        this.ok = this.status >= 200 && this.status < 300;
    }

    async json() {
        const body = await fetch_response_body_string(this._resp);
        return JSON.parse(body);
    }

}

/**
 *
 * @param url {string}
 * @param options {FetchOptions}
 * @returns {Promise<FetchResponse>}
 */
async function fetch(url, options) {
    const resp = await fetch_create(url, options);
    let status = await fetch_response_status(resp);
    return new FetchResponse(resp, status);
}

globalThis.fetch = fetch;

globalThis.navigator = new Navigator();
globalThis.process = new Process();
globalThis.process.setPromiseRejectionTracker(error => {
    console.error('uncaught promise error', error);
});
globalThis.fileDialog = new FileDialog();
globalThis.Worker = Worker;
globalThis.WorkerContext = WorkerContext;
globalThis.Window = Window;
if (globalThis.SystemTray_create) {
    globalThis.SystemTray = SystemTray;
}
globalThis.Menu = Menu;
globalThis.StandardMenuItem = StandardMenuItem;
globalThis.Element = Element;
globalThis.ContainerElement = ContainerElement;
globalThis.ScrollElement = ScrollElement;
globalThis.LabelElement = LabelElement;
globalThis.TextInputElement = TextInputElement;
globalThis.TextEditElement = TextEditElement;
globalThis.ButtonElement = ButtonElement;
globalThis.ImageElement  = ImageElement;
globalThis.RichTextElement = RichTextElement;
globalThis.CheckboxElement = CheckboxElement;
globalThis.RadioElement = RadioElement;
globalThis.RadioGroupElement = RadioGroupElement;
globalThis.SelectElement = SelectElement;
globalThis.Audio = Audio;
globalThis.WebSocket = WebSocket;
globalThis.Sqlite = Sqlite;

globalThis.setTimeout = globalThis.timer_set_timeout;
globalThis.clearTimeout = globalThis.timer_clear_timeout;
globalThis.setInterval = globalThis.timer_set_interval;
globalThis.clearInterval = globalThis.timer_clear_interval;
globalThis.KEY_MOD_CTRL = 0x1;
globalThis.KEY_MOD_ALT = 0x1 << 1;
globalThis.KEY_MOD_META = 0x1 << 2;
globalThis.KEY_MOD_SHIFT = 0x1 << 3;

/**
 * @template T
 * @typedef {{
 *     detail: T,
 *     target: Element,
 *     currentTarget: Element,
 *     stopPropagation(): void,
 *     preventDefault(): void,
 * }} IEvent<T>
 */

/**
 * @typedef {IEvent<BoundsChangeDetail>} IBoundsChangeEvent
 * @typedef {IEvent<void>} IVoidEvent
 * @typedef {IEvent<CaretDetail>} ICaretEvent
 * @typedef {IEvent<MouseDetail>} IMouseEvent
 * @typedef {IEvent<KeyDetail>} IKeyEvent
 * @typedef {IEvent<MouseWheelDetail>} IMouseWheelEvent
 * @typedef {IEvent<TextDetail>} ITextEvent
 * @typedef {IEvent<TouchDetail>} ITouchEvent
 * @typedef {IEvent<ScrollDetail>} IScrollEvent
 * @typedef {IEvent<string>} IDroppedFileEvent
 * @typedef {IEvent<string>} IHoveredFileEvent
 */