use anyhow::Context;
use log::info;
use std::path::{Path, PathBuf};

struct SkillAsset {
    name: &'static str,
    body: &'static str, // Entire SKILL.md contents.
}

const REVIEW_HELPER_NAME: &str = "illuc-review.py";
const REVIEW_HELPER_BODY: &str = include_str!("assets/illuc-review.py");

// Embed repo skill files so this also works in packaged builds.
const SKILLS: &[SkillAsset] = &[
    SkillAsset {
        name: "illuc-review",
        body: include_str!("assets/illuc-review/SKILL.md"),
    },
    SkillAsset {
        name: "illuc-fix-review",
        body: include_str!("assets/illuc-fix-review/SKILL.md"),
    },
];

fn resolve_home_dir() -> anyhow::Result<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .context("failed to resolve home directory")
}

fn write_skill(root: &Path, skill: &SkillAsset) -> anyhow::Result<()> {
    let dir = root.join(skill.name);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create skills dir {}", dir.display()))?;
    let skill_path = dir.join("SKILL.md");
    let helper_path = dir.join(REVIEW_HELPER_NAME);

    // Always overwrite with the latest version on startup.
    std::fs::write(&skill_path, skill.body.as_bytes())
        .with_context(|| format!("failed to write {}", skill_path.display()))?;
    std::fs::write(&helper_path, REVIEW_HELPER_BODY.as_bytes())
        .with_context(|| format!("failed to write {}", helper_path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&helper_path)
            .with_context(|| format!("failed to stat {}", helper_path.display()))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&helper_path, perms)
            .with_context(|| format!("failed to chmod {}", helper_path.display()))?;
    }

    Ok(())
}

fn install_to(root: &Path) -> anyhow::Result<()> {
    for skill in SKILLS {
        write_skill(root, skill)?;
    }
    Ok(())
}

/// Install illuc's built-in skills into external tool locations.
///
/// Target dirs:
/// - `~/.agents/skills/<skill>/SKILL.md` (Codex/Agents)
/// - `~/.copilot/skills/<skill>/SKILL.md` (Copilot CLI)
pub fn install_predefined_skills_on_startup() -> anyhow::Result<()> {
    let home = resolve_home_dir()?;

    let agents_dir = home.join(".agents").join("skills");
    let copilot_dir = home.join(".copilot").join("skills");

    install_to(&agents_dir)
        .with_context(|| format!("failed installing skills into {}", agents_dir.display()))?;
    install_to(&copilot_dir)
        .with_context(|| format!("failed installing skills into {}", copilot_dir.display()))?;

    info!(
        "installed predefined skills into {} and {}",
        agents_dir.display(),
        copilot_dir.display()
    );
    Ok(())
}
