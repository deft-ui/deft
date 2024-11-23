function test() {
    return new Promise((resolve) => setTimeout(resolve, 100))
}

test().then(() => {
    console.log("Worker run!");
})