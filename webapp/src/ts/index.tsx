// Import our custom CSS
import '../scss/styles.scss';

// Import all of Bootstrap's JS
import * as _ from "bootstrap";

import React from "react"
import {createRoot} from "react-dom/client";
import App from "./components/App";

function run() {
    const root = createRoot(document.getElementById("root")!);
    root.render(<App/>);
}

run();