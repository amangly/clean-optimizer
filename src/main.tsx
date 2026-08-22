import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { ExternalLinkGuard } from "./components/external-link-guard";
import { ThemeProvider } from "./components/theme-provider";
import "./index.css";

const root = document.getElementById("root");
if (!root) {
  throw new Error("root element missing");
}
createRoot(root).render(
  <StrictMode>
    <ThemeProvider defaultTheme="dark">
      <ExternalLinkGuard />
      <App />
    </ThemeProvider>
  </StrictMode>,
);
