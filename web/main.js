/**
 * Make a canvas element fit to the display window.
 */
function resizeCanvasToDisplaySize(canvas) {
    const scale = window.devicePixelRatio;
    const width = (canvas.clientWidth | 1) * scale;
    const height = (canvas.clientHeight | 1) * scale;
    if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
        return true;
    }
    return false;
}

// This loads and initialize our WASM module
loadDeftApp().then((app) => {
    // Create the WebGL context
    let context;
    const canvas = document.querySelector("#glcanvas");
    context = canvas.getContext("webgl2", {
        antialias: true,
        depth: true,
        stencil: true,
        alpha: true,
    });

    // Register the context with emscripten
    handle = app.GL.registerContext(context, {majorVersion: 2});
    app.GL.makeContextCurrent(handle);

    // Fit the canvas to the viewport
    resizeCanvasToDisplaySize(canvas);

    try {
        app._asm_main();
    } catch (error) {
        // console.error("main error", error);
    }

    // // Make canvas size stick to the window size
    window.addEventListener("resize", () => {
        resizeCanvasToDisplaySize(canvas);
    });
});
