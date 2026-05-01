// Trunk data-initializer for bike-fitter-1000.
//
// Trunk's pipeline emits the wasm-bindgen JS shim and binary; this file is
// loaded as an ES module and gets to define how the wasm module is started.
// Trunk calls the default export with the shim's init function (and friends)
// once the page is ready.
//
// We:
//   1. Run wasm-bindgen's `init()` to fetch + instantiate the wasm module.
//   2. Call our exported Rust `start()` to hand off to eframe.
//   3. Hide the loading shim. Surface init errors to the user.
export default function initializer() {
  return {
    onStart: () => {},
    onProgress: () => {},
    onComplete: () => {
      const loading = document.getElementById("loading");
      if (loading) loading.classList.add("hidden");
    },
    onSuccess: (wasm) => {
      // wasm here is the resolved module exports object. Our Rust code
      // exposes `start` via #[wasm_bindgen]; calling it kicks off eframe.
      try {
        wasm.start();
      } catch (e) {
        console.error("bike-fitter-1000 start() threw:", e);
        const loading = document.getElementById("loading");
        if (loading) {
          loading.textContent = "App crashed during init. See console.";
          loading.classList.remove("hidden");
        }
      }
    },
    onFailure: (err) => {
      console.error("wasm load failed:", err);
      const loading = document.getElementById("loading");
      if (loading) {
        loading.textContent = "Failed to load wasm module. See console.";
      }
    },
  };
}
