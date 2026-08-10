/* Himalayas Browser — UI shell logic (docs/UI_UX_VISION.md)
   Vue 3 (global build, no bundler). This is a client-side shell: tabs and
   panels are local state for now, since the daemon doesn't yet expose an
   HTTP API for live navigation/session isolation (see src/browser/mod.rs
   for the real Browser::open_tab / IsolationMode backend that a future API
   would call). /device is the one real endpoint, reporting actual device
   tier + UI-enabled state from the Rust daemon. */

const SIDEBAR_SECTIONS = [
  { id: "tabs", icon: "🗂️", label: "Tabs" },
  { id: "history", icon: "🕘", label: "History" },
  { id: "collections", icon: "📚", label: "Collections" },
  { id: "downloads", icon: "⬇️", label: "Downloads" },
  { id: "passwords", icon: "🔑", label: "Passwords" },
  { id: "ai-notes", icon: "📝", label: "AI Notes" },
  { id: "research", icon: "🔬", label: "Research" },
  { id: "pinned", icon: "📌", label: "Pinned Websites" },
  { id: "extensions", icon: "🧩", label: "Extensions" },
  { id: "workspaces", icon: "🗃️", label: "Workspaces" },
  { id: "devices", icon: "💻", label: "Devices" },
  { id: "clipboard", icon: "📋", label: "Clipboard History" },
  { id: "recent-files", icon: "📄", label: "Recent Files" },
];

// History capture is off by default (privacy-first) — nothing to list until
// the user opts in, so this section never shows fabricated entries.
const SECTION_ITEMS = {
  history: [],
  collections: ["Cloud Native (auto-collected)", "Travel research"],
  downloads: [],
  passwords: [],
  "ai-notes": ["Kafka notes from research session"],
  research: ["Kubernetes vs Nomad", "Himalayas UI vision"],
  pinned: ["github.com", "news.ycombinator.com"],
  extensions: [],
  workspaces: ["Work", "Personal", "Research"],
  devices: ["This device — local only, no account required"],
  clipboard: [],
  "recent-files": [],
};

const CONTEXTS = [
  { id: "browsing", icon: "🌐", label: "Browsing", actions: ["Inspect", "Screenshot"] },
  { id: "reading", icon: "📖", label: "Reading", actions: ["Summarize", "Explain word", "Export Markdown"] },
  { id: "shopping", icon: "🛍️", label: "Shopping", actions: ["Compare Prices", "Coupons", "Reviews", "Price History"] },
  { id: "research", icon: "🔬", label: "Research", actions: ["Summarize", "Highlight", "Citation", "Mind Map"] },
  { id: "developer", icon: "🛠️", label: "Developer", actions: ["Inspect", "Network", "Performance", "Lighthouse"] },
  { id: "banking", icon: "🏦", label: "Banking", actions: ["Security status", "Certificate"] },
];

const AI_ACTIONS = ["Summarize page", "Explain selection", "Compare tabs", "Write email", "Generate diagram", "Write code"];

const COMMANDS = [
  { name: "Open Workspace", shortcut: "" },
  { name: "New Tab", shortcut: "Ctrl+T" },
  { name: "Close Tab", shortcut: "Ctrl+W" },
  { name: "Mute Tabs", shortcut: "" },
  { name: "Take Screenshot", shortcut: "" },
  { name: "Translate Page", shortcut: "" },
  { name: "Generate Summary", shortcut: "Ctrl+Shift+S" },
  { name: "Split Screen", shortcut: "" },
  { name: "Clear Cookies", shortcut: "" },
  { name: "Open Dev Tools", shortcut: "F12" },
  { name: "Pin Tab", shortcut: "Ctrl+Shift+K" },
  { name: "Restart Browser", shortcut: "" },
];

function classifyIntent(text) {
  const t = text.trim();
  if (!t) return { kind: "search", label: "Search" };

  const looksLikeUrl = /^https?:\/\//i.test(t) || (/^[a-z0-9-]+(\.[a-z0-9-]+)+([/:?#].*)?$/i.test(t) && !t.includes(" "));
  if (looksLikeUrl) return { kind: "navigate", label: "Navigate" };

  const commandWords = ["clear cache", "clear cookies", "settings", "open devtools", "screenshot", "restart browser"];
  if (commandWords.some((c) => t.toLowerCase().startsWith(c))) return { kind: "command", label: "Command" };

  const aiVerbs = ["explain", "summarize", "translate", "compare", "find", "write", "generate", "convert", "search github", "define"];
  if (aiVerbs.some((v) => t.toLowerCase().startsWith(v))) return { kind: "ai", label: "AI Task" };

  return { kind: "search", label: "Search" };
}

let notificationSeq = 0;

