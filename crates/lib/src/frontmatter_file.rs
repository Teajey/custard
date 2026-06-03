pub mod keeper;

use anyhow::{anyhow, Context as _};
use camino::{Utf8Path as Path, Utf8PathBuf};
use chrono::{DateTime, Utc};
use serde::Serialize;

pub use keeper::Keeper;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FrontmatterFile {
    pub name: String,
    pub directory: Utf8PathBuf,
    pub frontmatter: Option<serde_yaml::Mapping>,
    pub body: String,
    pub modified: DateTime<Utc>,
    pub created: DateTime<Utc>,
}

impl Ord for FrontmatterFile {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.created.cmp(&other.created)
    }
}

impl PartialOrd for FrontmatterFile {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Short {
    pub name: String,
    pub directory: Utf8PathBuf,
    pub frontmatter: Option<serde_yaml::Mapping>,
    one_liner: Option<String>,
    pub modified: DateTime<Utc>,
    pub created: DateTime<Utc>,
}

impl Ord for Short {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.created.cmp(&other.created)
    }
}

impl PartialOrd for Short {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl From<FrontmatterFile> for Short {
    fn from(
        FrontmatterFile {
            name,
            directory,
            frontmatter,
            body,
            modified,
            created,
        }: FrontmatterFile,
    ) -> Self {
        let lines = body.lines().collect::<Vec<_>>();
        let one_liner = if lines.len() == 1 {
            Some(lines[0].to_owned())
        } else {
            None
        };
        Self {
            name,
            directory,
            frontmatter,
            one_liner,
            modified,
            created,
        }
    }
}

impl FrontmatterFile {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn frontmatter(&self) -> Option<&serde_yaml::Mapping> {
        self.frontmatter.as_ref()
    }

    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    #[must_use]
    pub fn created(&self) -> &DateTime<Utc> {
        &self.created
    }

    #[must_use]
    pub fn modified(&self) -> &DateTime<Utc> {
        &self.modified
    }

    pub fn read_from_path(path: &Path) -> anyhow::Result<Self> {
        let cwd = std::env::current_dir().context("getting cwd")?;
        let rel_path = path.strip_prefix(cwd).context("stripping cwd from path")?;
        let name = rel_path.to_string();
        let path = path.to_owned();
        let Some(parent) = rel_path.parent().map(ToOwned::to_owned) else {
            return Err(anyhow!("missing parent directory"));
        };
        let metadata = std::fs::metadata(&path).context("loading metadata")?;
        let modified = metadata.modified().context("reading modified date")?.into();
        let created = metadata.created().context("reading created date")?.into();
        let string = std::fs::read_to_string(&path).context("reading file content")?;

        if !string.starts_with("---\n") {
            let md = FrontmatterFile {
                name,
                directory: parent,
                frontmatter: None,
                body: string,
                modified,
                created,
            };
            return Ok(md);
        }

        let [_, frontmatter, body] = string.splitn(3, "---\n").collect::<Vec<_>>()[..] else {
            let md = FrontmatterFile {
                name,
                directory: parent,
                frontmatter: None,
                body: string,
                modified,
                created,
            };
            return Ok(md);
        };

        let frontmatter = serde_yaml::from_str(frontmatter).context("reading frontmatter")?;

        Ok(FrontmatterFile {
            name,
            directory: parent,
            frontmatter: Some(frontmatter),
            body: body.to_owned(),
            modified,
            created,
        })
    }
}
