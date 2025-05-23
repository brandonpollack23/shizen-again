import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

// Prepare window.___TAURI___ type for TypeScript
declare global {
  interface Window {
    __TAURI__: {
      invoke: (command: string, args?: Record<string, unknown>) => Promise<any>;
    };
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
