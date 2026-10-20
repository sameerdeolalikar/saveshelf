"use strict";
const $ = (id) => document.getElementById(id);
let selected = null,
  verified = false,
  working = false,
  state = { sources: [], snapshots: [] };
function fail(error) {
  $("error").textContent = error.message;
  $("error").hidden = false;
}
function status(message) {
  $("status").textContent = message;
  $("status").hidden = !message;
}
function controls() {
  document
    .querySelectorAll("button,input,select")
    .forEach((el) => (el.disabled = working));
  $("capture").disabled = working || !$("source").value;
  $("verify").disabled = working || !selected;
  $("restore").disabled = working || !verified || !$("destination").value;
}
async function api(route, body = {}) {
  const response = await fetch("/api/" + route, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Requested-With": "SaveShelf",
    },
    body: JSON.stringify(body),
  });
  const data = await response.json();
  if (!response.ok) throw new Error(data.error || "Operation failed.");
  return data;
}
async function task(message, fn) {
  if (working) return;
  working = true;
  $("error").hidden = true;
  $("busy").textContent = message;
  $("busy").hidden = false;
  controls();
  try {
    await fn();
  } catch (error) {
    fail(error);
  } finally {
    working = false;
    $("busy").hidden = true;
    controls();
  }
}
function resetSelection() {
  verified = false;
  $("details").hidden = true;
  $("complete").hidden = true;
  $("verified-count").textContent = "—";
  $("verification").textContent = selected
    ? "Not verified"
    : "Nothing selected";
  $("verification").className = "tag";
  $("selection").textContent =
    selected || "Select a checkpoint above to inspect its contents.";
  $("step1").className = "step" + (selected ? "" : " active");
  $("step2").className = "step" + (selected ? " active" : "");
  $("step3").className = "step";
}
function render() {
  $("source-count").textContent = state.sources.length;
  $("snapshot-count").textContent = state.snapshots.length;
  $("workspace").textContent = state.workspace;
  const old = $("source").value;
  $("source").replaceChildren();
  if (!state.sources.length)
    $("source").add(new Option("No source folders yet", ""));
  state.sources.forEach((name) => $("source").add(new Option(name, name)));
  if (state.sources.includes(old)) $("source").value = old;
  $("snapshots").replaceChildren();
  if (!state.snapshots.length) {
    const empty = document.createElement("div");
    empty.className = "empty";
    const title = document.createElement("strong");
    title.textContent = "No snapshots yet.";
    empty.append(
      title,
      "Create a fictional demo save, then capture your first checkpoint.",
    );
    $("snapshots").append(empty);
  }
  [...state.snapshots].reverse().forEach((name, index) => {
    const button = document.createElement("button");
    button.className = name === selected ? "selected" : "";
    button.setAttribute("aria-pressed", String(name === selected));
    const title = document.createElement("strong");
    title.textContent =
      index === 0 ? "Latest checkpoint" : `Earlier checkpoint ${index}`;
    const note = document.createElement("small");
    note.className = "snapshot-name";
    note.textContent = name;
    button.append(title, note);
    button.addEventListener("click", () => {
      selected = name;
      resetSelection();
      render();
      controls();
    });
    $("snapshots").append(button);
  });
}
async function refresh() {
  state = await api("status");
  if (selected && !state.snapshots.includes(selected)) {
    selected = null;
    resetSelection();
  }
  render();
}
$("refresh").addEventListener("click", () =>
  task("Reading your local workspace…", async () => {
    verified = false;
    resetSelection();
    await refresh();
    status(
      "Workspace refreshed. Verify a snapshot to inspect its stored bytes.",
    );
  }),
);
$("demo").addEventListener("click", () =>
  task("Creating fictional save data…", async () => {
    const result = await api("demo");
    await refresh();
    $("source").value = "starfall-demo";
    status(result.message + " You can now create a snapshot.");
  }),
);
$("capture").addEventListener("click", () =>
  task(
    "Copying and checksumming files… Do not edit the source folder.",
    async () => {
      const result = await api("snapshot", { source: $("source").value });
      selected = result.snapshot;
      resetSelection();
      await refresh();
      status(
        "Checkpoint captured. Select “Verify selected snapshot” to inspect it.",
      );
    },
  ),
);
$("source").addEventListener("change", controls);
$("destination").addEventListener("input", controls);
$("verify").addEventListener("click", () =>
  task("Verifying all stored files…", async () => {
    verified = false;
    $("details").hidden = true;
    $("complete").hidden = true;
    $("verification").textContent = "Not verified";
    $("verification").className = "tag";
    $("verified-count").textContent = "—";
    const result = await api("verify", { snapshot: selected }),
      manifest = result.manifest;
    verified = true;
    $("files").replaceChildren();
    manifest.files.slice(0, 100).forEach((file) => {
      const row = document.createElement("tr");
      [
        file.path,
        `${file.bytes.toLocaleString()} B`,
        file.sha256.slice(0, 16) + "…",
      ].forEach((value) => {
        const cell = document.createElement("td");
        cell.textContent = value;
        row.append(cell);
      });
      $("files").append(row);
    });
    $("manifest-summary").textContent =
      `${manifest.files.length} files · ${manifest.directories.length} directories · captured ${new Date(manifest.created_unix_seconds * 1000).toLocaleString()}`;
    $("verified-count").textContent = manifest.files.length;
    $("verification").textContent = "Verified";
    $("verification").className = "tag added";
    $("details").hidden = false;
    $("step2").className = "step";
    $("step3").className = "step active";
    $("destination").value = "recovery-" + Date.now();
    status(result.message + " Choose a new folder name to restore a copy.");
  }),
);
$("restore").addEventListener("click", () =>
  task("Verifying and restoring into a new folder…", async () => {
    const destination = $("destination").value;
    if (!/^[A-Za-z0-9_-]{1,100}$/.test(destination))
      throw new Error(
        "Use 1–100 letters, numbers, hyphens, or underscores for the folder name.",
      );
    const result = await api("restore", { snapshot: selected, destination });
    $("complete").hidden = false;
    $("restored-path").textContent = result.path;
    status("Recovery complete. Existing source files were not changed.");
  }),
);
task("Connecting to the local snapshot engine…", refresh);
