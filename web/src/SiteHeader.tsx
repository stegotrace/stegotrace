import type { Locale } from "./i18n";
import { copy, route } from "./i18n";

export default function SiteHeader({ locale, page }: { locale: Locale; page: "home" | "cli" }) {
  const text = copy[locale].nav;
  const other: Locale = locale === "es" ? "en" : "es";
  return (
    <header className="header">
      <a className="wordmark" href={route(locale)} aria-label="StegoTrace">
        Stego<span>Trace</span>
      </a>
      <nav aria-label={locale === "es" ? "Navegación principal" : "Main navigation"}>
        {page === "home" && <a href="#methods">{text.methods}</a>}
        <a href="https://github.com/stegotrace/stegotrace/blob/main/docs/RESEARCH.md">{text.research}</a>
        <a href="https://github.com/stegotrace/stegotrace">{text.source}</a>
        <a href={route(locale, "cli")}>{text.cli}</a>
        <a className="language-link" href={route(other, page)} hrefLang={other}>{text.language}</a>
      </nav>
    </header>
  );
}
