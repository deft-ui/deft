declare type FrameType = "normal" | "menu"
declare interface FrameAttrs {
    width ?: number
    height ?: number
    title ?: string
    resizable ?: boolean,
    decorations ?: boolean,
    overrideRedirect ?: boolean,
    position ?: [number, number],
    visible ?: boolean,
    frameType ?: FrameType,
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

declare interface EventData<T> {
    detail: T
}

declare interface EventCallback<T> {
    (event: EventData<T>): void;
}

declare interface CloseEventCallback extends EventCallback<void> {}
declare interface FocusEventCallback extends EventCallback<void> {}
declare interface BlurEventCallback extends EventCallback<void> {}

declare interface BoundsChangeCallback extends EventCallback<{
    originBounds: ElementRect
}> {
}

declare interface MouseDetail {
    button: number,
    offsetX: number;
    offsetY: number;
    frameX: number;
    frameY: number;
    screenX: number;
    screenY: number;
}

declare type IMouseEvent = EventData<MouseDetail>;


declare interface LocalStorage {
    getItem(key: string): string | null,

    setItem(key: string, value: string): void,
}
// @ts-ignore
declare const localStorage: LocalStorage;

declare interface TrayMenu {
    id: string,
    label: string,
}

declare function process_exit(code: number);
declare function path_filename(path: string): string;
declare function path_join(path: string, other: string): string;
declare function animation_create(name: string, keyFrames: Record<string, Record<string, any>>)

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

declare function audio_create(path: string);
declare function audio_destroy(id): void;
declare function audio_position(id): number;
declare function audio_duration(id): number;

declare function base64_encode_str(str: string): string;

declare interface ShowFileDialogOptions {
    dialogType ?: "single" | "multiple" | "save" | "dir"
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