const app = Vue.createApp({
  data() {
    return {
      sidebarOpen: true,
      sidebarSections: SIDEBAR_SECTIONS,
      activeSection: "tabs",
      sidebarQuery: "",

      tabs: [
        { id: "t1", title: "New Tab", url: "himalayas://newtab", memoryMb: 12, sleeping: false, isolation: "isolated" },
      ],
      activeTabId: "t1",

      addressInput: "",
      contexts: CONTEXTS,
      activeContext: "browsing",

      aiOpen: false,
      aiDock: "dock-right",
      aiActions: AI_ACTIONS,
      aiLog: [],

      paletteOpen: false,
      paletteQuery: "",
      paletteIndex: 0,

      notifications: [],

      device: { tier: null, isolation: "…", ui_enabled: true },

      dashboard: {
        continueReading: [{ title: "Kubernetes docs", progress: "62%" }, { title: "Rust async book", progress: "18%" }],
        pinned: ["github.com", "news.ycombinator.com", "gmail.com"],
        aiChats: ["Summarized Kafka architecture", "Explained CSS grid"],
        notes: ["Follow up on Helm chart review"],
        downloads: [{ name: "report.pdf", size: "1.2MB" }],
      },
    };
  },

  computed: {
    activeSectionLabel() {
      const s = this.sidebarSections.find((s) => s.id === this.activeSection);
      return s ? s.label : "";
    },
    intent() {
      return classifyIntent(this.addressInput);
    },
    filteredTabs() {
      const q = this.sidebarQuery.trim().toLowerCase();
      if (!q) return this.tabs;
      return this.tabs.filter((t) => t.title.toLowerCase().includes(q) || t.url.toLowerCase().includes(q));
    },
    filteredSectionItems() {
      const items = SECTION_ITEMS[this.activeSection] || [];
      const q = this.sidebarQuery.trim().toLowerCase();
      const filtered = q ? items.filter((i) => i.toLowerCase().includes(q)) : items;
      if (this.activeSection === "history" && filtered.length === 0) {
        return ["History capture is off by default — enable it in Settings to start recording visits."];
      }
      if (filtered.length === 0) return ["Nothing here yet."];
      return filtered;
    },
    activeTab() {
      return this.tabs.find((t) => t.id === this.activeTabId) || null;
    },
    currentToolbarActions() {
      const ctx = this.contexts.find((c) => c.id === this.activeContext);
      return ctx ? ctx.actions : [];
    },
    filteredCommands() {
      const q = this.paletteQuery.trim().toLowerCase();
      if (!q) return COMMANDS;
      return COMMANDS.filter((c) => c.name.toLowerCase().includes(q));
    },
  },

  methods: {
    selectSection(id) {
      this.activeSection = id;
      this.sidebarOpen = true;
    },

    newTab() {
      const id = "t" + Math.random().toString(36).slice(2, 8);
      // Reflects the real backend default: isolated sessions on capable
      // hardware, shared on constrained tiers (src/browser/mod.rs IsolationMode).
      const isolation = this.device.isolation === "shared" ? "shared" : "isolated";
      this.tabs.push({ id, title: "New Tab", url: "himalayas://newtab", memoryMb: 8, sleeping: false, isolation });
      this.activeTabId = id;
      this.addressInput = "";
    },

    closeTab(id) {
      const idx = this.tabs.findIndex((t) => t.id === id);
      if (idx === -1) return;
      this.tabs.splice(idx, 1);
      if (this.activeTabId === id) {
        this.activeTabId = this.tabs.length ? this.tabs[0].id : null;
      }
      this.notify("Tab closed");
      if (!this.tabs.length) this.newTab();
    },

    activateTab(id) {
      this.activeTabId = id;
    },

    submitAddress() {
      const text = this.addressInput.trim();
      if (!text) return;
      const intent = classifyIntent(text);

      if (intent.kind === "ai") {
        this.aiOpen = true;
        this.runAiAction(text, true);
        this.addressInput = "";
        return;
      }

      if (this.activeTab) {
        this.activeTab.title = text;
        this.activeTab.url = intent.kind === "navigate" ? (/^https?:\/\//i.test(text) ? text : "https://" + text) : "Search: " + text;
      }
      this.notify(intent.kind === "navigate" ? "Navigating…" : "Searching…");
      this.addressInput = "";
    },

    toggleAi() {
      this.aiOpen = !this.aiOpen;
    },

    runAiAction(action, fromAddressBar) {
      this.aiOpen = true;
      const prefix = fromAddressBar ? "" : action + " — ";
      this.aiLog.unshift((fromAddressBar ? action : prefix + (this.activeTab ? this.activeTab.title : "page")) + " (demo response)");
      this.notify("AI: " + action);
    },

    openPalette() {
      this.paletteOpen = true;
      this.paletteQuery = "";
      this.paletteIndex = 0;
      this.$nextTick(() => this.$refs.paletteInput && this.$refs.paletteInput.focus());
    },

    paletteMove(delta) {
      const len = this.filteredCommands.length;
      if (!len) return;
      this.paletteIndex = (this.paletteIndex + delta + len) % len;
    },

    paletteRun() {
      const cmd = this.filteredCommands[this.paletteIndex];
      this.paletteOpen = false;
      if (!cmd) return;
      this.notify("Executed: " + cmd.name);
    },

    notify(text) {
      const id = ++notificationSeq;
      this.notifications.push({ id, text });
      setTimeout(() => {
        this.notifications = this.notifications.filter((n) => n.id !== id);
      }, 4000);
    },

    async loadDevice() {
      try {
        const res = await fetch("/device");
        const data = await res.json();
        this.device.tier = data.device_tier;
        this.device.ui_enabled = data.ui_enabled;
        this.device.isolation = ["LowMemory", "Constrained"].includes(data.device_tier) ? "shared" : "isolated";
      } catch (e) {
        this.device.tier = "unknown";
      }
    },

    onKeydown(e) {
      const mod = e.ctrlKey || e.metaKey;
      if (mod && e.shiftKey && e.key.toLowerCase() === "p") {
        e.preventDefault();
        this.openPalette();
      } else if (mod && e.key === " ") {
        e.preventDefault();
        this.toggleAi();
      } else if (e.key === "Escape") {
        this.paletteOpen = false;
      }
    },
  },

  mounted() {
    this.loadDevice();
    window.addEventListener("keydown", this.onKeydown);
  },

  beforeUnmount() {
    window.removeEventListener("keydown", this.onKeydown);
  },
});

app.mount("#app");
