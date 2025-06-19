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

const loadingTip = document.createElement("div");
loadingTip.textContent = "Loading wasm...";
document.body.appendChild(loadingTip);

// This loads and initialize our WASM module
loadDeftApp().then((app) => {
    document.body.removeChild(loadingTip);
    const canvas = document.createElement("canvas");
    canvas.style.cssText = "width: 100vw; height: 100vh; display: block;";
    document.body.appendChild(canvas);
    // Create the WebGL context
    let context;
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
}).catch((e) => {
    console.error(e);
    loadingTip.style.color = "red";
    loadingTip.textContent = "Failed to load wasm: " + e.message;
});
