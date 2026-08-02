declare type WindowType = "normal" | "menu"
declare type RenderBackend = "SoftBuffer" | "GL" | "SoftGL"
declare interface WindowAttrs {
    width ?: number
    height ?: number
    title ?: string
    resizable ?: boolean,
    decorations ?: boolean,
    overrideRedirect ?: boolean,
    position ?: [number, number],
    visible ?: boolean,
    windowType ?: WindowType,
    minimizable ?: boolean,
    maximizable ?: boolean,
    closable ?: boolean,
    preferredRenderers ?: RenderBackend | RenderBackend[],
}

declare interface ResizeDetail {
    width: number;
    height: number;
}

declare interface ElementRect {
    x: number;
    y: number;
    width: number;
    height: number;
}

declare interface Size {
    width: number;
    height: number;
}

declare interface MouseDetail {
    button: number,
    offsetX: number;
    offsetY: number;
    windowX: number;
    windowY: number;
    screenX: number;
    screenY: number;
}

declare interface CaretDetail {
    position: number,
    originBounds: ElementRect,
    bounds: ElementRect,
}

declare interface KeyDetail {
    modifiers: number,
    ctrlKey: boolean,
    altKey: boolean,
    metaKey: boolean,
    shiftKey: boolean,
    key: string,
    keyStr: string,
    repeat: boolean,
    pressed: boolean,
}

declare interface MouseWheelDetail {
    cols: number;
    rows: number;
}

declare interface TextDetail {
    value: string;
}


declare interface TextChangeDetail {
    value: string;
}

declare interface ScrollDetail {
    scrollTop: number;
    scrollLeft: number;
}

declare interface BoundsChangeDetail {
    originBounds: ElementRect,
}

declare interface AppReopenDetail {
    hasVisible: boolean,
}

declare interface TouchInfo {
    identifier: number;
    offsetX: number;
    offsetY: number;
    windowX: number;
    windowY: number;
}

declare interface TouchDetail {
    touches: TouchInfo[],
}

declare type Align =
    'auto'
    | 'flex-start'
    | 'center'
    | 'flex-end'
    | 'stretch'
    | 'baseline'
    | 'space-between'
    | 'space-around'

declare interface SelectOption {
    label: string,
    value: string,
}

declare interface AlertOptions {
    title ?: string;
    confirmBtnText ?: string;
    callback ?: () => void;
}

declare interface ConfirmOptions {
    title ?: string;
    confirmBtnText ?: string;
    cancelBtnText ?: string;
    hideCancel ?: boolean;
}

declare interface StyleProps extends Record<string, number | string>{
    color?: string,
    backgroundColor?: string;
    fontSize?: number;
    lineHeight?: number;

    borderTop?: string;
    borderRight?: string;
    borderBottom?: string;
    borderLeft?: string;

    display?: "none" | "flex",

    width?: number | string,
    height?: number | string,
    maxWidth?: number | string,
    maxHeight?: number | string,
    minWidth?: number | string,
    minHeight?: number | string,

    marginTop?: number | string,
    marginRight?: number | string,
    marginBottom?: number | string,
    marginLeft?: number | string,

    paddingTop?: number | string,
    paddingRight?: number | string,
    paddingBottom?: number | string,
    paddingLeft?: number | string,

    flex?: number,
    flexBasis?: number | string,
    flexGrow?: number,
    flexShrink?: number,
    alignSelf?: Align,
    direction?: 'inherit' | 'ltr' | 'rtl',
    position?: 'static' | 'relative' | 'absolute',
    overflow?: 'visible' | 'hidden' | 'scroll',

    borderTopLeftRadius?: number,
    borderTopRightRadius?: number,
    borderBottomRightRadius?: number,
    borderBottomLeftRadius?: number,

    justifyContent?: 'flex-start' | 'center' | 'flex-end' | 'space-between' | 'space-around' | 'space-evenly',
    flexDirection?: 'column' | 'column-reverse' | 'row' | 'row-reverse',
    alignContent?: Align,
    alignItems?: Align,
    flexWrap?: 'no-wrap' | 'wrap' | 'wrap-reverse',
    columnGap?: number,
    rowGap?: number,
    top?: number | string,
    right?: number | string,
    bottom?: number | string,
    left?: number | string,
    transform?: string,
    animationName?: string,
    animationDuration?: number,
    animationIterationCount?: number,

