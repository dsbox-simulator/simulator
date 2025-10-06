import {createRoot} from "react-dom/client";
import React from "react";
import App from "./app"


(function () {
    const root = createRoot(document.getElementById('app')!);
    root.render(<App wsPath="/socket" />);
})();