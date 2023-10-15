pub mod map;
use std::path::Path;

pub use map::Map;

use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
pub struct Named {
    name: String,
    frontmatter: Option<serde_yaml::Mapping>,
    body: String,
}

impl Named {
    pub fn from_map_entry((path, file): (&Path, &FrontmatterFile)) -> Option<Self> {
        let name = path
            .file_name()
            .expect("This path must have a valid file name")
            .to_str()?
            .to_owned();
        Some(Named {
            name,
            frontmatter: file.frontmatter.clone(),
            body: file.body.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FrontmatterFile {
    pub frontmatter: Option<serde_yaml::Mapping>,
    pub body: String,
}

impl FrontmatterFile {
    pub fn from_string(string: String) -> Result<Self, serde_yaml::Error> {
        if !string.starts_with("---\n") {
            let md = FrontmatterFile {
                frontmatter: None,
                body: string,
            };
            return Ok(md);
        }

        let [_, frontmatter, body] = string.splitn(3, "---\n").collect::<Vec<_>>()[..] else {
            let md = FrontmatterFile {
                frontmatter: None,
                body: string,
            };
            return Ok(md);
        };

        let frontmatter = serde_yaml::from_str(frontmatter)?;

        Ok(FrontmatterFile {
            frontmatter: Some(frontmatter),
            body: body.to_owned(),
        })
    }
}