    // short hands
    background?: string,
    gap?: number,
    border?: string,
    margin?: number | string,
    padding?: number | string,
    borderRadius?: number | string,
}

declare interface LocalStorage {
    getItem(key: string): string | null,

    setItem(key: string, value: string): void,
}
// @ts-ignore
declare const localStorage: LocalStorage;

declare interface TrayMenu {
    kind ?: "standard" | "checkmark" | "separator"
    id ?: string,
    label ?: string,
    checked ?: boolean,
    enabled ?: boolean,
    handler ?: () => void,
}

declare module "deft:path" {
    export function filename(path: string): string;
    export function join(path: string, other: string): string;
}

declare function animation_create(name: string, keyFrames: Record<string, Record<string, any>>)

declare interface TypefaceParams {
    family: string,
    weight?: string,
}

// declare function typeface_create(name: string, params: TypefaceParams): boolean;

declare module "deft:env" {
    export function exeDir(): string;
    export function exePath(): string;
}

declare interface UploadOptions {
    file: string,
    field: string,
    data ?: Record<string, string>,
    headers ?: Record<string, string>,
}
declare function http_upload(url: string, options: UploadOptions) : Promise<{status: number, body: string}>;
declare function http_request(url: string) : Promise<any>;
declare interface FetchOptions {
    method ?: 'GET' | 'POST',
    headers ?: Record<string, string>,
    body ?: string,
    proxy ?: string,
}

// declare function fetch_create(url: string, options ?: FetchOptions) : Promise<any>;
// declare function fetch_response_status(rsp): Promise<number>;
// declare function fetch_response_headers(rsp): Promise<{name: string, value: string}[]>;
// declare function fetch_response_save(rsp, path: string): Promise<number>;
// declare function fetch_response_body_string(rsp): Promise<string>;

declare function AudioRef_create(path: string);
declare function AudioRef_destroy(id): void;
declare function AudioRef_position(id): number;
declare function AudioRef_duration(id): number;

declare function Base64_encode_str(str: string): string;

declare interface ShowFileDialogOptions {
    dialogType ?: "single" | "multiple" | "save" | "dir",
    //TODO fix type
    window ?: any,
}
declare function dialog_show_file_dialog(options ?: ShowFileDialogOptions): Promise<string[]>;

declare module "deft:fs" {
    export function readDir(path: string): Promise<string[]>;
    export function exists(path: string): Promise<boolean>;
    export function rename(path: string, dest:string): Promise<void>;
    export function deleteFile(path: string): Promise<void>;
    export function createDir(path: string): Promise<void>;
    export function createDirAll(path: string): Promise<void>;
    export function removeDir(path: string): Promise<void>;
    export function removeDirAll(path: string): Promise<void>;
}

declare module "deft:appfs" {
    export function dataPath(path ?: string): string;

    export function exists(path: string): Promise<boolean>;

    export function readdir(path: string): Promise<string[]>;

    export function read(path: string): Promise<string>;

    export function writeNew(path: string, content: string): Promise<void>;

    export function write(path: string, content: string): Promise<void>;

    export function deleteFile(path: string): Promise<void>;

    export function createDir(path: string): Promise<void>;

    export function createDirAll(path: string): Promise<void>;

    export function removeDir(path: string): Promise<void>;

    export function removeDirAll(path: string): Promise<void>;
}


// declare function shell_spawn(executable: string, args ?: string[]): void;

declare function setTimeout(callback: () => void, timeout: number): number;

declare function clearTimeout(timer: number): void;

declare function setInterval(callback: () => void, interval: number): number;

declare function clearInterval(timer: number): void;

declare class DeftNavigator {
    app: import("deft:core:jsapp").DeftApp;
    stylesheet: import("deft:core:stylesheet").Stylesheet;
    clipboard: import("deft:clipboard").Clipboard;
    fileDialog: import("deft:dialog").FileDialog;
}

//@ts-ignore
declare const navigator: DeftNavigator;
declare module 'deft:core' {
/**
 * @template D
 */
export class EventObject<D> {
    constructor(type: any, detail: any);
    _propagationCancelled: boolean;
    _preventDefault: boolean;
    type: any;
    /**
     * @type {D}
     */
    detail: D;
    stopPropagation(): void;
    preventDefault(): void;
    result(): {
        propagationCancelled: boolean;
        preventDefault: boolean;
    };
}
export class EventBinder {
    constructor(target: any, addApi: any, removeApi: any, self: any);
    bindEvent(type: any, callback: any): void;
    addEventListener(type: any, callback: any): any;
    removeEventListener(type: any, callback: any): void;
    
}
export class EventRegistry {
    constructor(id: any, addApi: any, removeApi: any, self: any);
    eventListeners: any;
    _id: any;
    _remove_api: any;
    _add_api: any;
    bindEvent(type: any, callback: any): void;
    
}
}

