var App;
/******/ (() => { // webpackBootstrap
/******/ 	"use strict";
var __webpack_exports__ = {};
// This entry need to be wrapped in an IIFE because it uses a non-standard name for the exports (exports).
(() => {
var exports = __webpack_exports__;
/*!*******************!*\
  !*** ./ts/app.ts ***!
  \*******************/

Object.defineProperty(exports, "__esModule", ({ value: true }));
function run() {
    let app = new App();
    app.run();
}
exports["default"] = run;
function error(message) {
    throw new Error(message);
}
class App {
    constructor() {
        var _a;
        this.events_table = (_a = document.getElementById('events')) !== null && _a !== void 0 ? _a : error("could not find element with id #logs");
        let loc = window.location;
        let ws_protocol;
        if (loc.protocol === "https:") {
            ws_protocol = "wss";
        }
        else {
            ws_protocol = "ws";
        }
        let websocket_uri = `${ws_protocol}://${loc.host}/socket`;
        this.socket = new WebSocket(websocket_uri);
    }
    run() {
        this.socket.addEventListener("message", this.socketMessage.bind(this));
        let resume_button = document.getElementById("resume");
        if (resume_button !== null) {
            resume_button.addEventListener("click", this.resume.bind(this));
        }
        let step_button = document.getElementById("step");
        if (step_button !== null) {
            step_button.addEventListener("click", this.step.bind(this));
        }
    }
    addEvent(event) {
        let row = document.createElement("tr");
        let cell = document.createElement("td");
        cell.innerText = event.timestamp.logical;
        row.appendChild(cell);
        cell = document.createElement("td");
        cell.innerText = event.timestamp.physical;
        row.appendChild(cell);
        cell = document.createElement("td");
        cell.innerText = event.data.type;
        row.appendChild(cell);
        delete event.data.type;
        cell = document.createElement("td");
        cell.innerText = JSON.stringify(event.data);
        row.appendChild(cell);
        this.events_table.appendChild(row);
    }
    socketMessage(message) {
        this.addEvent(JSON.parse(message.data));
    }
    resume() {
        this.socket.send("resume");
    }
    step() {
        this.socket.send("step");
    }
}

})();

App = __webpack_exports__;
/******/ })()
;
//# sourceMappingURL=app.js.map