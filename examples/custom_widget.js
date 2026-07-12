import {Widget} from "deft:ui";
import * as customWidgetApi from "native";

export class HelloWidget extends Widget {
    constructor() {
        super(customWidgetApi.create);
    }
}