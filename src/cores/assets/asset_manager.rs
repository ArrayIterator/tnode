use crate::cores::{assets::{asset_dependencies::AssetDependencies, assets::Assets}, helper::hack::Hack};
use std::{collections::HashMap, ops::{Deref, DerefMut}};

#[derive(Debug, Clone, Default)]
pub struct Queue {
    pub(crate) css: HashMap<String, Vec<String>>,
    pub(crate) js_header: HashMap<String, Vec<String>>,
    pub(crate) js_footer: HashMap<String, Vec<String>>,
}
impl Queue {
    fn reset(&mut self) {
        self.css = HashMap::new();
        self.js_header = HashMap::new();
        self.js_footer = HashMap::new();
    }
}

// #[derive(Debug, Clone, Default)]
// pub struct ExtendedItem {
//     pub(crate) css: HashMap<String, Vec<String>>,
//     pub(crate) js: HashMap<String, Vec<String>>
// }

// #[derive(Debug, Clone, Default)]
// pub struct Extended {
//     pub(crate) items: HashMap<String, ExtendedItem>,
// }

// impl Extended {
//     fn reset(&mut self) {
//         self.items = HashMap::new();
//     }
// }

#[derive(Debug, Clone, Default)]
pub struct Rendered {
    pub(crate) css: HashMap<String, bool>,
    pub(crate) js: HashMap<String, bool>,
}

impl Rendered {
    pub fn is_css_rendered(&self, id: &str) -> bool {
        self.css.contains_key(id)
    }

    pub fn is_js_rendered(&self, id: &str) -> bool {
        self.js.contains_key(id)
    }

    fn reset(&mut self) {
        self.css = HashMap::new();
        self.js = HashMap::new();
    }
}

#[derive(Debug, Clone)]
pub struct AssetManager {
    pub(crate) assets: Assets,
    pub(crate) header: Option<String>,
    pub(crate) footer: Option<String>,
    pub(crate) last_style: Vec<String>,
    pub(crate) last_script: Vec<String>,
    pub(crate) queue: Queue,
    // pub(crate) extended: Extended,
    pub(crate) rendered: Rendered,
}

impl DerefMut for AssetManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.assets_mut()
    }
}

impl Deref for AssetManager {
    type Target = Assets;
    fn deref(&self) -> &Self::Target {
        self.assets()
    }
}

impl Queue {
    fn queue_css(&mut self, id: &str, inherits: Vec<String>) {
        self.css.insert(id.to_string(), Hack::unique_set_string(inherits));
    }
    fn queue_js_header(&mut self, id: &str, inherits: Vec<String>) {
        self.js_header.insert(id.to_string(), Hack::unique_set_string(inherits));
    }
    fn queue_js_footer(&mut self, id: &str, inherits: Vec<String>) {
        self.js_footer.insert(id.to_string(), Hack::unique_set_string(inherits));
    }
}

impl AssetManager {
    pub(crate) fn create_manager(assets: &Assets) -> Self {
        Self {
            assets: assets.clone_self(),
            header: None,
            footer: None,
            last_style: Vec::new(),
            last_script: Vec::new(),
            queue: Queue::default(),
            // extended: Extended::default(),
            rendered: Rendered::default()
        }
    }

    pub fn assets(&self) -> &Assets {
        &self.assets
    }

    pub fn assets_mut(&mut self) -> &mut Assets {
        &mut self.assets
    }

    pub fn reset(&mut self) {
        self.header = None;
        self.footer = None;
        self.last_style = Vec::new();
        self.last_script = Vec::new();
        self.queue.reset();
        // self.extended.reset();
        self.rendered.reset();
    }

    pub fn queue_css(&mut self, id: &str, inherits: Vec<String>) {
        self.queue.queue_css(id, inherits);
    }
    pub fn dequeue_css(&mut self, id: &str) -> Option<Vec<String>> {
        self.queue.css.remove(id)
    }
    pub fn queue_js(&mut self, id: &str, inherits: Vec<String>, in_footer: bool) {
        if in_footer {
            self.queue.queue_js_footer(id, inherits);
            self.queue.js_header.remove(id); // remove
            if let Some(item) = &self.assets.get_package(id) {
                self.queue.js_footer.entry(item.id().to_string()).or_insert(item.js().clone());
            }
        } else {
            self.queue.queue_js_header(id, inherits);
            self.queue.js_footer.remove(id); // remove
            if let Some(item) = &self.assets.get_package(id) {
                self.queue.css.entry(item.id().to_string()).or_insert(item.css().clone());
            }
        }
        // if in_footer {
        //     self.queue.queue_js_footer(id, inherits);
        //     self.queue.js_header.remove(id); // remove
        //     if let Some(item) = self.extended.items.get(id) {
        //         for (key, x) in &item.js {
        //             self.queue.js_footer.entry(key.to_string()).or_insert(Hack::unique_set_string(x.clone()));
        //         }
        //     }
        // } else {
        //     self.queue.queue_js_header(id, inherits);
        //     self.queue.js_footer.remove(id); // remove
        //     if let Some(item) = self.extended.items.get(id) {
        //         for (key, x) in &item.css {
        //             self.queue.css.entry(key.to_string()).or_insert(Hack::unique_set_string(x.clone()));
        //         }
        //     }
        // }
    }

