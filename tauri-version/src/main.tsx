import ReactDOM from "react-dom/client";
import App from "./App";
import { installFontFaces } from "./fonts/catalog";

import "./styles/index.css";

import { initScaling } from "./modules/scaling";
import { initTheme } from "./modules/theme";
import { initI18n, t } from "./i18n";

import { SettingsProvider } from "./modules/settings";
import { ErrorBoundary } from "./components/Common/ErrorBoundary";
import { revealAppWindow } from "./modules/windowReveal";
import { beginBootTrace, markBootStage } from "./modules/perf/bootTrace";

beginBootTrace();
markBootStage("scripts");

installFontFaces();
initI18n();
initTheme();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <ErrorBoundary title={t('error.appTitle')} onCatch={revealAppWindow}>
    <SettingsProvider>
      <App />
    </SettingsProvider>
  </ErrorBoundary>,
);

markBootStage("render");

initScaling();

const pdfExportCheck = import.meta.env.VITE_E2E_PDF;
if (pdfExportCheck) {
  void import("./modules/export/pdfExportCheck").then((check) => check.runPdfExportCheck(pdfExportCheck));
}
