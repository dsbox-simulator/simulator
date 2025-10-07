import {createRoot} from "react-dom/client";
import React from "react";
import App from "./app"

export const inTauri = "__TAURI_INTERNALS__" in window;

(function () {
    const root = createRoot(document.getElementById('app')!);
    root.render(<App wsPath="/socket" inTauri={inTauri} />);
})();