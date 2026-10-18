// Generated from offline.cs/en.json. No network or document storage.
const messages = {
  cs: {
    title: "Připojení není dostupné",
    intro: "Open Faktura potřebuje připojení k vašemu serveru.",
    detail:
      "Doklady ani jejich stav nejsou dostupné offline. Z této obrazovky nelze nic uložit nebo odeslat. Po obnovení připojení ověřte stav poslední operace v aplikaci.",
    retry: "Zkusit znovu",
    language: "Jazyk / Language",
  },
  en: {
    title: "Connection unavailable",
    intro: "Open Faktura needs a connection to your server.",
    detail:
      "Documents and their status are unavailable offline. Nothing can be saved or sent from this screen. When the connection returns, check the status of your last action in the app.",
    retry: "Try again",
    language: "Jazyk / Language",
  },
};
function show(language) {
  const selected = language === "en" ? "en" : "cs";
  document.documentElement.lang = selected;
  document.title = messages[selected].title + " | Open Faktura";
  for (const el of document.querySelectorAll("[data-message]"))
    el.textContent = messages[selected][el.dataset.message];
  for (const el of document.querySelectorAll("[data-language]"))
    el.setAttribute("aria-pressed", String(el.dataset.language === selected));
}
try {
  show(localStorage.getItem("open-faktura.language"));
} catch {
  show("cs");
}
for (const el of document.querySelectorAll("[data-language]"))
  el.addEventListener("click", () => {
    show(el.dataset.language);
    try {
      localStorage.setItem("open-faktura.language", el.dataset.language);
    } catch {}
  });
document.getElementById("retry").addEventListener("click", (event) => {
  event.preventDefault();
  location.reload();
});