declare module 'deft:ui' {
/**
 * @template T
 * @typedef {{
 *     detail: T,
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
 * @typedef {IEvent<AppReopenDetail>} IAppReopenEvent
 */
/**
 * @typedef {IEvent<ResizeDetail>} IResizeEvent
 */
export class Window {
    /**
     *
     * @param windowHandle
     * @returns {Window}
     */
    static fromHandle(windowHandle: any): Window;
    static supportMultipleWindows(): any;
    /**
     *
     * @param attrs {WindowAttrs}
     */
    constructor(attrs?: WindowAttrs);
    get handle(): any;
    /**
     *
     * @returns {BodyWidget}
     */
    get body(): BodyWidget;
    /**
     *
     * @returns {{width: number, height: number}}
     */
    get innerSize(): {
        width: number;
        height: number;
    };
    /**
     *
     * @param content {Widget}
     * @param x {number}
     * @param y {number}
     * @return {Page}
     */
    createPage(content: Widget, x: number, y: number): Page;
    /**
     *
     * @param content {Widget}
     * @param target {{x: number, y: number, width?: number, height?: number}}
     * @return {Popup}
     */
    popup(content: Widget, target: {
        x: number;
        y: number;
        width?: number;
        height?: number;
    }): Popup;
    /**
     *
     * @param menu { import('deft:menu').Menu }
     * @param x {number}
     * @param y {number}
     */
    popupMenu(menu: import("deft:menu").Menu, x: number, y: number): void;
    /**
     *
     * @param message {string | Widget}
     * @param options {AlertOptions}
     */
    showAlert(message: string | Widget, options?: AlertOptions): void;
    /**
     *
     * @param message {string | Widget}
     * @param options {ConfirmOptions}
     * @returns {Promise<boolean>}
     */
    showConfirm(message: string | Widget, options?: ConfirmOptions): Promise<boolean>;
    /**
     *
     * @param content {Widget}
     * @param title {string}
     * @returns {{close(): void}}
     */
    showDialog(content: Widget, title: string): {
        close(): void;
    };
    /**
     *
     * @param title {string}
     */
    set title(title: string);
    /**
     *
     * @returns {string}
     */
    get title(): string;
    /**
     *
     * @returns {{width: number, height: number}}
     */
    get monitorSize(): {
        width: number;
        height: number;
    };
    /**
     *
     * @param size {Size}
     */
    resize(size: Size): void;
    drag(): void;
    focus(): void;
    /**
     *
     * @param minimized {boolean}
     */
    set minimized(minimized: boolean);
    /**
     *
     * @returns {boolean}
     */
    get minimized(): boolean;
    /**
     *
     * @param maximized {boolean}
     */
    set maximized(maximized: boolean);
    /**
     *
     * @returns {boolean}
     */
    get maximized(): boolean;
    /**
     *
     * @param owner {Window}
     */
    setModal(owner: Window): void;
    /**
     *
     * @param value {{x: number, y: number}}
     */
    set outerPosition(value: {
        x: number;
        y: number;
    });
    /**
     *
     * @returns {{x: number, y: number}}
     */
    get outerPosition(): {
        x: number;
        y: number;
    };
    close(): void;
    /**
     *
     * @param visible {boolean}
     */
    set visible(visible: boolean);
    /**
     *
     * @returns {boolean}
     */
    get visible(): boolean;
    requestFullscreen(): void;
    exitFullscreen(): void;
    get fullscreen(): any;
    /**
     *
     * @param callback {(event: IResizeEvent) => void}
     */
    bindResize(callback: (event: IResizeEvent) => void): void;
    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindClose(callback: (event: IVoidEvent) => void): void;
    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindFocus(callback: (event: IVoidEvent) => void): void;
    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindBlur(callback: (event: IVoidEvent) => void): void;
    bindEvent(type: any, callback: any): void;
    /**
     * @param type
     * @param callback
     */
    addEventListener(type: any, callback: any): void;
    removeEventListener(type: any, callback: any): void;
    
}
export class Popup {
    constructor(handle: any);
    handle: any;
    close(): void;
}
export class Page {
    constructor(handle: any);
    handle: any;
    close(): void;
}
export class Widget {
    static fromHandle(elementHandle: any): any;
    /**
     *
     * @param el {any}
     * @param context {object}
     */
    constructor(el: any, context: object);
    /**
     * @type {ContainerBasedWidget}
     */
    _parent: ContainerBasedWidget;
    /**
     * @type number
     */
    handle: number;
    createEventBinder(target: any, addEventListenerApi: any, removeEventListenerApi: any): EventBinder;
    /**
     * Get the eid of the element
     * @returns {number}
     */
    get eid(): number;
    /**
     *
     * @param clazz {string}
     */
    set class(clazz: string);
    /**
     *
     * @returns {string}
     */
    get class(): string;
    /**
     *
     * @param clazz {string}
     */
    set className(clazz: string);
    /**
     *
     * @returns {string}
     */
    get className(): string;
    /**
     * Get the parent of element
     * @returns {Widget | null}
     */
    get parent(): Widget | null;
    /**
     * Make element focusable or not
     * @param focusable {boolean}
     */
    set focusable(focusable: boolean);
    /**
     * Whether element is focusable
     * @returns {boolean}
     */
    get focusable(): boolean;
    /**
     * Get the root of current element
     * @returns {Widget}
     */
    get rootWidget(): Widget;
    /**
     * Request focus on the current element
     */
    focus(): void;
    set tooltip(text: any);
    get tooltip(): any;
    /**
     * Get the window of element
     * @returns {Window}
     */
    get window(): Window;
    /**
     * Set element style
     * @param style {StyleProps | string}
     */
    set style(style: StyleProps | string);
    /**
     * Get element style
     * @returns {StyleProps}
     */
    get style(): StyleProps;
    /**
     * Set element style in hover state
     * @param style {StyleProps | string}
     */
    set hoverStyle(style: StyleProps | string);
    /**
     * Get element style in hover state
     * @returns {StyleProps}
     */
    get hoverStyle(): StyleProps;
    /**
     * The scrollTop property gets or sets the number of pixels by which an element's content is scrolled from its top edge.
     * @param value {number}
     */
    set scrollTop(value: number);
    /**
     * The scrollTop property gets or sets the number of pixels by which an element's content is scrolled from its top edge.
     * @returns {number}
     */
    get scrollTop(): number;
    /**
     * The scrollLeft property gets or sets the number of pixels by which an element's content is scrolled from its left edge.
     * @param value {number}
     */
    set scrollLeft(value: number);
    /**
     * The scrollLeft property gets or sets the number of pixels by which an element's content is scrolled from its left edge.
     * @returns {number}
     */
    get scrollLeft(): number;
    /**
     * Make element draggable
     * @param value {boolean}
     */
    set draggable(value: boolean);
    /**
     * Whether element is draggable or not
     * @returns {*}
     */
    get draggable(): any;
    /**
     * Set the cursor in hover state
     * @param value {string}
     */
    set cursor(value: string);
    /**
     * Get the size of element
     * @returns {[number, number]}
     */
    get size(): [number, number];
    /**
     *
     * @returns {[number, number]}
     */
    get contentSize(): [number, number];
    /**
     *
     * @returns {ElementRect}
     */
    getBoundingClientRect(): ElementRect;
    /**
     * The scrollWidth read-only property is a measurement of the height of an element's content, including content not visible on the screen due to overflow.
     * @returns {number}
     */
    get scrollHeight(): number;
    /**
     * The scrollWidth read-only property is a measurement of the width of an element's content, including content not visible on the screen due to overflow.
     * @returns {number}
     */
    get scrollWidth(): number;
    setAttribute(key: any, value: any): void;
    getAttribute(key: any): any;
    removeAttribute(key: any): void;
    /**
     *
     * @param callback {(event: IBoundsChangeEvent) => void}
     */
    bindBoundsChange(callback: (event: IBoundsChangeEvent) => void): void;
    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindFocus(callback: (event: IVoidEvent) => void): void;
    /**
     *
     * @param callback {(event: IVoidEvent) => void}
     */
    bindBlur(callback: (event: IVoidEvent) => void): void;
    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindClick(callback: (event: IMouseEvent) => void): void;
    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindContextMenu(callback: (event: IMouseEvent) => void): void;
    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseDown(callback: (event: IMouseEvent) => void): void;
    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseUp(callback: (event: IMouseEvent) => void): void;
    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseMove(callback: (event: IMouseEvent) => void): void;
    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseEnter(callback: (event: IMouseEvent) => void): void;
    /**
     *
     * @param callback {(event: IMouseEvent) => void}
     */
    bindMouseLeave(callback: (event: IMouseEvent) => void): void;
    /**
     *
     * @param callback {(e: IKeyEvent) => void}
     */
    bindKeyDown(callback: (e: IKeyEvent) => void): void;
    /**
     *
     * @param callback {(e: IKeyEvent) => void}
     */
    bindKeyUp(callback: (e: IKeyEvent) => void): void;
    bindSizeChanged(callback: any): void;
    bindScroll(callback: any): void;
    bindMouseWheel(callback: any): void;
    bindDragStart(callback: any): void;
    bindDragOver(callback: any): void;
    bindDrop(callback: any): void;
    bindTouchStart(callback: any): void;
    bindTouchMove(callback: any): void;
    bindTouchEnd(callback: any): void;
    bindTouchCancel(callback: any): void;
    /**
     *
     * @param callback {(e: IDroppedFileEvent) => void}
     */
    bindDroppedFile(callback: (e: IDroppedFileEvent) => void): void;
    /**
     *
     * @param callback {(e: IHoveredFileEvent) => void}
     */
    bindHoveredFile(callback: (e: IHoveredFileEvent) => void): void;
    bindEvent(type: any, callback: any): void;
    /**
     *
     * @param value {boolean}
     */
    set autoFocus(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get autoFocus(): boolean;
    toString(): string;
    
}
export class LabelWidget extends Widget {
    constructor();
    /**
     *
     * @param text {string}
     */
    set text(text: string);
}
export class CheckboxWidget extends Widget {
    constructor();
    /**
     *
     * @param text {string}
     */
    set label(text: string);
    /**
     *
     * @returns {string}
     */
    get label(): string;
    /**
     *
     * @param value {boolean}
     */
    set checked(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get checked(): boolean;
    /**
     *
     * @param value {boolean}
     */
    set disabled(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get disabled(): boolean;
    /**
     *
     * @param callback {(e: IVoidEvent) => void}
     */
    bindChange(callback: (e: IVoidEvent) => void): void;
}
export class RadioWidget extends Widget {
    constructor();
    /**
     *
     * @param text {string}
     */
    set label(text: string);
    /**
     *
     * @returns {string}
     */
    get label(): string;
    /**
     *
     * @param value {boolean}
     */
    set checked(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get checked(): boolean;
    /**
     *
     * @param value {boolean}
     */
    set disabled(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get disabled(): boolean;
    /**
     *
     * @param callback {(e: IVoidEvent) => void}
     */
    bindChange(callback: (e: IVoidEvent) => void): void;
}
export class SelectWidget extends Widget {
    constructor();
    /**
     *
     * @param value {string}
     */
    set value(value: string);
    /**
     *
     * @returns {string}
     */
    get value(): string;
    /**
     *
     * @param options {SelectOption[]}
     */
    set options(options: SelectOption[]);
    /**
     *
     * @returns {SelectOption[]}
     */
    get options(): SelectOption[];
    /**
     *
     * @param placeholder {string}
     */
    set placeholder(placeholder: string);
    /**
     *
     * @returns {string}
     */
    get placeholder(): string;
    /**
     *
     * @param value {boolean}
     */
    set disabled(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get disabled(): boolean;
    bindChange(callback: any): void;
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
export class RichTextWidget extends Widget {
    constructor();
    /**
     *
     * @param units {TextUnit[]}
     */
    addLine(units: TextUnit[]): void;
    /**
     *
     * @param index {number}
     * @param units {TextUnit[]}
     */
    insertLine(index: number, units: TextUnit[]): void;
    /**
     *
     * @param index {number}
     */
    deleteLine(index: number): void;
    /**
     *
     * @param index {number}
     * @param units {TextUnit[]}
     */
    updateLine(index: number, units: TextUnit[]): void;
    clear(): void;
    /**
     *
     * @param units {TextUnit[]}
     * @return {[number, number]}
     */
    measureLine(units: TextUnit[]): [number, number];
    /**
     *
     * @returns {string | undefined}
     */
    get selectionText(): string | undefined;
}
export class ImageWidget extends Widget {
    constructor();
    set src(src: any);
}
export class TextInputWidget extends Widget {
    constructor();
    /**
     *
     * @param text {string}
     */
    set text(text: string);
    /**
     *
     * @returns {string}
     */
    get text(): string;
    /**
     *
     * @param placeholder {string}
     */
    set placeholder(placeholder: string);
    get placeholder(): string;
    /**
     *
     * @param type {"text"|"password"}
     */
    set type(type: "text" | "password");
    /**
     *
     * @returns {"text" | "password"}
     */
    get type(): "text" | "password";
    /**
     *
     * @param value {boolean}
     */
    set disabled(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get disabled(): boolean;
    /**
     *
     * @param callback {(e: ITextEvent) => void}
     */
    bindTextChange(callback: (e: ITextEvent) => void): void;
    /**
     *
     * @param callback {(e: ICaretEvent) => void}
     */
    bindCaretChange(callback: (e: ICaretEvent) => void): void;
}
export class TextEditWidget extends Widget {
    constructor();
    /**
     *
     * @param text {string}
     */
    set text(text: string);
    /**
     *
     * @returns {string}
     */
    get text(): string;
    /**
     *
     * @param placeholder {string}
     */
    set placeholder(placeholder: string);
    get placeholder(): string;
    /**
     *
     * @param maxHistory {number}
     */
    set maxHistory(maxHistory: number);
    /**
     *
     * @returns {number}
     */
    get maxHistory(): number;
    /**
     *
     * @param start {number}
     * @param end {number}
     */
    setSelectionByCharOffset(start: number, end: number): void;
    /**
     *
     * @param charOffset {number}
     */
    setCaretByCharOffset(charOffset: number): void;
    /**
     *
     * @param value {boolean}
     */
    set disabled(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get disabled(): boolean;
    /**
     *
     * @param callback {(e: ITextEvent) => void}
     */
    bindTextChange(callback: (e: ITextEvent) => void): void;
    /**
     *
     * @param callback {(e: ICaretEvent) => void}
     */
    bindCaretChange(callback: (e: ICaretEvent) => void): void;
}
export class ContainerBasedWidget extends Widget {
    /**
     *
     * @param child {Widget}
     * @param index {number}
     */
    addChild(child: Widget, index?: number): void;
    /**
     *
     * @param newNode {Widget}
     * @param referenceNode {Widget}
     */
    addChildBefore(newNode: Widget, referenceNode: Widget): void;
    /**
     *
     * @param newNode {Widget}
     * @param referenceNode {Widget}
     */
    addChildAfter(newNode: Widget, referenceNode: Widget): void;
    /**
     *
     * @param child {Widget}
     */
    removeChild(child: Widget): void;
    /**
     *
     * @returns {Widget[]}
     */
    get children(): Widget[];
    
}
export class ButtonWidget extends ContainerBasedWidget {
    constructor();
    /**
     *
     * @param value {boolean}
     */
    set disabled(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get disabled(): boolean;
}
export class ContainerWidget extends ContainerBasedWidget {
    constructor();
}
export class DialogWidget extends ContainerBasedWidget {
    constructor();
}
export class DialogTitleWidget extends ContainerBasedWidget {
    constructor();
}
export class BodyWidget extends ContainerBasedWidget {
    constructor(creator: any);
}
export class RadioGroupWidget extends ContainerBasedWidget {
    constructor();
}
/**
 * <T>
 */
export type IEvent<T> = {
    detail: T;
    stopPropagation(): void;
    preventDefault(): void;
};
export type IBoundsChangeEvent = IEvent<BoundsChangeDetail>;
export type IVoidEvent = IEvent<void>;
export type ICaretEvent = IEvent<CaretDetail>;
export type IMouseEvent = IEvent<MouseDetail>;
export type IKeyEvent = IEvent<KeyDetail>;
export type IMouseWheelEvent = IEvent<MouseWheelDetail>;
export type ITextEvent = IEvent<TextDetail>;
export type ITouchEvent = IEvent<TouchDetail>;
export type IScrollEvent = IEvent<ScrollDetail>;
export type IDroppedFileEvent = IEvent<string>;
export type IHoveredFileEvent = IEvent<string>;
export type IAppReopenEvent = IEvent<AppReopenDetail>;
export type IResizeEvent = IEvent<ResizeDetail>;
export type TextUnit = {
    type: "text";
    text: string;
    weight?: string;
    textDecorationLine?: string;
    fontFamilies?: string[];
    fontSize?: number;
    color?: string;
    backgroundColor?: string;
};
import { EventBinder } from "deft:core";
}

declare module 'deft:menu' {
export class StandardMenuItem {
    constructor(label: any, callback: any);
    /**
     *
     * @param value {boolean}
     */
    set disabled(value: boolean);
    /**
     *
     * @returns {boolean}
     */
    get disabled(): boolean;
    get handle(): any;
    
}
export class Menu {
    /**
     *
     * @param item {StandardMenuItem}
     */
    addStandardItem(item: StandardMenuItem): void;
    addSeparator(): void;
    get handle(): any;
    
}
}

declare module 'deft:systemtray' {
export class SystemTray {
    tray: any;
    set title(title: any);
    set icon(icon: any);
    /**
     *
     * @param menus {TrayMenu[]}
     */
    setMenus(menus: TrayMenu[]): void;
    setShowMenuOnLeftClick(value: any): void;
    bindActivate(callback: any): void;
    bindMenuClick(callback: any): void;
    
}
}

declare module 'deft:process' {
export class Process {
    /**
     *
     * @param code {number}
     */
    exit(code: number): void;
    /**
     *
     * @param value {boolean}
     */
    setExitOnAllWindowsClosed(value: boolean): void;
    /**
     *
     * @returns {string[]}
     */
    get argv(): string[];
    /**
     *
     * @returns {boolean}
     */
    get isMobilePlatform(): boolean;
    /**
     *
     * @returns {string}
     */
    get platform(): string;
    /**
     *
     * @param handler {Function}
     */
    setPromiseRejectionTracker(handler: Function): void;
}
}

declare module 'deft:audio' {
export class Audio {
    constructor(config: any);
    context: any;
    id: any;
    play(): void;
    pause(): void;
    stop(): void;
    bindLoad(callback: any): void;
    bindTimeUpdate(callback: any): void;
    bindEnd(callback: any): void;
    bindPause(callback: any): void;
    bindStop(callback: any): void;
    bindCurrentChange(callback: any): void;
    bindEvent(type: any, callback: any): void;
    
}
}

declare module 'deft:clipboard' {
export class Clipboard {
    /**
     *
     * @returns {Promise<string>}
     */
    readText(): Promise<string>;
    /**
     *
     * @param text {string}
     * @returns {Promise<void>}
     */
    writeText(text: string): Promise<void>;
}
}

declare module 'deft:dialog' {
export class FileDialog {
    /**
     *
     * @param options {ShowFileDialogOptions}
     * @returns {Promise<string[]>}
     */
    show(options: ShowFileDialogOptions): Promise<string[]>;
}
}

declare module 'deft:core:stylesheet' {
export class StylesheetItem {
    constructor(id: any);
    id: any;
    update(code: any): void;
}
export class Stylesheet {
    /**
     *
     * @param code {string}
     * @returns {StylesheetItem}
     */
    append(code: string): StylesheetItem;
    /**
     *
     * @param stylesheet {StylesheetItem}
     */
    remove(stylesheet: StylesheetItem): void;
}
}

declare module 'deft:core:jsapp' {
export class DeftApp {
    /**
     *
     * @param callback {(event: import("deft:ui").IAppReopenEvent) => void}
     */
    bindReopen(callback: (event: import("deft:ui").IAppReopenEvent) => void): void;
    
}
}

declare module 'deft:sqlite' {
export class SqliteConn {
    constructor(conn: any);
    /**
     *
     * @param sql {string}
     * @param params {*[]}
     * @returns {Promise<number>}
     */
    execute(sql: string, params?: any[]): Promise<number>;
    /**
     *
     * @param sql {string}
     * @param params {*[]}
     * @returns {Promise<Object[]>}
     */
    query(sql: string, params?: any[]): Promise<any[]>;
    
}
export class Sqlite {
    /**
     *
     * @param path {string}
     * @returns {Promise<SqliteConn>}
     */
    static open(path: string): Promise<SqliteConn>;
}
}

/**
 *
 * @param url {string}
 * @param options {FetchOptions}
 * @returns {Promise<FetchResponse>}
 */
declare function fetch(url: string, options: FetchOptions): Promise<FetchResponse>;
declare class FetchResponse {
    constructor(resp: any, status: any);
    _resp: any;
    status: any;
    ok: boolean;
    json(): Promise<any>;
}

