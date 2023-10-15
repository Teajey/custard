pub mod map;
pub use map::Map;

use serde::Serialize;

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
