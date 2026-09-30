import { LANGS } from "./i18n.js";

const UPPER = ["ones", "twos", "threes", "fours", "fives", "sixes"];
const RULES = {
  "yatzy-scandinavian": { bonus: 50, extra: false },
  american: { bonus: 35, extra: true },
};
const $ = (id) => document.getElementById(id);
const worker = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });

let lang = localStorageGet("lang") || ((navigator.language || "en").startsWith("sv") ? "sv" : "en");
let variant = localStorageGet("variant") || "yatzy-scandinavian";
let categories = [];
let card = {};
let extraBonus = 0;
let dice = roll(5);
let rollsLeft = 2;
let status = "loading";
let lastOptions = null;
let lastPoints = {};
let seq = 0;

function localStorageGet(k) {
  try {
    return localStorage.getItem(k);
  } catch {
    return null;
  }
}
function localStorageSet(k, v) {
  try {
    localStorage.setItem(k, v);
  } catch {
    /* private mode */
  }
}

function roll(n) {
  const out = [];
  const buf = new Uint8Array(1);
  while (out.length < n) {
    crypto.getRandomValues(buf);
    if (buf[0] < 252) out.push((buf[0] % 6) + 1);
  }
  return out;
}

const t = () => LANGS[lang];
const catName = (c) => t().cats[c] || c;
const filledIds = () => categories.filter((c) => card[c] !== null && card[c] !== undefined);
const upperTotal = () => UPPER.reduce((s, c) => s + (card[c] ?? 0), 0);
const armed = () => RULES[variant].extra && card.five_of_a_kind === 50;
const scoreSoFar = () =>
  categories.reduce((s, c) => s + (card[c] ?? 0), 0) + (upperTotal() >= 63 ? RULES[variant].bonus : 0) + (RULES[variant].extra ? extraBonus : 0);
const isOver = () => categories.length > 0 && filledIds().length === categories.length;

function situation() {
  const filled = filledIds();
  let s = `dice ${[...dice].sort((a, b) => a - b).join(" ")} | rolls ${rollsLeft} | upper ${upperTotal()} | filled ${filled.length ? filled.join(",") : "-"}`;
  if (RULES[variant].extra && filled.includes("five_of_a_kind")) s += ` | five_of_a_kind ${armed() ? 50 : 0}`;
  return s;
}

function ask(msg) {
  const id = ++seq;
  worker.postMessage({ ...msg, id, variant });
  return id;
}

let queryId = 0;
let pointsId = 0;
worker.onmessage = (e) => {
  const m = e.data;
  if (m.type === "solving") {
    status = "solving";
  } else if (m.type === "ready") {
    status = m.source;
    categories = m.categories;
    for (const c of categories) if (!(c in card)) card[c] = null;
    query();
  } else if (m.type === "options" && m.id === queryId) {
    lastOptions = m.options;
  } else if (m.type === "points" && m.id === pointsId) {
    lastPoints = m.points;
  } else if (m.type === "error") {
    if (m.id === queryId) lastOptions = { error: m.message };
    else if (m.id !== pointsId) status = { error: m.message };
  }
  render();
};

function query() {
  if (!categories.length) return;
  lastOptions = null;
  if (isOver()) return render();
  queryId = ask({ type: "query", situation: situation() });
  pointsId = ask({ type: "points", situation: situation() });
  render();
}

function start(v) {
  variant = v;
  localStorageSet("variant", v);
  categories = [];
  card = {};
  extraBonus = 0;
  dice = roll(5);
  rollsLeft = 2;
  status = "loading";
  lastOptions = null;
  ask({ type: "init" });
  render();
}

function apply(action) {
  if (action.startsWith("keep")) {
    const kept = action === "keep -" ? [] : action.slice(5).split(" ").map(Number);
    dice = [...kept, ...roll(5 - kept.length)];
    rollsLeft -= 1;
  } else {
    const c = action.slice(6);
    const total = lastPoints[c];
    const before = upperTotal();
    const extra = RULES[variant].extra && armed() && new Set(dice).size === 1 ? 100 : 0;
    let box = total - extra;
    if (UPPER.includes(c) && before < 63 && before + box >= 63) box -= RULES[variant].bonus;
    card[c] = box;
    extraBonus += extra;
    dice = roll(5);
    rollsLeft = 2;
  }
  query();
}

