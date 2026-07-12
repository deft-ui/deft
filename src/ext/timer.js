import * as timerApi from "native";

globalThis.setTimeout = timerApi.setTimeout;
globalThis.clearTimeout = timerApi.clearTimeout;
globalThis.setInterval = timerApi.setInterval;
globalThis.clearInterval = timerApi.clearInterval;