    pub fn dequeue_js(&mut self, id: &str) -> Option<Vec<String>> {
        let mut vec = Vec::new();
        if let Some(head) = self.queue.js_header.remove(id) {
            vec.extend(head);
        }
        if let Some(foot)  = self.queue.js_footer.remove(id) {
            vec.extend(foot);
        }
        if vec.is_empty() {None} else {Some(vec)}
    }
    pub fn render_header(&mut self) -> String {
        if let Some(header) = &self.header {
            return header.clone()
        }
        let mut header = Vec::new();
        self.header = Some("".to_string()); // temp
        let mut queue_css = self.queue.css.clone();
        let mut queue_js_header = self.queue.js_header.clone();
        for (name, items) in self.assets.get_packages() {
            if !queue_js_header.contains_key(name) && !self.queue.js_footer.contains_key(name) {
                continue;
            }
            // sorting
            let mut js_first: HashMap<String, Vec<String>> = HashMap::new();
            let mut js_later: HashMap<String, Vec<String>> = HashMap::new();
            for (x, js) in &queue_js_header {
                if js_first.contains_key(x) {
                    js_later.insert(x.to_string(), js.clone());
                    continue;
                }
                js_first.insert(x.to_string(), js.clone());
            }
            let id = items.id();
            let inherits_js = items.js();
            let inherits_css = items.css();
            let mut ve_js = self.queue.js_header
                .get(id)
                .map(|e|e.clone())
                .unwrap_or(self.queue.js_footer.get(id).map(|e|e.clone()).unwrap_or(Vec::new()));
            if !ve_js.is_empty() {
                ve_js.extend(inherits_js.clone());
                js_later.insert(id.to_string(), Hack::unique_set_string(ve_js));
            } else {
                js_first.insert(id.to_string(), inherits_js.clone());
            }
            for (i, w) in &js_first {
                queue_js_header.entry(i.clone()).or_insert(w.clone());
            }
            for (i, w) in &js_later {
                queue_js_header.entry(i.clone()).or_insert(w.clone());
            }
            let mut ve_css = js_first
                    .get(id)
                    .map(|e|e.clone())
                    .unwrap_or(js_later.get(id).map(|e|e.clone()).unwrap_or(Vec::new()));
            if !ve_css.is_empty() {
                ve_css.extend(inherits_css.clone());
            } else {
                ve_css = inherits_css.clone();
            }
            if !ve_css.is_empty() {
                queue_css.insert(id.to_string(), ve_css);
            }
        }

        // for (name, items) in &self.extended.items {
        //     if !queue_js_header.contains_key(name) && !self.queue.js_footer.contains_key(name) {
        //         continue;
        //     }
        //     // sorting
        //     let mut js_first: HashMap<String, Vec<String>> = HashMap::new();
        //     let mut js_later: HashMap<String, Vec<String>> = HashMap::new();
        //     for (x, js) in &queue_js_header {
        //         if js_first.contains_key(x) {
        //             js_later.insert(x.to_string(), js.clone());
        //             continue;
        //         }
        //         js_first.insert(x.to_string(), js.clone());
        //     }
        //     for (x, inherits) in &items.js {
        //         let mut ve = self.queue.js_header
        //             .get(x)
        //             .map(|e|e.clone())
        //             .unwrap_or(self.queue.js_footer.get(x).map(|e|e.clone()).unwrap_or(Vec::new()));
        //         if !ve.is_empty() {
        //             ve.extend(inherits.clone());
        //             js_later.insert(x.to_string(), Hack::unique_set_string(ve));
        //             continue;
        //         }
        //         js_first.insert(x.to_string(), inherits.clone());
        //     }
        //     for (i, w) in &js_first {
        //         queue_js_header.entry(i.clone()).or_insert(w.clone());
        //     }
        //     for (i, w) in &js_later {
        //         queue_js_header.entry(i.clone()).or_insert(w.clone());
        //     }
        //     for (x, inherits) in &items.css {
        //         let mut inherits = inherits.clone();
        //         let mut ve = js_first
        //             .get(x)
        //             .map(|e|e.clone())
        //             .unwrap_or(js_later.get(x).map(|e|e.clone()).unwrap_or(Vec::new()));
        //         if !ve.is_empty() {
        //             ve.extend(inherits.clone());
        //             inherits = ve;
        //         }
        //         queue_css.insert(x.to_string(), inherits);
        //     }
        // }
        for (id, m) in queue_css {
            self.queue.css.remove(&id);
            if self.rendered.is_css_rendered(&id) {
                continue;
            }
            if let Some(dep) = self.css.get(&id) {
                self.rendered.css.insert(id.clone(), true);
                let p = dep.render();
                if !p.is_empty() {
                    header.push(p);
                }
            }
        }
        for (id, m) in queue_js_header {
            self.queue.js_header.remove(&id);
            if self.rendered.is_js_rendered(&id) {
                continue;
            }
            if let Some(dep) = self.js.get(&id) {
                self.rendered.js.insert(id.clone(), true);
                self.queue.js_footer.remove(&id); // directly remove
                let p = dep.render();
                if !p.is_empty() {
                    header.push(p);
                }
            }
        }
        let header = header.join("");
        self.header = Some(header.clone());
        header
    }
    pub fn render_footer(&mut self) -> String {
        if let Some(footer) = &self.footer {
            return footer.clone()
        }
        let mut footer = Vec::new();
        self.footer = Some("".to_string()); //temp
        let mut queue_js = self.queue.js_footer.clone();
        for (name, items) in self.assets.get_packages() {
            if !queue_js.contains_key(name) {
                continue;
            }
            // sorting
            let mut js_first: HashMap<String, Vec<String>> = HashMap::new();
            let mut js_later: HashMap<String, Vec<String>> = HashMap::new();
            for (x, js) in &queue_js {
                if js_first.contains_key(x) {
                    js_later.insert(x.to_string(), js.clone());
                    continue;
                }
                js_first.insert(x.to_string(), js.clone());
            }

            let id = items.id();
            let inherits_js = items.js();

            let mut ve_js = self.queue.js_footer
                .get(id)
                .map(|e|e.clone())
                .unwrap_or(Vec::new());
            if !ve_js.is_empty() {
                ve_js.extend(inherits_js.clone());
                js_later.insert(id.to_string(), Hack::unique_set_string(ve_js));
            } else {
                js_first.insert(id.to_string(), inherits_js.clone());
            }
            for (i, w) in &js_first {
                queue_js.entry(i.clone()).or_insert(w.clone());
            }
            for (i, w) in &js_later {
                queue_js.entry(i.clone()).or_insert(w.clone());
            }
        }
        // for (name, items) in &self.extended.items {
        //     if !queue_js.contains_key(name) {
        //         continue;
        //     }
        //     // sorting
        //     let mut js_first: HashMap<String, Vec<String>> = HashMap::new();
        //     let mut js_later: HashMap<String, Vec<String>> = HashMap::new();
        //     for (x, js) in &queue_js {
        //         if js_first.contains_key(x) {
        //             js_later.insert(x.to_string(), js.clone());
        //             continue;
        //         }
        //         js_first.insert(x.to_string(), js.clone());
        //     }
        //     for (x, inherits) in &items.js {
        //         let mut ve = queue_js
        //             .get(x)
        //             .map(|e|e.clone())
        //             .unwrap_or(Vec::new());
        //         if !ve.is_empty() {
        //             ve.extend(inherits.clone());
        //             js_later.insert(x.to_string(), Hack::unique_set_string(ve));
        //             continue;
        //         }
        //         js_first.insert(x.to_string(), inherits.clone());
        //     }
        //     for (i, w) in &js_first {
        //         queue_js.entry(i.clone()).or_insert(w.clone());
        //     }
        //     for (i, w) in &js_later {
        //         queue_js.entry(i.clone()).or_insert(w.clone());
        //     }
        // }
        for (id, m) in queue_js {
            self.queue.js_footer.remove(&id);
            if self.rendered.is_js_rendered(&id) {
                continue;
            }
            if let Some(dep) = self.js.get(&id) {
                self.rendered.js.insert(id.clone(), true);
                self.queue.js_footer.remove(&id); // directly remove
                self.queue.js_header.remove(&id); // directly remove
                let p = dep.render();
                if !p.is_empty() {
                    footer.push(dep.render());
                }
            }
        }
        let footer = footer.join("");
        self.footer = Some(footer.clone());
        footer
    }
    pub fn render_last_style(&mut self) -> String {
        let mut style : Vec<String> = Vec::new();
        let css = self.queue.css.clone();
        self.queue.css.clear();
        for (id, x) in &css {
            if self.rendered.is_css_rendered(id) {
                continue;
            }
            if let Some(css) = self.css.get(id) {
                let css = css.render();
                if !css.is_empty() {
                    style.push(css.clone());
                    self.last_style.push(css);
                }
            }
        }
        style.join("\n")
    }
    pub fn render_last_js(&mut self) -> String {
        let mut script : Vec<String> = Vec::new();
        let mut merge = self.queue.js_header.clone();
        self.queue.js_header.clear();
        let js_footer = self.queue.js_footer.clone();
        self.queue.js_footer.clear();
        merge.extend(js_footer);

        for (id, x) in &merge {
            if self.rendered.is_js_rendered(id) {
                continue;
            }
            if let Some(js) = self.js.get(id) {
                let js = js.render();
                if !js.is_empty() {
                    script.push(js.clone());
                    self.last_script.push(js);
                }
            }
        }
        script.join("\n")
    }

    pub fn get_last_style(&self) -> String {
        self.last_style.join("")
    }

    pub fn get_last_script(&self) -> String {
        self.last_script.join("")
    }
}
