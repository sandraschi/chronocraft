//! Hierarchical `SubApp` sidebar registry for ChronoCraft's 6 workbenches.

use crate::app::{AppLocale, Page};

#[derive(Debug, Clone, Copy)]
pub struct SubPage {
    pub page: Page,
    pub sub: usize,
    pub en: &'static str,
    pub de: &'static str,
}

impl SubPage {
    pub fn label(&self, locale: AppLocale) -> &'static str {
        match locale {
            AppLocale::En => self.en,
            AppLocale::De => self.de,
        }
    }
}

pub trait SubApp: Sync {
    fn id(&self) -> &'static str;
    fn title(&self, locale: AppLocale) -> &'static str;
    fn subpages(&self) -> &'static [SubPage];
}

struct CraftWorkbench {
    id: &'static str,
    en: &'static str,
    de: &'static str,
    subpages: &'static [SubPage],
}

impl SubApp for CraftWorkbench {
    fn id(&self) -> &'static str {
        self.id
    }

    fn title(&self, locale: AppLocale) -> &'static str {
        match locale {
            AppLocale::En => self.en,
            AppLocale::De => self.de,
        }
    }

    fn subpages(&self) -> &'static [SubPage] {
        self.subpages
    }
}

const CANVAS_SUBS: &[SubPage] = &[
    SubPage {
        page: Page::Canvas,
        sub: 0,
        en: "Model Canvas & Links",
        de: "Modell-Leinwand & Links",
    },
    SubPage {
        page: Page::Canvas,
        sub: 1,
        en: "Presets & Model Builder",
        de: "Vorlagen & Modell-Editor",
    },
];

const SLIDERS_SUBS: &[SubPage] = &[
    SubPage {
        page: Page::Sliders,
        sub: 0,
        en: "Parameter Sliders",
        de: "Parameter-Regler",
    },
    SubPage {
        page: Page::Sliders,
        sub: 1,
        en: "Solver Settings",
        de: "Solver-Einstellungen",
    },
];

const FANCHART_SUBS: &[SubPage] = &[
    SubPage {
        page: Page::FanChart,
        sub: 0,
        en: "Fan-Chart Bands",
        de: "Faencher-Baender",
    },
    SubPage {
        page: Page::FanChart,
        sub: 1,
        en: "Deterministic Trace",
        de: "Deterministische Kurve",
    },
];

const RUNS_SUBS: &[SubPage] = &[
    SubPage {
        page: Page::Runs,
        sub: 0,
        en: "Scenarios & Compare",
        de: "Szenarien & Vergleich",
    },
    SubPage {
        page: Page::Runs,
        sub: 1,
        en: "Sensitivity Sweep",
        de: "Sensitivitaets-Sweep",
    },
];

const ASSISTANT_SUBS: &[SubPage] = &[SubPage {
    page: Page::Chat,
    sub: 0,
    en: "Chat",
    de: "Chat",
}];

const HELP_SUBS: &[SubPage] = &[SubPage {
    page: Page::Help,
    sub: 0,
    en: "Help (F1)",
    de: "Hilfe (F1)",
}];

static WORKBENCHES: [CraftWorkbench; 6] = [
    CraftWorkbench {
        id: "canvas",
        en: "1. Canvas Studio",
        de: "1. Canvas-Studio",
        subpages: CANVAS_SUBS,
    },
    CraftWorkbench {
        id: "sliders",
        en: "2. Sliders & Parameters",
        de: "2. Regler & Parameter",
        subpages: SLIDERS_SUBS,
    },
    CraftWorkbench {
        id: "fanchart",
        en: "3. Fan-Chart Viewer",
        de: "3. Faecher-Diagramm",
        subpages: FANCHART_SUBS,
    },
    CraftWorkbench {
        id: "runs",
        en: "4. Runs & Scenarios",
        de: "4. Laeufe & Szenarien",
        subpages: RUNS_SUBS,
    },
    CraftWorkbench {
        id: "assistant",
        en: "5. Assistant Chat",
        de: "5. Assistent-Chat",
        subpages: ASSISTANT_SUBS,
    },
    CraftWorkbench {
        id: "help",
        en: "6. Help (F1)",
        de: "6. Hilfe (F1)",
        subpages: HELP_SUBS,
    },
];

pub fn registry() -> Vec<&'static dyn SubApp> {
    WORKBENCHES.iter().map(|w| w as &dyn SubApp).collect()
}
