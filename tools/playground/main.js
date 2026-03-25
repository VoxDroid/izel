async function loadPlayground() {
  const output = document.getElementById("output");

  try {
    const wasm = await import("./pkg/izel_playground.js");
    await wasm.default();

    const runButton = document.getElementById("run");
    const source = document.getElementById("source");

    runButton.addEventListener("click", () => {
      const result = wasm.repl_eval(source.value);
      output.textContent = result;
    });

    output.textContent = "WASM playground loaded.";
  } catch (err) {
    output.textContent =
      "WASM module not built yet. Build tools/playground/wasm and generate pkg/ first.\n\n" +
      String(err);
  }
}

loadPlayground();
