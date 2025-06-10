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

class HelloElement extends Element {
    constructor() {
        super("hello");
    }
}

function main() {
    navigator.stylesheet.append(stylesheet);
    // saveStartTime();
    const window = new Window({
        width: 400,
        height: 360,
    });
    window.title = "Deft Custom Element";

    const helloElement = new HelloElement();
    window.body.addChild(helloElement);
}

try {
    main();
} catch (error) {
    console.error(error, error.stack);
    process.exit(1)
}

