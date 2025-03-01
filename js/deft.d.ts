declare type WindowType = "normal" | "menu"
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

declare function process_exit(code: number);
declare function path_filename(path: string): string;
declare function path_join(path: string, other: string): string;
declare function animation_create(name: string, keyFrames: Record<string, Record<string, any>>)

declare interface TypefaceParams {
    family: string,
    weight?: string,
}

declare function typeface_create(name: string, params: TypefaceParams): boolean;

declare function env_exe_dir(): String;
declare function env_exe_path(): String;


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
declare function fetch_create(url: string, options ?: FetchOptions) : Promise<any>;
declare function fetch_response_status(rsp): Promise<number>;
declare function fetch_response_headers(rsp): Promise<{name: string, value: string}[]>;
declare function fetch_response_save(rsp, path: string): Promise<number>;
declare function fetch_response_body_string(rsp): Promise<string>;

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

declare function fs_exists(path: string): Promise<boolean>;
declare function fs_rename(path: string, dest:string): Promise<void>;
declare function fs_delete_file(path: string): Promise<void>;
declare function fs_create_dir(path: string): Promise<void>;
declare function fs_create_dir_all(path: string): Promise<void>;
declare function fs_remove_dir(path: string): Promise<void>;
declare function fs_remove_dir_all(path: string): Promise<void>;


declare function appfs_data_path(path ?: string): string;
declare function appfs_exists(path: string): Promise<boolean>;

declare function appfs_readdir(path: string): Promise<string[]>;

declare function appfs_read(path: string): Promise<string>;

declare function appfs_write_new(path: string, content: string): Promise<void>;

declare function appfs_write(path: string, content: string): Promise<void>;

declare function appfs_delete_file(path: string): Promise<void>;

declare function appfs_create_dir(path: string): Promise<void>;

declare function appfs_create_dir_all(path: string): Promise<void>;

declare function appfs_remove_dir(path: string): Promise<void>;

declare function appfs_remove_dir_all(path: string): Promise<void>;

declare function shell_spawn(executable: string, args ?: string[]): void;

declare class Navigator {
    /**
     * @var {Clipboard}
     */
    clipboard: Clipboard;
}
declare class Process {
    /**
     *
     * @param code {number}
     */
    exit(code: number): void;
    get argv(): any;
    get isMobilePlatform(): any;
    /**
     *
     * @param handler {Function}
     */
    setPromiseRejectionTracker(handler: Function): void;
}
declare class FileDialog {
    /**
     *
     * @param options {ShowFileDialogOptions}
     * @returns {Promise<string[]>}
     */
    show(options: ShowFileDialogOptions): Promise<string[]>;
}
/**
 * @typedef {IEvent<ResizeDetail>} IResizeEvent
 */
declare class Window {
    /**
     *
     * @param windowHandle
     * @returns {Window}
     */
    static fromHandle(windowHandle: any): Window;
    /**
     *
     * @param attrs {WindowAttrs}
     */
    constructor(attrs: WindowAttrs);
    get handle(): any;
    /**
     *
     * @param element {Element}
     */
    set body(element: Element);
    /**
     *
     * @returns {Element}
     */
    get body(): Element;
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
     * @param size {Size}
     */
    resize(size: Size): void;
    /**
     *
     * @param owner {Window}
     */
    setModal(owner: Window): void;
    close(): void;
    /**
     *
     * @param visible {boolean}
     */
    set visible(visible: boolean);
    get visible(): boolean;
    requestFullscreen(): void;
    exitFullscreen(): void;
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
     * @typedef {("resize", event)} addEventListener
     * @param type
     * @param callback
     */
    addEventListener(type: any, callback: any): void;
    removeEventListener(type: any, callback: any): void;
    
}
/**
 * @template D
 * @template E
 */
