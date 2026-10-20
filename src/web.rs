//! Local browser interface, restricted to a user-selected workspace.
use saveshelf::{list, restore, snapshot, verify, Result};
use serde_json::{json, Value};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};
use tiny_http::{Header, Method, Response, Server};

fn child(root: &Path, name: &str) -> Result<PathBuf> {
    if name.is_empty()
        || name.len() > 100
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("Use a name with 1–100 letters, numbers, hyphens, or underscores.".into());
    }
    let path = root.join(name);
    if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("Symlinks are not allowed in the workspace.".into());
    }
    Ok(path)
}

fn text<'a>(body: &'a Value, key: &str) -> Result<&'a str> {
    body.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Missing {key}.").into())
}

fn operation(root: &Path, route: &str, body: Value) -> Result<Value> {
    let sources = root.join("sources");
    let vault = root.join("vault");
    let restores = root.join("restores");
    // Recheck these boundaries on every operation, including after external edits.
    for directory in [&sources, &vault, &restores] {
        let meta = fs::symlink_metadata(directory)?;
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err("Workspace directories must be real directories.".into());
        }
    }
    match route {
        "/api/status" => {
            let mut names = Vec::new();
            for entry in fs::read_dir(&sources)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        if child(&sources, name).is_ok() {
                            names.push(name.to_owned());
                        }
                    }
                }
            }
            names.sort();
            let snapshots: Vec<String> = list(&vault)?
                .iter()
                .filter_map(|p| p.file_name()?.to_str().map(String::from))
                .collect();
            Ok(
                json!({"sources": names, "snapshots": snapshots, "workspace": root.display().to_string()}),
            )
        }
        "/api/demo" => {
            let path = child(&sources, "starfall-demo")?;
            fs::create_dir(&path)?;
            fs::write(
                path.join("slot1.sav"),
                b"fictional_game=Starfall\nlevel=3\ncoins=42\n",
            )?;
            Ok(json!({"message": "Created fictional Starfall save data."}))
        }
        "/api/snapshot" => {
            let source = child(&sources, text(&body, "source")?)?;
            let path = snapshot(&source, &vault)?;
            Ok(
                json!({"message": "Snapshot complete.", "snapshot": path.file_name().and_then(|s| s.to_str())}),
            )
        }
        "/api/verify" => {
            let path = child(&vault, text(&body, "snapshot")?)?;
            Ok(json!({"manifest": verify(&path)?, "message": "All stored files verified."}))
        }
        "/api/restore" => {
            let path = child(&vault, text(&body, "snapshot")?)?;
            let output = child(&restores, text(&body, "destination")?)?;
            let restored = restore(&path, &output)?;
            Ok(json!({"message": "Restore complete.", "path": restored.display().to_string()}))
        }
        _ => Err("Unknown operation.".into()),
    }
}

pub fn serve(root: &Path) -> Result<()> {
    fs::create_dir_all(root)?;
    let root = root.canonicalize()?;
    for name in ["sources", "vault", "restores"] {
        let path = root.join(name);
        fs::create_dir_all(&path)?;
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err("Workspace directories cannot be symlinks.".into());
        }
    }
    let server = Server::http("127.0.0.1:7413").map_err(|e| e.to_string())?;
    println!("SaveShelf workspace: http://127.0.0.1:7413");
    println!("Local files: {}", root.display());
    for mut request in server.incoming_requests() {
        let header = |name: &str| {
            request
                .headers()
                .iter()
                .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
                .map(|h| h.value.as_str().to_owned())
        };
        let allowed = header("Host").as_deref() == Some("127.0.0.1:7413");
        let origin = header("Origin").as_deref() == Some("http://127.0.0.1:7413");
        let custom = header("X-Requested-With").as_deref() == Some("SaveShelf");
        let route = request.url().to_owned();
        let (status, kind, body) = if !allowed {
            (
                403,
                "application/json",
                json!({"error": "Use the printed loopback URL."}).to_string(),
            )
        } else if request.method() == &Method::Get
            && ["/", "/app.js", "/style.css"].contains(&route.as_str())
        {
            match route.as_str() {
                "/" => (
                    200,
                    "text/html",
                    include_str!("../web/index.html").to_owned(),
                ),
                "/app.js" => (
                    200,
                    "text/javascript",
                    include_str!("../web/app.js").to_owned(),
                ),
                _ => (200, "text/css", include_str!("../web/style.css").to_owned()),
            }
        } else if request.method() == &Method::Post && origin && custom {
            let result = (|| -> Result<Value> {
                let mut bytes = Vec::new();
                request.as_reader().take(16385).read_to_end(&mut bytes)?;
                if bytes.len() > 16384 {
                    return Err("Request too large.".into());
                }
                operation(&root, &route, serde_json::from_slice(&bytes)?)
            })();
            match result {
                Ok(value) => (200, "application/json", value.to_string()),
                Err(error) => (
                    400,
                    "application/json",
                    json!({"error": error.to_string()}).to_string(),
                ),
            }
        } else {
            (
                403,
                "application/json",
                json!({"error": "Request must come from the local workspace."}).to_string(),
            )
        };
        let mut response = Response::from_string(body).with_status_code(status);
        for (name, value) in [
            ("Content-Type", format!("{kind}; charset=utf-8")),
            ("Cache-Control", "no-store".to_owned()),
            ("X-Content-Type-Options", "nosniff".to_owned()),
            ("Content-Security-Policy", "default-src 'self'; style-src 'self'; script-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'".to_owned()),
        ] {
            response.add_header(Header::from_bytes(name, value).map_err(|_| "Invalid response header.")?);
        }
        // A disconnected browser must not terminate the server or undo a completed operation.
        let _ = request.respond(response);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_names_cannot_escape() {
        for bad in ["", "../escape", "/tmp", "a/b", "a\\b", ".", "..", "a:b"] {
            assert!(child(Path::new("/tmp"), bad).is_err());
        }
        assert!(child(Path::new("/tmp"), "slot-1_backup").is_ok());
    }
}
