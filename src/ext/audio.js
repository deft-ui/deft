import * as audioApi from "native";
import {EventRegistry} from "deft:core";

export class Audio {
    context;
    #eventRegistry;
    id;
    constructor(config) {
        this.id = audioApi.create(config || {})
        this.#eventRegistry = new EventRegistry(this.id, audioApi.addEventListener, audioApi.removeEventListener, this);
    }

    play() {
        audioApi.play(this.id);
    }

    pause() {
        audioApi.pause(this.id);
    }

    stop() {
        audioApi.stop(this.id);
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