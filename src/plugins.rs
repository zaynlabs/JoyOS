//! Plugins are folders in `plugins\` under JoyOS's data folder. Each folder has a
//! `plugin.js`, which runs inside the shell when plugins are on and the plan allows
//! it. A plugin can show a toast with `joy.toast(text)` or open a web page with
//! `joy.open(url)`. Each script runs in its own sandboxed frame, so it can't read
//! or change the JoyOS page; still, only install plugins you trust.

use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plugin {
    pub name: String,
    pub script: String,
}

pub fn folder() -> Option<PathBuf> {
    crate::settings::data_root().map(|d| d.join("plugins"))
}

pub fn load() -> Vec<Plugin> {
    let Some(dir) = folder() else { return vec![] };
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
    let mut plugins: Vec<Plugin> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let script = std::fs::read_to_string(e.path().join("plugin.js")).ok()?;
            Some(Plugin { name, script })
        })
        .collect();
    plugins.sort_by(|a, b| a.name.cmp(&b.name));
    plugins
}

/// What runs inside a plugin's frame: it waits for its code, then runs it with a `joy` helper.
const FRAME: &str = r#"<!doctype html><script>
addEventListener("message", (e) => {
  if (e.source !== parent) return;
  const send = (m) => parent.postMessage(m, "*");
  const joy = {
    toast: (m) => send({ t: "plugin-toast", m: String(m) }),
    open: (u) => send({ t: "plugin-open", u: String(u) }),
  };
  try { new Function("joy", e.data.code)(joy); } catch (err) { send({ t: "log", m: String(err) }); }
}, { once: true });
</script>"#;

/// Runs a plugin in a sandboxed frame. It can't touch the JoyOS page or its
/// messages; all it can do is ask for a toast or a web page, a few times a minute.
pub fn wrap(name: &str, script: &str) -> String {
    let code = serde_json::to_string(script).unwrap_or_else(|_| "\"\"".into());
    let frame = serde_json::to_string(FRAME).unwrap_or_else(|_| "\"\"".into());
    let title = serde_json::to_string(name).unwrap_or_else(|_| "\"\"".into());
    format!(
        r#"(() => {{
  const f = document.createElement("iframe");
  f.sandbox = "allow-scripts";
  f.hidden = true;
  f.title = {title};
  f.srcdoc = {frame};
  let start = Date.now(), count = 0;
  addEventListener("message", (e) => {{
    if (e.source !== f.contentWindow) return;
    const now = Date.now();
    if (now - start > 60000) {{ start = now; count = 0; }}
    if (++count > 10) return;
    const d = e.data || {{}};
    if (d.t === "plugin-toast") dioxus.send({{ t: d.t, m: String(d.m).slice(0, 160) }});
    else if (d.t === "plugin-open") dioxus.send({{ t: d.t, u: String(d.u) }});
    else if (d.t === "log") dioxus.send({{ t: "log", m: String(d.m).slice(0, 500) }});
  }});
  f.onload = () => f.contentWindow.postMessage({{ code: {code} }}, "*");
  document.body.appendChild(f);
}})();"#
    )
}

/// Only web addresses may be opened by a plugin.
pub fn allowed_link(url: &str) -> bool {
    crate::system::launch::web_link(url).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugins_are_wrapped_and_links_checked() {
        let js = wrap("hello", "joy.toast('hi'); // </script>");
        assert!(js.contains(r#""joy.toast('hi'); // </script>""#), "the code is passed as a string, not pasted in");
        assert!(js.contains(r#"f.sandbox = "allow-scripts""#));
        assert!(!js.contains("allow-same-origin"));
        assert!(allowed_link("https://example.com"));
        assert!(!allowed_link("file:///C:/Windows/System32/cmd.exe"));
        assert!(!allowed_link("javascript:alert(1)"));
    }
}
