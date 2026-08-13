use std::fs;
use std::path::Path;

use crate::process::{self, Process};

const REGISTRY: &str = "git.perish.top/perishlab";

struct Image {
    face: &'static str,
    file: &'static str,
    cargo: bool,
}

pub fn run() -> Result<(), String> {
    let root = process::root()?;
    let git = Process::new("git");
    let branch = git.output(["branch", "--show-current"], Some(&root))?;
    if branch != "main" {
        return Err(format!("ship: publish from main, not {branch}"));
    }
    let dirty = git.output(["status", "--short"], Some(&root))?;
    if !dirty.is_empty() {
        return Err("ship: working tree must be clean".to_owned());
    }
    let version = version(&root)?;
    println!("==> ship v{version}");
    Image {
        face: "api",
        file: "deploy/api.Dockerfile",
        cargo: true,
    }
    .forge(&root, &version)?;
    chart(&root, &version)?;
    println!("ship: clean");
    Ok(())
}

impl Image {
    fn forge(&self, root: &Path, version: &str) -> Result<(), String> {
        let docker = Process::new("docker");
        let image = format!("{REGISTRY}/codehull-{}:{version}", self.face);
        if docker.quiet(["manifest", "inspect", &image], Some(root))? {
            println!("==> codehull-{} v{version} already pushed", self.face);
            return Ok(());
        }
        println!("==> build codehull-{}", self.face);
        let mut args = vec![
            "build".to_owned(),
            "--network=host".to_owned(),
            "-f".to_owned(),
            self.file.to_owned(),
            "-t".to_owned(),
            image.clone(),
        ];
        if self.cargo {
            let home = plumb::config::home().ok_or_else(|| "ship: home is required".to_owned())?;
            args.extend([
                "--secret".to_owned(),
                format!(
                    "id=cargo,src={}",
                    home.join(".cargo/credentials.toml").display()
                ),
            ]);
        }
        args.push(".".to_owned());
        docker.run(args, Some(root))?;
        println!("==> push codehull-{}", self.face);
        docker.run(["push", &image], Some(root))
    }
}

fn chart(root: &Path, version: &str) -> Result<(), String> {
    let image = format!("oci://{REGISTRY}/charts/codehull");
    let helm = Process::new("helm");
    if helm.quiet(["show", "chart", &image, "--version", version], Some(root))? {
        println!("==> codehull chart v{version} already pushed");
        return Ok(());
    }
    println!("==> package chart");
    let target = root.join(".local");
    helm.run(
        [
            "package".into(),
            "charts/codehull".into(),
            "--version".into(),
            version.into(),
            "--app-version".into(),
            version.into(),
            "--destination".into(),
            target.into_os_string(),
        ],
        Some(root),
    )?;
    println!("==> push chart");
    let package = root.join(".local").join(format!("codehull-{version}.tgz"));
    helm.run(
        [
            "push".into(),
            package.into_os_string(),
            format!("oci://{REGISTRY}/charts").into(),
        ],
        Some(root),
    )
}

fn version(root: &Path) -> Result<String, String> {
    let cargo = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|error| format!("ship: Cargo.toml: {error}"))?;
    let chart = fs::read_to_string(root.join("charts/codehull/Chart.yaml"))
        .map_err(|error| format!("ship: Chart.yaml: {error}"))?;
    let workspace = quoted(&cargo, "version = ")
        .ok_or_else(|| "ship: no workspace version in Cargo.toml".to_owned())?;
    let package = plain(&chart, "version: ");
    let app = quoted(&chart, "appVersion: ").or_else(|| plain(&chart, "appVersion: "));
    if package.as_deref() != Some(&workspace) || app.as_deref() != Some(&workspace) {
        return Err("ship: Cargo and chart versions must match".to_owned());
    }
    Ok(workspace)
}

fn quoted(text: &str, prefix: &str) -> Option<String> {
    text.lines().find_map(|line| {
        line.trim()
            .strip_prefix(prefix)
            .and_then(|value| value.strip_prefix('"'))
            .and_then(|value| value.strip_suffix('"'))
            .map(str::to_owned)
    })
}

fn plain(text: &str, prefix: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix(prefix).map(str::to_owned))
}