function render() {
  const L = t();
  document.documentElement.lang = lang;
  document.title = L.title;
  $("title").textContent = L.title;
  $("intro").textContent = L.intro;
  $("lang").textContent = L.lang;
  $("variant-label").textContent = L.variant;
  const sel = $("variant");
  sel.innerHTML = Object.entries(L.variants)
    .map(([id, name]) => `<option value="${id}"${id === variant ? " selected" : ""}>${name}</option>`)
    .join("");
  $("status").textContent =
    status === "loading" ? L.loading : status === "solving" ? L.solving : status.error ? `${L.failed} ${status.error}` : L.ready(status);

  // Dice.
  $("dice-title").textContent = L.dice;
  $("dice-help").textContent = L.diceHelp;
  $("roll").textContent = L.roll;
  $("dice").innerHTML = dice
    .map((d, i) => `<button class="die" data-i="${i}" aria-label="${L.dice} ${i + 1}: ${d}">${"⚀⚁⚂⚃⚄⚅"[d - 1]}<span>${d}</span></button>`)
    .join("");
  $("rolls-label").textContent = L.rollsLeft;
  $("rolls").innerHTML = [2, 1, 0]
    .map((r) => `<button class="seg${r === rollsLeft ? " on" : ""}" data-r="${r}" aria-pressed="${r === rollsLeft}">${r}</button>`)
    .join("");

  // Score card.
  $("card-title").textContent = L.card;
  $("card-help").textContent = L.cardHelp;
  const row = (c) =>
    `<tr><th scope="row">${catName(c)}</th><td><input inputmode="numeric" data-c="${c}" value="${card[c] ?? ""}" aria-label="${catName(c)}"></td><td class="hint">${
      card[c] == null && lastPoints[c] !== undefined ? `+${lastPoints[c]}` : ""
    }</td></tr>`;
  const upper = categories.filter((c) => UPPER.includes(c));
  const lower = categories.filter((c) => !UPPER.includes(c));
  $("card").innerHTML =
    upper.map(row).join("") +
    `<tr class="sum"><th scope="row">${L.upperTotal}</th><td>${upperTotal()}</td><td></td></tr>` +
    `<tr class="sum"><th scope="row">${L.bonus}</th><td>${upperTotal() >= 63 ? RULES[variant].bonus : 0}</td><td></td></tr>` +
    lower.map(row).join("") +
    (RULES[variant].extra
      ? `<tr><th scope="row">${L.extraBonus}</th><td><input inputmode="numeric" id="extra" value="${extraBonus}" aria-label="${L.extraBonus}"></td><td></td></tr>`
      : "") +
    `<tr class="sum total"><th scope="row">${L.scoreSoFar}</th><td>${scoreSoFar()}</td><td></td></tr>`;

  // Options.
  $("options-title").textContent = L.options;
  $("options-help").textContent = L.optionsHelp;
  $("new-game").textContent = L.newGame;
  const out = $("options");
  if (isOver()) {
    out.innerHTML = `<p class="over">${L.gameOver(scoreSoFar())}</p>`;
  } else if (!lastOptions) {
    out.innerHTML = `<p class="muted">${status === "solving" ? L.solving : L.loading}</p>`;
  } else if (lastOptions.error) {
    out.innerHTML = `<p class="error">${L.invalid} ${lastOptions.error}</p>`;
  } else {
    const sorted = [...lastOptions].sort((a, b) => b.value - a.value);
    const label = (a) =>
      a.startsWith("keep") ? L.keep(a === "keep -" ? [] : a.slice(5).split(" ")) : L.score(catName(a.slice(6)), lastPoints[a.slice(6)] ?? "");
    out.innerHTML =
      `<table class="opts"><thead><tr><th>${L.action}</th><th class="num">${L.expected}</th><th class="num">${L.loss}</th></tr></thead><tbody>` +
      sorted
        .map(
          (o) =>
            `<tr class="${o.best ? "best" : ""}" data-a="${o.action}" tabindex="0"><td>${label(o.action)}${o.best ? ` <span class="tag">${L.best}</span>` : ""}</td><td class="num">${(
              scoreSoFar() + o.value
            ).toFixed(2)}</td><td class="num">${o.best ? "0" : "−" + o.loss.toFixed(2)}</td></tr>`,
        )
        .join("") +
      "</tbody></table>";
  }
  $("note").textContent = L.note;
}

document.addEventListener("click", (e) => {
  const die = e.target.closest(".die");
  if (die) {
    const i = Number(die.dataset.i);
    dice[i] = (dice[i] % 6) + 1;
    return query();
  }
  const seg = e.target.closest(".seg");
  if (seg) {
    rollsLeft = Number(seg.dataset.r);
    return query();
  }
  const opt = e.target.closest("tr[data-a]");
  if (opt) return apply(opt.dataset.a);
});
document.addEventListener("keydown", (e) => {
  const opt = e.target.closest && e.target.closest("tr[data-a]");
  if (opt && (e.key === "Enter" || e.key === " ")) {
    e.preventDefault();
    apply(opt.dataset.a);
  }
});
document.addEventListener("change", (e) => {
  if (e.target.id === "variant") return start(e.target.value);
  if (e.target.id === "extra") {
    extraBonus = Math.max(0, parseInt(e.target.value, 10) || 0);
    return query();
  }
  const c = e.target.dataset && e.target.dataset.c;
  if (c) {
    const n = parseInt(e.target.value, 10);
    card[c] = Number.isFinite(n) && n >= 0 ? n : null;
    query();
  }
});
$("roll").addEventListener("click", () => {
  dice = roll(5);
  rollsLeft = 2;
  query();
});
$("new-game").addEventListener("click", () => start(variant));
$("lang").addEventListener("click", () => {
  lang = lang === "sv" ? "en" : "sv";
  localStorageSet("lang", lang);
  render();
});

if ("serviceWorker" in navigator) navigator.serviceWorker.register("./sw.js").catch(() => {});
start(variant);
