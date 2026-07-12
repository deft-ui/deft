import {Window} from "deft:ui";
import {HelloWidget} from "custom_widget";

const stylesheet = `
body {
    justify-content: center;
    align-items: center;
}
hello {
    width: 100px;
    height: 100px;
    background: #ccc;
}
`

function main() {
    navigator.stylesheet.append(stylesheet);
    // saveStartTime();
    const window = new Window({
        width: 400,
        height: 360,
    });
    window.title = "Deft Custom Widget";

    const helloWidget = new HelloWidget();
    window.body.addChild(helloWidget);
}

try {
    main();
} catch (error) {
    console.error(error, error.stack);
    process.exit(1)
}

