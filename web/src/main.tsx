import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource/ibm-plex-mono/400.css";
import "@fontsource/ibm-plex-mono/500.css";
import App from "./App";
import Guide from "./Guide";
import type { Locale } from "./i18n";
import "./styles.css";

const path = window.location.pathname;
const locale: Locale = path.startsWith("/en/") ? "en" : "es";
const page = /\/(?:es|en)\/cli\/?$/.test(path) ? "cli" : "home";
document.documentElement.lang = locale;

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {page === "cli" ? <Guide locale={locale} /> : <App locale={locale} />}
  </StrictMode>,
);
