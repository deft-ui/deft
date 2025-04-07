
workerContext.bindMessage((data) => {
    console.log("Worker: received message from parent", data);
})

function test() {
    return new Promise((resolve) => setTimeout(resolve, 100))
}

test().then(() => {
    console.log("Worker: initialized!");
    workerContext.postMessage("Hello, i am a worker!");
})