declare class EventObject<D, E> {
    constructor(type: any, detail: any, target: any, currentTarget: any);
    _propagationCancelled: boolean;
    _preventDefault: boolean;
    type: any;
    /**
     * @type {D}
     */
    detail: D;
    /**
     * @type {E}
     */
    target: E;
    /**
     * @type {E}
     */
    currentTarget: E;
    stopPropagation(): void;
    preventDefault(): void;
    result(): {
        propagationCancelled: boolean;
        preventDefault: boolean;
    };
}
declare class EventRegistry {
    constructor(id: any, addApi: any, removeApi: any, self: any, contextGetter: any);
    eventListeners: any;
    _id: any;
    _remove_api: any;
    _add_api: any;
    bindEvent(type: any, callback: any): void;
    
}
declare class EventBinder {
    constructor(target: any, addApi: any, removeApi: any, self: any, contextGetter: any);
    bindEvent(type: any, callback: any): void;
    addEventListener(type: any, callback: any): any;
    removeEventListener(type: any, callback: any): void;
    
}
declare class SystemTray {
    tray: any;
    set title(title: any);
    set icon(icon: any);
    /**
     *
     * @param menus {TrayMenu[]}
     */
    setMenus(menus: TrayMenu[]): void;
    bindActivate(callback: any): void;
    bindMenuClick(callback: any): void;
    
}
declare class Element {
    static fromHandle(elementHandle: any): any;
    /**
     *
     * @param el {any}
     * @param context {object}
     */
    constructor(el: any, context: object);
    /**
     * @type {ContainerBasedElement}
     */
    _parent: ContainerBasedElement;
    /**
     * @type number
     */
    handle: number;
    createEventBinder(target: any, addEventListenerApi: any, removeEventListenerApi: any): EventBinder;
    /**
     *
     * @returns {number}
     */
    get id(): number;
    /**
     *
     * @returns {Element | null}
     */
    get parent(): Element | null;
    /**
     *
     * @returns {Element}
     */
    get rootElement(): Element;
    focus(): void;
    get window(): Window;
    /**
     *
     * @param style {StyleProps}
     */
    set style(style: StyleProps);
    /**
     *
     * @returns {StyleProps}
     */
    get style(): StyleProps;
    /**
     *
     * @param style {StyleProps}
     */
    set hoverStyle(style: StyleProps);
    get hoverStyle(): StyleProps;
    /**
     *
     * @param value {number}
     */
    set scrollTop(value: number);
    /**
     *
     * @returns {number}
     */
    get scrollTop(): number;
    /**
     *
     * @param value {number}
     */
    set scrollLeft(value: number);
    /**
     *
     * @returns {number}
     */
    get scrollLeft(): number;
    /**
     *
     * @param value {boolean}
     */
    set draggable(value: boolean);
    get draggable(): boolean;
    /**
     *
     * @param value {string}
     */
    set cursor(value: string);
    /**
     *
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
     *
     * @returns {number}
     */
    get scrollHeight(): number;
    /**
     *
     * @returns {number}
     */
    get scrollWidth(): number;
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
    bindKeyDown(callback: any): void;
    bindKeyUp(callback: any): void;
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
declare class Audio {
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
declare class LabelElement extends Element {
    constructor();
    /**
     *
     * @param wrap {boolean}
     */
    set textWrap(wrap: boolean);
    /**
     *
     * @param text {string}
     */
    set text(text: string);
    /**
     *
     * @param align {"left" | "right" | "center"}
     */
    set align(align: "left" | "right" | "center");
    /**
     *
     * @param selection {number[]}
     */
    set selection(selection: number[]);
    /**
     *
     * @param startCaretOffset {number}
     * @param endCaretOffset {number}
     */
    selectByCaretOffset(startCaretOffset: number, endCaretOffset: number): void;
    /**
     *
     * @param line {number}
     * @returns {number}
     */
    getLineBeginOffset(line: number): number;
    /**
     *
     * @param line {number}
     * @param text {string}
     */
    insertLine(line: number, text: string): void;
    /**
     *
     * @param line {number}
     * @param newText {string}
     */
    updateLine(line: number, newText: string): void;
    /**
     *
     * @param line {number}
     */
    deleteLine(line: number): void;
    /**
     *
     * @param row {number}
     * @param col {number}
     * @return {number}
     */
    getCaretOffsetByCursor(row: number, col: number): number;
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
 * }} ParagraphUnit
 */
declare class ParagraphElement extends Element {
    constructor();
    /**
     *
     * @param units {ParagraphUnit[]}
     */
    addLine(units: ParagraphUnit[]): void;
    /**
     *
     * @param index {number}
     * @param units {ParagraphUnit[]}
     */
    insertLine(index: number, units: ParagraphUnit[]): void;
    /**
     *
     * @param index {number}
     */
    deleteLine(index: number): void;
    /**
     *
     * @param index {number}
     * @param units {ParagraphUnit[]}
     */
    updateLine(index: number, units: ParagraphUnit[]): void;
    clear(): void;
    /**
     *
     * @param units {ParagraphUnit[]}
     * @return {[number, number]}
     */
    measureLine(units: ParagraphUnit[]): [number, number];
    /**
     *
     * @returns {string | undefined}
     */
    get selectionText(): string | undefined;
    
}
declare class ImageElement extends Element {
    constructor();
    set src(src: any);
}
declare class EntryElement extends Element {
    constructor();
    /**
     *
     * @param align {"left"|"right"|"center"}
     */
    set align(align: "left" | "right" | "center");
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
     * @param style {StyleProps}
     */
    set placeholderStyle(style: StyleProps);
    /**
     *
     * @returns {StyleProps}
     */
    get placeholderStyle(): StyleProps;
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
     * @param multipleLine {boolean}
     */
    set multipleLine(multipleLine: boolean);
    /**
     *
     * @param value {boolean}
     */
    set autoHeight(value: boolean);
    /**
     *
     * @param rows {number}
     */
    set rows(rows: number);
    bindTextChange(callback: any): void;
    
}
declare class TextEditElement extends Element {
    constructor();
    /**
     *
     * @param align {"left"|"right"|"center"}
     */
    set align(align: "left" | "right" | "center");
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
     * @param selection {[number, number]}
     */
    set selection(selection: [number, number]);
    /**
     *
     * @param caret {number}
     */
    set caret(caret: number);
    /**
     *
     * @param top {number}
     */
    scrollToTop(top: number): void;
    bindTextChange(callback: any): void;
    bindCaretChange(callback: any): void;
}
declare class ButtonElement extends ContainerBasedElement {
    constructor();
}
declare class ContainerElement extends ContainerBasedElement {
    constructor();
}
declare class ScrollElement extends ContainerBasedElement {
    constructor();
    /**
     *
     * @param value {"auto"|"always"|"never"}
     */
    set scrollX(value: "auto" | "always" | "never");
    /**
     *
     * @param value {"auto"|"always"|"never"}
     */
    set scrollY(value: "auto" | "always" | "never");
    scrollBy(value: any): void;
}
declare class WebSocket {
    constructor(url: any);
    client: any;
    listeners: any;
    onopen: any;
    onclose: any;
    onmessage: any;
    onping: any;
    onpong: any;
    onerror: any;
    addEventListener(name: any, callback: any): void;
    send(data: any): Promise<void>;
    close(): void;
    
}
declare class Worker {
    /**
     *
     * @param source {number | string}
     */
    constructor(source: number | string);
    postMessage(data: any): void;
    bindMessage(callback: any): void;
    
}
declare class WorkerContext {
    static create(): WorkerContext;
    postMessage(data: any): void;
    bindMessage(callback: any): void;
    
}
declare class SqliteConn {
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
declare class Sqlite {
    /**
     *
     * @param path {string}
     * @returns {Promise<SqliteConn>}
     */
    static open(path: string): Promise<SqliteConn>;
}
declare const workerContext: WorkerContext;
declare class FetchResponse {
    constructor(resp: any, status: any);
    _resp: any;
    status: any;
    ok: boolean;
    json(): Promise<any>;
}
declare type IResizeEvent = IEvent<ResizeDetail>;
declare type ParagraphUnit = {
    type: "text";
    text: string;
    weight?: string;
    textDecorationLine?: string;
    fontFamilies?: string[];
    fontSize?: number;
    color?: string;
    backgroundColor?: string;
};
/**
 * <T>
 */
declare type IEvent<T> = {
    detail: T;
    target: Element;
    currentTarget: Element;
    stopPropagation(): void;
    preventDefault(): void;
};
declare type IBoundsChangeEvent = IEvent<BoundsChangeDetail>;
declare type IVoidEvent = IEvent<void>;
declare type ICaretEvent = IEvent<CaretDetail>;
declare type IMouseEvent = IEvent<MouseDetail>;
declare type IKeyEvent = IEvent<KeyDetail>;
declare type IMouseWheelEvent = IEvent<MouseWheelDetail>;
declare type ITextEvent = IEvent<TextDetail>;
declare type ITouchEvent = IEvent<TouchDetail>;
declare type IScrollEvent = IEvent<ScrollDetail>;
declare class Clipboard {
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
declare class ContainerBasedElement extends Element {
    /**
     *
     * @param child {Element}
     * @param index {number}
     */
    addChild(child: Element, index?: number): void;
    /**
     *
     * @param newNode {Element}
     * @param referenceNode {Element}
     */
    addChildBefore(newNode: Element, referenceNode: Element): void;
    /**
     *
     * @param newNode {Element}
     * @param referenceNode {Element}
     */
    addChildAfter(newNode: Element, referenceNode: Element): void;
    /**
     *
     * @param child {Element}
     */
    removeChild(child: Element): void;
    
}

